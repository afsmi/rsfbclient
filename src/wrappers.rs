//TODO: iterator for cursor
//TODO: Reimpl named params/fromrow support
//TODO: impl drop for each type
//TODO: reimpl tests

use rsfbclient_core::{
    Column, Dialect, FbError, FirebirdClient, FirebirdClientDbOps, FreeStmtOp, FromRow, Row,
    SqlType, TrIsolationLevel, TrOp,
};
use std::collections::HashSet;
use std::marker::PhantomData;

use super::raw_handles::*;


pub struct DisconnectedFbClient<C: FirebirdClient>(pub(self) HandleData<C>);
impl<C: FirebirdClient> DisconnectedFbClient<C> {

    /// Create a new disconnected client from a raw client instance
    pub fn new(client: C) -> Self {
      DisconnectedFbClient(HandleData::new(client))
    }

    /// Try to connect using the given attachment configuration,
    /// converting the disconnected client into a connected one.
    ///
    /// On failure, return a disconnected client that can be used to try again,
    /// together with an error describing what failed during the connection attempt.
    pub fn connect(
        mut self,
        attachment_conf: &C::AttachmentConfig,
    ) -> Result<ConnectedFbClient<C>, (Self, FbError)> {
        let res = self.0.attach_database(&attachment_conf);

        match res {
          Ok(_) => {
            Ok(ConnectedFbClient(self.0))
          }
          Err(e) => {
            Err((self, e))
          }
        }
    }
}

pub struct ConnectedFbClient<C: FirebirdClient>(HandleData<C>);
impl<'a, C: FirebirdClient> ConnectedFbClient<C> {
    //TODO: prepare (using temporary transaction)
    //TODO: exec_immed
    //TODO: drop_database

    /// Run a closure with a transaction.
    /// use DropBehavior to select what will happen at the end of the transaction
    /// Note that if you commit or rollback the transaction manually as the final action
    /// on the transaction, then no action will be taken automatically
    pub fn with_transaction<T, F>(
        &mut self,
        on_drop: DropBehavior,
        closure: F,
    ) -> Result<T, FbError>
    where
        F: for<'b> FnOnce(&mut Tran<'b, C>) -> Result<T, FbError>,
    {
        use DropBehavior::*;
        use TrState::*;

        let mut transaction = Tran::try_new(&mut self.0, TrIsolationLevel::ReadCommited)?;

        let res = closure(&mut transaction);

        //TODO: need to not drop here, since open cursors may have produced side effects which need to be committed
        drop(transaction);

        if (on_drop != Ignore) && (self.0.tr_state != Clean) {
            match on_drop {
                Commit => self.0.commit().or_else(|_| self.0.rollback()),
                Rollback => self.0.rollback(),
                _ => Ok(()),
            }?;
        }

        res
    }

    pub fn disconnect(mut self) -> Result<DisconnectedFbClient<C>, (Self, FbError)> {
        let res = self.0.detach_database();

        match res {
          Ok(_) => Ok(DisconnectedFbClient(self.0)),
          Err(e) => Err((self, e))
        }
    }
}


// design considerations for transactions
// within a call to "with_transaction" what should a user be able to do
// - do we want to track the state of a transaction through types?
// - what about setting isolation levels?
pub struct Tran<'a, C: FirebirdClient>(&'a mut HandleData<C>);

impl<'a, C: FirebirdClient> Tran<'a, C> {
    //TODO: fetch_all
    //TODO: prepare
    //TODO: execute
    //TODO: execute2
    //TODO: execute_immed
    //TODO: free stmt
    //TODO: close_stmt -> use swap_remove
    //TODO: fetch
    //TODO: commit_x
    //TODO: rollback_x
    //
    fn try_new(
        handle_data: &'a mut HandleData<C>,
        isolation_level: TrIsolationLevel,
    ) -> Result<Self, FbError> {
        //note: begin_transaction will panic if not connected
        handle_data.begin_transaction(isolation_level)?;

        Ok(Tran(handle_data))
    }

    fn open_cursor<R: FromRow>(&mut self, mut stmt: Stmt<R>) -> Cursor<R> {
        self.0.cursors.insert(stmt.handle);
        Cursor::from_stmt(stmt)
    }

