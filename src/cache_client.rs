//!
//! Rust Firebird Client
//!
//! Statement Cache
//!

use lru_cache::LruCache;
use rsfbclient_core::{
  FirebirdClient,
  FirebirdClientSqlOps,
  FirebirdClientDbOps,
  Dialect,
  SqlType,
  StmtType,
  TrIsolationLevel,
  TrOp,
  FreeStmtOp,
  Column,
  FbError};


#[derive(Clone)]
pub struct StatementData<C: FirebirdClient> {
    pub(crate) stmt_type: StmtType,
    pub(crate) sql: String,
    pub(crate) raw_handle: C::StmtHandle,
}

/// Cache of prepared statements.
///
/// Must be emptied by calling `close_all` before dropping.
pub struct StmtCache<C: FirebirdClient> {
    cache: LruCache<String, StatementData<C>>,
    cli: C,
}

/// General functions
impl<C: FirebirdClient> StmtCache<C> {
    pub fn new(cli: C, capacity: usize) -> Self {
        Self {
            cache: LruCache::new(capacity),
            cli: cli,
        }
    }

    /// Get a prepared statement from the cache
    fn get(&mut self, sql: &str) -> Option<StatementData<C>> {
        self.cache.remove(&sql.to_string())
    }

    /// Adds a prepared statement to the cache, returning the previous one for this sql
    /// or another if the cache is full
    fn insert(&mut self, data: StatementData<C>) -> Option<StatementData<C>> {
        self.cache.insert(data.sql.clone(), data)
    }
}


impl<C:FirebirdClient> FirebirdClientSqlOps  for StmtCache<C>
  where C: FirebirdClient {
    type DbHandle = <C as FirebirdClientSqlOps>::DbHandle;
    type TrHandle = C::TrHandle;
    type StmtHandle = StatementData<C>;

    fn begin_transaction(
        &mut self,
        db_handle: &mut Self::DbHandle,
        isolation_level: TrIsolationLevel,
    ) -> Result<Self::TrHandle, FbError> {
      self.cli.begin_transaction(db_handle,isolation_level)
    }

    fn transaction_operation(
        &mut self,
        tr_handle: &mut Self::TrHandle,
        op: TrOp,
    ) -> Result<(), FbError> {
      self.cli.transaction_operation(tr_handle, op)
    }

    fn exec_immediate(
        &mut self,
        db_handle: &mut Self::DbHandle,
        tr_handle: &mut Self::TrHandle,
        dialect: Dialect,
        sql: &str,
    ) -> Result<(), FbError> {
      self.cli.exec_immediate(db_handle, tr_handle, dialect, sql)
    }

    fn prepare_statement(
        &mut self,
        db_handle: &mut Self::DbHandle,
        tr_handle: &mut Self::TrHandle,
        dialect: Dialect,
        sql: &str,
    ) -> Result<(StmtType, Self::StmtHandle), FbError> {

        if let Some(data) = self.get(sql) {

          Ok((data.stmt_type,data))

        } else {

          let (stmt_type, stmt_hdl) = self.cli.prepare_statement(db_handle,tr_handle, dialect, sql)?;

          let data = StatementData {
              stmt_type,
              sql: sql.to_string(),
              raw_handle: stmt_hdl,
          };

          Ok((stmt_type, data))
        
        }
    }

    //fn statement_info(
    //  &mut self,
    //  stmt_hdl: &mut Self::StmtHandle,
    //) -> Result<Self::StmtInfo, FbError> {
    //  self.cli.statement_info(stmt_hdl.raw_handle)
    //}


    fn free_statement(
        &mut self,
        stmt_handle: &mut Self::StmtHandle,
        op: FreeStmtOp,
    ) -> Result<(), FbError> {

      //Do nothing.
      //TODO: decide what to do about this?
      Ok(())

    }

    fn close_statement(
      &mut self,
      stmt_handle: &mut Self::StmtHandle,
    ) -> Result<(), FbError>{


      self.cli.close_statement( &mut stmt_handle.raw_handle)
    
    }

    fn drop_statement(
      &mut self,
      mut stmt_handle: Self::StmtHandle
    ) -> Result<(), FbError> {

      let result = self.cli.close_statement( &mut stmt_handle.raw_handle )?;

      let old_maybe = self.insert(stmt_handle);

      //free the statement that fell off the end of the cache 
      //if this fails we'll leave it up to the server to handle
      if let Some(old) = old_maybe {
        self.cli.drop_statement( old.raw_handle );
      }

      Ok(result)

    }

    fn execute(
        &mut self,
        db_handle: &mut Self::DbHandle,
        tr_handle: &mut Self::TrHandle,
        stmt_handle: &mut Self::StmtHandle,
        params: Vec<SqlType>,
    ) -> Result<(), FbError> {

      self.cli.execute(
        db_handle,
        tr_handle,
        &mut stmt_handle.raw_handle,
        params
      )
    }

    fn execute2(
        &mut self,
        db_handle: &mut Self::DbHandle,
        tr_handle: &mut Self::TrHandle,
        stmt_handle: &mut Self::StmtHandle,
        params: Vec<SqlType>,
    ) -> Result<Vec<Column>, FbError> {

      self.cli.execute2(
        db_handle,
        tr_handle,
        &mut stmt_handle.raw_handle,
        params
        )
    }

    fn fetch(
        &mut self,
        db_handle: &mut Self::DbHandle,
        tr_handle: &mut Self::TrHandle,
        stmt_handle: &mut Self::StmtHandle,
    ) -> Result<Option<Vec<Column>>, FbError> {

      self.cli.fetch(
        db_handle,
        tr_handle,
        &mut stmt_handle.raw_handle
      )
    }
}


