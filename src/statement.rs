//!
//! Rust Firebird Client
//!
//! Preparation and execution of statements
//!

use crate::{
    transaction::{Transaction, TransactionData},
    Connection
};
use rsfbclient_core::{
    Column, FbError, FirebirdClient, FreeStmtOp, FromRow, StmtType, IntoStmtArgs
    , DynParam, DynParams
};
use std::marker::PhantomData;

pub(crate) enum StmtDropBehavior {
  DropStmt,
  CacheStmt
}

trait IntoParams : IntoStmtArgs<Idx=DynParam>{}
impl<T:IntoStmtArgs<Idx=DynParam>> IntoParams for T{}

/// Low level statement handler.
///
/// Needs to be closed calling `close` before dropping.
pub struct StatementData<C: FirebirdClient> {
    pub(crate) sql: String,
    pub(crate) handle: C::StmtHandle,
    pub(crate) stmt_type: StmtType,
}
 
pub struct Statement<'c, 't, C: FirebirdClient> {
    pub(crate) data: StatementData<C>,
    drop_behavior: StmtDropBehavior,
    pub(crate) tr: &'t mut Transaction<'c, C>,
}

pub struct StmtIter<'c, 't, R, C>
  where
  C:FirebirdClient,
  R: FromRow {
    base_iter: Statement<'c,'t, C>,
    //maybe can be replaced by R: TryFrom<Vec<Column>>
    _marker: PhantomData<R>,
}

/// Cursor to fetch the results of a statement
pub struct StatementFetch<'c, 's, R, C: FirebirdClient> {
    pub(crate) stmt: &'s mut StatementData<C>,
    /// Transaction needs to be alive for the fetch to work
    pub(crate) tr: &'s mut Transaction<'c, C>,
    /// Type to convert the rows
    _marker: std::marker::PhantomData<R>,
}

impl<C:FirebirdClient> Drop for Statement<'_, '_, C>
{
    fn drop(&mut self) {
      use StmtDropBehavior::*;

      match self.drop_behavior {
        DropStmt => {
          self.tr.conn.client().free_statement(
            &mut self.data.handle,
            FreeStmtOp::Drop
          );
        }
        CacheStmt => {
          self.tr.conn.client().free_statement(
            &mut self.data.handle,
            FreeStmtOp::Close
          );
          self.tr.conn.stmt_cache.insert(self.data);
        }
      }
    }
}

impl<C> Iterator for Statement<'_, '_, C>
where
    C: FirebirdClient,
{
    type Item = Result<Vec<Column>, FbError>;

    fn next(&mut self) -> Option<Self::Item> {
        self
          .tr
          .conn
          .client()
          .fetch(
            &mut self.tr.conn.handle,
            &mut self.tr.data.handle,
            &mut self.data.handle
          )
          .transpose()
    }
}


impl<R, C> Iterator for StmtIter<'_, '_, R, C>
where
    R: FromRow,
    C: FirebirdClient,
{
    type Item = Result<R, FbError>;

    fn next(&mut self) -> Option<Self::Item> {
        self
          .base_iter
          .next()
          .map(|maybe_row_untyped| maybe_row_untyped.and_then(FromRow::try_from) )

    }
}



impl<C:FirebirdClient> AsRef<str> for StatementData<C> {
  fn as_ref(&self) -> &str {
    self.sql.as_ref()
  }
}




impl<'c, 't, C> Statement<'c, 't, C>
where
    C: FirebirdClient,
{
      
    /// Prepare the statement that will be executed
    pub fn prepare<Sql:AsRef<str>>(
        tr: &'t mut Transaction<'c, C>,
        sql: Sql,
    ) -> Result<Self, FbError> {

        let raw_sql = sql.as_ref();
        let data = StatementData::prepare(
          tr.conn,
          &mut tr.data,
          raw_sql
        )?;

        Ok(Statement {
          data,
          drop_behavior: StmtDropBehavior::DropStmt,
          tr
        })
    }

    /// Execute the current statement without returnig any row
    ///
    /// Use `()` for no parameters or a tuple of parameters
    pub fn execute<P>(&mut self, params: P) -> Result<(), FbError>
      where P: IntoStmtArgs<Idx=DynParam>
    {
        self.data.execute(
          self.tr.conn,
          &mut self.tr.data,
          DynParams::params(params)
        )
    }

    /// Execute the current statement
    /// and returns the lines founds
    ///
    /// Use `()` for no parameters or a tuple of parameters
    pub fn query<'s, R, P>(&'s mut self, params: P) -> Result<StatementFetch<'c, 's, R, C>, FbError>
    where
        R: FromRow,
        P: IntoStmtArgs<Idx=DynParam>,
    {
            self.data.query(
              self.tr.conn,
              &mut self.tr.data,
              DynParams::params(params)
            )?;

            Ok(StatementFetch {
                stmt: &mut self.data,
                tr: self.tr,
                _marker: Default::default(),
            })
          
    }
}