    fn close_cursor<R: FromRow>(&mut self, mut cursor: Cursor<R>) -> Stmt<R> {
        self.0.close_cursor(cursor.0.handle);
        self.0.cursors.remove(&cursor.0.handle);
        cursor.0
    }

    fn execute<R: FromRow>(&mut self, stmt: Stmt<R>) -> Result<Option<R>, FbError> {
        //TODO: actually execute
        Ok(None)
    }

}

impl<'a, C: FirebirdClient> Drop for Tran<'a, C> {
    fn drop(&mut self) {
        self.0.close_all_cursors();
    }
}

pub struct Stmt<R: FromRow> {
    handle: usize,
    _marker: PhantomData<R>,
}

pub struct Cursor<R: FromRow>(Stmt<R>);

impl<R: FromRow> Cursor<R> {
    // a statement gets associated with a transaction while it is open
    // so it should be mutably borrowed or consumed
    // here we choose to consume it
    fn from_stmt(stmt: Stmt<R>) -> Self {
        Cursor(stmt)
    }

    fn into_stmt(mut self) -> Stmt<R> {
        self.0
    }
}

///Action to take when transaction when a transaction is dropped.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DropBehavior {
    ///Commit the transaction
    ///
    ///On failure, attempt to rollback
    Commit,

    ///Rollback
    Rollback,

    ///Do nothing.
    ///
    ///This will cause the transaction handle to be left open,
    ///even though the handle becomes inaccessible.
    ///Use at your own risk.
    Ignore,
}