impl<C:FirebirdClient> FirebirdClientDbOps for StmtCache<C> {
  type DbHandle = <C as FirebirdClientDbOps>::DbHandle;
  type AttachmentConfig = C::AttachmentConfig;

    /// Create a new attachment to a database with the provided configuration
    /// Returns a database handle on success
    fn attach_database(
        &mut self,
        config: &Self::AttachmentConfig,
    ) -> Result<Self::DbHandle, FbError> {
      self.cli.attach_database(config)
    }

    /// Disconnect from the database
    fn detach_database(&mut self, db_handle: &mut Self::DbHandle) -> Result<(), FbError> {
      self.cli.detach_database(db_handle)
    }

    /// Drop the database
    fn drop_database(&mut self, db_handle: &mut Self::DbHandle) -> Result<(), FbError> {
      self.cli.drop_database(db_handle)
    }
}


//#[test]
//fn stmt_cache_test() {
//    let mut cache = StmtCache::new(2);
//
//    let mk_test_data = |n: usize| StmtCacheData {
//        sql: format!("sql {}", n),
//        stmt: n,
//    };
//
//    let sql1 = mk_test_data(1);
//    let sql2 = mk_test_data(2);
//    let sql3 = mk_test_data(3);
//    let sql4 = mk_test_data(4);
//    let sql5 = mk_test_data(5);
//    let sql6 = mk_test_data(6);
//
//    assert!(cache.get(&sql1.sql).is_none());
//
//    assert!(cache.insert(sql1).is_none());
//
//    assert!(cache.insert(sql2).is_none());
//
//    let stmt = cache.insert(sql3).expect("sql1 not returned");
//    assert_eq!(stmt, 1);
//
//    assert!(cache.get("sql 1").is_none());
//
//    // Marks sql2 as recently used, so 3 must be removed in the next insert
//    let sql2 = cache.get("sql 2").expect("Sql 2 not in the cache");
//    assert!(cache.insert(sql2).is_none());
//
//    let stmt = cache.insert(sql4).expect("sql3 not returned");
//    assert_eq!(stmt, 3);
//
//    let stmt = cache.insert(sql5).expect("sql2 not returned");
//    assert_eq!(stmt, 2);
//
//    let stmt = cache.insert(sql6).expect("sql4 not returned");
//    assert_eq!(stmt, 4);
//
//    assert_eq!(cache.get("sql 5").expect("sql5 not in the cache").stmt, 5);
//    assert_eq!(cache.get("sql 6").expect("sql6 not in the cache").stmt, 6);
//
//    assert!(cache.cache.is_empty());
//    assert!(cache.sqls.is_empty());
//}