impl<C: FirebirdClient> StatementData<C>
{
    /// Prepare the statement that will be executed
    pub fn prepare<Sql:AsRef<str>>(
        conn: &mut Connection<C>,
        tr: &mut TransactionData<C>,
        sql: Sql,
    ) -> Result<Self, FbError> {

        let raw_sql = sql.as_ref();
        let (stmt_type, handle) =
            conn.client()
                .prepare_statement(
                  &mut conn.handle,
                  &mut tr.handle,
                  conn.dialect,
                  raw_sql,
                )?;

        Ok(Self {
            sql: raw_sql.to_string(),
            stmt_type,
            handle,
        })
    }

    /// Execute the current statement without returnig any row
    ///
    /// Use `()` for no parameters or a tuple of parameters
    pub fn execute<T>(
        &mut self,
        conn: &mut Connection<C>,
        tr: &mut TransactionData<C>,
        params: T,
    ) -> Result<(), FbError>
    where
        T: IntoStmtArgs<Idx=DynParam>,
    {

        conn.client().execute(
            &mut conn.handle,
            &mut tr.handle,
            &mut self.handle,
            DynParams::params(params)
        )?;

        if self.stmt_type == StmtType::Select {
            // Close the cursor, as it will not be used
            self.close_cursor(conn)?;
        }

        Ok(())
    }

    /// Execute the current statement with input and returns a single row
    ///
    /// Use `()` for no parameters or a tuple of parameters
    pub fn execute2<T>(
        &mut self,
        conn: &mut Connection<C>,
        tr: &mut TransactionData<C>,
        params: T,
    ) -> Result<Vec<Column>, FbError>
    where
        T: IntoStmtArgs<Idx=DynParam>,
    {
        conn.client().execute2(
            &mut conn.handle,
            &mut tr.handle,
            &mut self.handle,
            DynParams::params(params)
        )
    }

    /// Execute the current statement
    /// and returns the column buffer
    ///
    /// Use `()` for no parameters or a tuple of parameters
    pub fn query<'s, T>(
        &'s mut self,
        conn: &'s mut Connection<C>,
        tr: &mut TransactionData<C>,
        params: T,
    ) -> Result<(), FbError>
    where
        T: IntoStmtArgs<Idx=DynParam>,
    {
        conn.client().execute(
            &mut conn.handle,
            &mut tr.handle,
            &mut self.handle,
            DynParams::params(params)
        )
    }

    /// Fetch for the next row, needs to be called after `query`
    pub fn fetch(
        &mut self,
        conn: &mut Connection<C>,
        tr: &mut TransactionData<C>,
    ) -> Result<Option<Vec<Column>>, FbError> {
        conn.client()
            .fetch(
              &mut conn.handle,
              &mut tr.handle,
              &mut self.handle
            )
    }

    /// Closes the statement cursor, if it was open
    pub fn close_cursor(&mut self, conn: &mut Connection<C>) -> Result<(), FbError> {
        conn.client().free_statement(
          &mut self.handle,
          FreeStmtOp::Close
        )
    }

    /// Closes the statement
    pub fn close(&mut self, conn: &mut Connection<C>) -> Result<(), FbError> {
        conn.client().free_statement(
          &mut self.handle,
          FreeStmtOp::Drop
        )
    }
}

#[cfg(test)]
/// Counter to allow the tests to be run in parallel without interfering in each other
static TABLE_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

