//!
//! Rust Firebird Client
//!
//! Transaction functions
//!

use rsfbclient_core::{FbError, FirebirdClient, TrIsolationLevel, TrOp};
use std::mem::ManuallyDrop;

use super::{connection::Connection, statement::Statement};


#[derive(Debug)]
/// Low level transaction handler.
///
/// Needs to be closed calling `rollback` before dropping.
pub struct TransactionData<C: FirebirdClient> {
    pub(crate) handle: C::TrHandle,
}

pub struct Transaction<'c, C>
where
    C: FirebirdClient,
{
    pub(crate) data: TransactionData<C>,
    pub(crate) conn: &'c mut Connection<C>,
}


impl<'c, C: FirebirdClient> Transaction<'c, C> {
    /// Start a new transaction
    pub fn new(conn: &'c mut Connection<C>) -> Result<Self, FbError> {
        let data = TransactionData::new(conn)?;

        Ok(Transaction { data, conn })
    }

    /// Commit the current transaction changes
    pub fn commit(mut self) -> Result<(), FbError> {
        let result = self.data.commit(self.conn);

        if result.is_ok() {
            ManuallyDrop::new(self);
        } else {
            let _ = self.rollback();
        }

        result
    }

    /// Commit the current transaction changes, but allowing to reuse the transaction
    pub fn commit_retaining(&mut self) -> Result<(), FbError> {
        self.data.commit_retaining(self.conn)
    }

    /// Rollback the current transaction changes, but allowing to reuse the transaction
    pub fn rollback_retaining(&mut self) -> Result<(), FbError> {
        self.data.rollback_retaining(self.conn)
    }

    /// Rollback the current transaction changes
    pub fn rollback(mut self) -> Result<(), FbError> {
        let result = self.data.rollback(self.conn);
        ManuallyDrop::new(self);
        result
    }

    /// Execute the statement without returning any row
    pub fn execute_immediate(&mut self, sql: &str) -> Result<(), FbError> {
        self.data.execute_immediate(self.conn, sql)
    }

    /// Prepare a new statement for execute
    pub fn prepare<'t,Sql:AsRef<str>>(
        &'t mut self,
        sql: Sql,
    ) -> Result<Statement<'c, 't, C>, FbError> {
        Statement::prepare(self, sql)
    }
}

impl<'c, C: FirebirdClient> Drop for Transaction<'c, C> {
    fn drop(&mut self) {
        self.data.rollback(self.conn).ok();
    }

}

//impl<'c, C: FirebirdClient> Queryable for Transaction<'c, C> {
//    fn query_iter<'a, P, R>(
//        &'a mut self,
//        sql: &str,
//        params: P,
//    ) -> Result<Box<dyn Iterator<Item = Result<R, FbError>> + 'a>, FbError>
//    where
//        P: IntoParams,
//        R: FromRow + 'static,
//    {
//        let params = params.to_params();
//
//        // Get a statement from the cache
//        let mut stmt_cache_data = StmtCache::get_or_prepare(self, sql, params.named())?;
//
//        match stmt_cache_data
//            .stmt
//            .query(self.conn, &mut self.data, params)
//        {
//            Ok(_) => {
//                let iter = StmtIter {
//                    stmt_cache_data: Some(stmt_cache_data),
//                    tr: self,
//                    _marker: Default::default(),
//                };
//
//                Ok(Box::new(iter))
//            }
//            Err(e) => {
//                // Return the statement to the cache
//                StmtCache::insert_and_close(self.conn, stmt_cache_data)?;
//
//                Err(e)
//            }
//        }
//    }
//}
//
//impl<C: FirebirdClient> Execute for Transaction<'_, C> {
//    fn execute<P>(&mut self, sql: &str, params: P) -> Result<(), FbError>
//    where
//        P: IntoParams,
//    {
//        let params = params.to_params();
//
//        // Get a statement from the cache
//        let mut stmt_cache_data = StmtCache::get_or_prepare(self, sql, params.named())?;
//
//        // Do not return now in case of error, because we need to return the statement to the cache
//        let res = stmt_cache_data
//            .stmt
//            .execute(self.conn, &mut self.data, params);
//
//        // Return the statement to the cache
//        StmtCache::insert_and_close(self.conn, stmt_cache_data)?;
//
//        res?;
//
//        Ok(())
//    }
//
//    fn execute_returnable<P, R>(&mut self, sql: &str, params: P) -> Result<R, FbError>
//    where
//        P: IntoParams,
//        R: FromRow + 'static,
//    {
//        let params = params.to_params();
//
//        // Get a statement from the cache
//        let mut stmt_cache_data = StmtCache::get_or_prepare(self, sql, params.named())?;
//
//        // Do not return now in case of error, because we need to return the statement to the cache
//        let res = stmt_cache_data
//            .stmt
//            .execute2(self.conn, &mut self.data, params);
//
//        // Return the statement to the cache
//        StmtCache::insert_and_close(self.conn, stmt_cache_data)?;
//
//        FromRow::try_from(res?)
//    }
//}

impl<C: FirebirdClient> TransactionData<C>
{
    /// Start a new transaction
    fn new(conn: &mut Connection<C>) -> Result<Self, FbError> {
        let handle = conn
            .client()
            .begin_transaction(&mut conn.handle, TrIsolationLevel::ReadCommited)?;

        Ok(Self { handle })
    }

    /// Execute the statement without returning any row
    fn execute_immediate(&mut self, conn: &mut Connection<C>, sql: &str) -> Result<(), FbError> {
        conn
          .client()
          .exec_immediate(&mut conn.handle, &mut self.handle, conn.dialect, sql)
    }

    /// Commit the current transaction changes, not allowing to reuse the transaction
    pub fn commit(&mut self, conn: &mut Connection<C>) -> Result<(), FbError> {
        conn
          .client()
          .transaction_operation(&mut self.handle, TrOp::Commit)
    }

    /// Commit the current transaction changes, but allowing to reuse the transaction
    pub fn commit_retaining(&mut self, conn: &mut Connection<C>) -> Result<(), FbError> {
        conn
          .client()
          .transaction_operation(&mut self.handle, TrOp::CommitRetaining)
    }

    /// Rollback the current transaction changes, but allowing to reuse the transaction
    pub fn rollback_retaining(&mut self, conn: &mut Connection<C>) -> Result<(), FbError> {
        conn
          .client()
          .transaction_operation(&mut self.handle, TrOp::RollbackRetaining)
    }

    /// Rollback the transaction, invalidating it
    pub fn rollback(&mut self, conn: &mut Connection<C>) -> Result<(), FbError> {
        conn
          .client()
          .transaction_operation(&mut self.handle, TrOp::Rollback)
    }
}