#[cfg(test)]
mod tests {
  use crate::mock_client;

   


//  mk_tests_default! {
//      use crate::*;
//  
//      #[test]
//      fn remote_connection() -> Result<(), FbError> {
//          let conn = cbuilder().connect()?;
//  
//          conn.close().expect("error closing the connection");
//  
//          Ok(())
//      }
//  
//      #[test]
//      fn query_iter() -> Result<(), FbError> {
//          let mut conn = cbuilder().connect()?;
//  
//          let mut rows = 0;
//  
//          for row in conn
//              .query_iter("SELECT -3 FROM RDB$DATABASE WHERE 1 = ?", (1,))?
//          {
//              let (v,): (i32,) = row?;
//  
//              assert_eq!(v, -3);
//  
//              rows += 1;
//          }
//  
//          assert_eq!(rows, 1);
//  
//          Ok(())
//      }
//  }
//  
//  #[cfg(test)]
//  /// Counter to allow the tests to be run in parallel without interfering in each other
//  static TABLE_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
//  
//  #[cfg(test)]
//  mk_tests_default! {
//      use crate::{prelude::*, Connection, Row};
//      use rsfbclient_core::FirebirdClient;
//  
//      #[test]
//      fn new_api_select() {
//          let (mut conn, table) = setup();
//  
//          let vals = vec![
//              (Some(2), "coffee".to_string()),
//              (Some(3), "milk".to_string()),
//              (None, "fail coffee".to_string()),
//          ];
//  
//          conn.with_transaction(|tr| {
//              for val in &vals {
//                  tr.execute(&format!("insert into {} (id, name) values (?, ?)", table), val.clone())
//                      .expect("Error on insert");
//              }
//  
//              Ok(())
//          })
//          .expect("Error commiting the transaction");
//  
//          let rows = conn
//              .query(&format!("select id, name from {}", table), ())
//              .expect("Error executing query");
//  
//          // Asserts that all values are equal
//          assert_eq!(vals, rows);
//      }
//  
//      #[test]
//      fn old_api_select() {
//          let (mut conn, table) = setup();
//  
//          let vals = vec![
//              (Some(2), "coffee".to_string()),
//              (Some(3), "milk".to_string()),
//              (None, "fail coffee".to_string()),
//          ];
//  
//          conn.with_transaction(|tr| {
//              let mut stmt = tr
//                  .prepare(&format!("insert into {} (id, name) values (?, ?)", table), false)
//                  .expect("Error preparing the insert statement");
//  
//              for val in &vals {
//                  stmt.execute(val.clone()).expect("Error on insert");
//              }
//  
//              Ok(())
//          })
//          .expect("Error commiting the transaction");
//  
//          conn.with_transaction(|tr| {
//              let mut stmt = tr
//                  .prepare(&format!("select id, name from {}", table), false)
//                  .expect("Error on prepare the select");
//  
//              let rows: Vec<(Option<i32>, String)> = stmt
//                  .query(())
//                  .expect("Error on query")
//                  .collect::<Result<_, _>>()
//                  .expect("Error on fetch");
//  
//              // Asserts that all values are equal
//              assert_eq!(vals, rows);
//  
//              let mut rows = stmt.query(()).expect("Error on query");
//  
//              let row1: Row = rows
//                  .fetch()
//                  .expect("Error on fetch the next row")
//                  .expect("No more rows");
//  
//              assert_eq!(
//                  2,
//                  row1.get::<i32>(0)
//                      .expect("Error on get the first column value")
//              );
//              assert_eq!(
//                  "coffee".to_string(),
//                  row1.get::<String>(1)
//                      .expect("Error on get the second column value")
//              );
//  
//              let row = rows
//                  .fetch()
//                  .expect("Error on fetch the next row")
//                  .expect("No more rows");
//  
//              assert_eq!(
//                  3,
//                  row.get::<i32>(0)
//                      .expect("Error on get the first column value")
//              );
//              assert_eq!(
//                  "milk".to_string(),
//                  row.get::<String>(1)
//                      .expect("Error on get the second column value")
//              );
//  
//              let row = rows
//                  .fetch()
//                  .expect("Error on fetch the next row")
//                  .expect("No more rows");
//  
//              assert!(
//                  row.get::<i32>(0).is_err(),
//                  "The 3° row have a null value, then should return an error"
//              ); // null value
//              assert!(
//                  row.get::<Option<i32>>(0)
//                      .expect("Error on get the first column value")
//                      .is_none(),
//                  "The 3° row have a null value, then should return a None"
//              ); // null value
//              assert_eq!(
//                  "fail coffee".to_string(),
//                  row.get::<String>(1)
//                      .expect("Error on get the second column value")
//              );
//  
//              let row = rows.fetch().expect("Error on fetch the next row");
//  
//              assert!(
//                  row.is_none(),
//                  "The 4° row dont exists, then should return a None"
//              ); // null value
//  
//              Ok(())
//          })
//          .expect("Error commiting the transaction");
//  
//          conn.close().expect("error on close the connection");
//      }
//  
//      #[test]
//      fn prepared_insert() {
//          let (mut conn, table) = setup();
//  
//          let vals = vec![(Some(9), "apple"), (Some(12), "jack"), (None, "coffee")];
//  
//          conn.with_transaction(|tr| {
//              for val in vals.into_iter() {
//                  tr.execute(&format!("insert into {} (id, name) values (?, ?)", table), val)
//                      .expect("Error on insert");
//              }
//  
//              Ok(())
//          })
//          .expect("Error in the transaction");
//  
//          conn.close().expect("error on close the connection");
//      }
//  
//   #[test]
//   fn immediate_insert() {
//       let (mut conn, table) = setup();
//  
//       conn.with_transaction(|tr| {
//           tr.execute_immediate(&format!("insert into {} (id, name) values (?, ?)", (1, "apple", table)))
//               .expect("Error on 1° insert");
//  
//           tr.execute_immediate(&format!("insert into {} (id, name) values (?, ?)", (2, "coffe", table)))
//               .expect("Error on 2° insert");
//  
//           Ok(())
//       })
//       .expect("Error in the transaction");
//  
//       conn.close().expect("error on close the connection");
//   }
//  
//  fn setup() -> (Connection<impl FirebirdClient>, String) {
//      let mut conn = cbuilder().connect()
//          .expect("Error on connect in the test database");
//  
//      let table_num = super::TABLE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
//      let table = format!("product{}", table_num);
//  
//      conn.with_transaction(|tr| {
//          tr.execute_immediate(&format!("DROP TABLE {}", table)).ok();
//  
//          tr.execute_immediate(&format!("CREATE TABLE {} (id int, name varchar(60), quantity int)", table))
//              .expect("Error on create the table product");
//  
//          Ok(())
//      })
//      .expect("Error in the transaction");
//  
//      (conn, table)
//  }
//  }
//


}