#[cfg(test)]
mk_tests_default! {
    use crate::{prelude::*, Connection, Row};
    use rsfbclient_core::FirebirdClient;

    #[test]
    fn new_api_select() {
        let (mut conn, table) = setup();

        let vals = vec![
            (Some(2), "coffee".to_string()),
            (Some(3), "milk".to_string()),
            (None, "fail coffee".to_string()),
        ];

        conn.with_transaction(|tr| {
            for val in &vals {
                tr.execute(&format!("insert into {} (id, name) values (?, ?)", table), val.clone())
                    .expect("Error on insert");
            }

            Ok(())
        })
        .expect("Error commiting the transaction");

        let rows = conn
            .query(&format!("select id, name from {}", table), ())
            .expect("Error executing query");

        // Asserts that all values are equal
        assert_eq!(vals, rows);
    }

    #[test]
    fn old_api_select() {
        let (mut conn, table) = setup();

        let vals = vec![
            (Some(2), "coffee".to_string()),
            (Some(3), "milk".to_string()),
            (None, "fail coffee".to_string()),
        ];

        conn.with_transaction(|tr| {
            let mut stmt = tr
                .prepare(
                  &format!("insert into {} (id, name) values (?, ?)", table),
                  false
                )
                .expect("Error preparing the insert statement");

            for val in &vals {
                stmt.execute(val.clone())
                  .expect("Error on insert");
            }

            Ok(())
        })
        .expect("Error commiting the transaction");

        conn.with_transaction(|tr| {
            let mut stmt = tr
                .prepare(&format!("select id, name from {}", table), false)
                .expect("Error on prepare the select");

            let rows: Vec<(Option<i32>, String)> = stmt
                .query(())
                .expect("Error on query")
                .collect::<Result<_, _>>()
                .expect("Error on fetch");

            // Asserts that all values are equal
            assert_eq!(vals, rows);

            let mut rows = stmt.query(()).expect("Error on query");

            let row1: Row = rows
                .fetch()
                .expect("Error on fetch the next row")
                .expect("No more rows");

            assert_eq!(
                2,
                row1.get::<i32>(0)
                    .expect("Error on get the first column value")
            );
            assert_eq!(
                "coffee".to_string(),
                row1.get::<String>(1)
                    .expect("Error on get the second column value")
            );

            let row = rows
                .fetch()
                .expect("Error on fetch the next row")
                .expect("No more rows");

            assert_eq!(
                3,
                row.get::<i32>(0)
                    .expect("Error on get the first column value")
            );
            assert_eq!(
                "milk".to_string(),
                row.get::<String>(1)
                    .expect("Error on get the second column value")
            );

            let row = rows
                .fetch()
                .expect("Error on fetch the next row")
                .expect("No more rows");

            assert!(
                row.get::<i32>(0).is_err(),
                "The 3° row have a null value, then should return an error"
            ); // null value
            assert!(
                row.get::<Option<i32>>(0)
                    .expect("Error on get the first column value")
                    .is_none(),
                "The 3° row have a null value, then should return a None"
            ); // null value
            assert_eq!(
                "fail coffee".to_string(),
                row.get::<String>(1)
                    .expect("Error on get the second column value")
            );

            let row = rows.fetch().expect("Error on fetch the next row");

            assert!(
                row.is_none(),
                "The 4° row dont exists, then should return a None"
            ); // null value

            Ok(())
        })
        .expect("Error commiting the transaction");

        conn.close().expect("error on close the connection");
    }

    #[test]
    fn prepared_insert() {
        let (mut conn, table) = setup();

        let vals = vec![(Some(9), "apple"), (Some(12), "jack"), (None, "coffee")];

        conn.with_transaction(|tr| {
            for val in vals.into_iter() {
                tr.execute(&format!("insert into {} (id, name) values (?, ?)", table), val)
                    .expect("Error on insert");
            }

            Ok(())
        })
        .expect("Error in the transaction");

        conn.close().expect("error on close the connection");
    }

    // #[test]
    // fn immediate_insert() {
    //     let (mut conn, table) = setup();

    //     conn.with_transaction(|tr| {
    //         tr.execute_immediate(&format!("insert into {} (id, name) values (?, ?)", (1, "apple", table)))
    //             .expect("Error on 1° insert");

    //         tr.execute_immediate(&format!("insert into {} (id, name) values (?, ?)", (2, "coffe", table)))
    //             .expect("Error on 2° insert");

    //         Ok(())
    //     })
    //     .expect("Error in the transaction");

    //     conn.close().expect("error on close the connection");
    // }

    fn setup() -> (Connection<impl FirebirdClient>, String) {
        let mut conn = cbuilder().connect()
            .expect("Error on connect in the test database");

        let table_num = super::TABLE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let table = format!("product{}", table_num);

        conn.with_transaction(|tr| {
            tr.execute_immediate(&format!("DROP TABLE {}", table)).ok();

            tr.execute_immediate(&format!("CREATE TABLE {} (id int, name varchar(60), quantity int)", table))
                .expect("Error on create the table product");

            Ok(())
        })
        .expect("Error in the transaction");

        (conn, table)
    }
}
