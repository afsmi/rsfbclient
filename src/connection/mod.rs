//!
//! Rust Firebird Client
//!
//! Connection functions
//!
use rsfbclient_core::{Dialect, FbError, FirebirdClient, FirebirdClientDbOps};
use std::{mem::ManuallyDrop};

use crate::{Transaction};
use stmt_cache::{StmtCache};

#[cfg(feature = "pool")]
pub mod pool;

///// Generic aggregate of configuration data for firebird db Connections
///// The data required for forming connections is partly client-implementation-dependent
//#[derive(Clone)]
//pub struct ConnectionConfiguration<A> {
//    attachment_conf: A,
//    dialect: Dialect,
//    stmt_cache_size: usize,
//}
//
//impl<A: Default> Default for ConnectionConfiguration<A> {
//    fn default() -> Self {
//        Self {
//            attachment_conf: Default::default(),
//            dialect: Dialect::D3,
//            stmt_cache_size: 20,
//        }
//    }
//}

/// A connection to a firebird database
pub struct Connection<C: FirebirdClient> {
    /// Database handler
    pub(crate) handle: <C as FirebirdClientDbOps>::DbHandle,

    /// Firebird dialect for the statements
    pub(crate) dialect: Dialect,

    /// Cache for the prepared statements. Contains the client
    //
    //TODO: properly disconnect on drop. Current situation is VERY BAD
    pub(crate) stmt_cache: StmtCache<C>,
}

impl<C: FirebirdClient> Connection<C> {

    pub fn client<'a>(&'a mut self) -> &'a mut C {
      &mut self.stmt_cache.cli
    }


    pub fn open(
        cli: C,
        conf: &ConnectionConfiguration<C::AttachmentConfig>,
    ) -> Result<Connection<C>, FbError> {
        let mut stmt_cache = StmtCache::new(conf.stmt_cache_size,cli);
        let handle = stmt_cache.cli.attach_database(&conf.attachment_conf)?;

        Ok(Connection {
            handle,
            dialect: conf.dialect,
            stmt_cache,
        })
    }

    /// Drop the current database
    pub fn drop_database(mut self) -> Result<(), FbError> {
        let handle = &mut self.handle;
        let client = self.client();
        client.drop_database(handle)?;

        Ok(())
    }

    /// Close the current connection.
    pub fn close(self) -> Result<(), FbError> {
        //TODO
        //let res = self.cleanup_and_detach();
        ManuallyDrop::new(self);
        //res
        Ok(())
    }


    /// Run a closure with a transaction, if the closure returns an error
    /// the transaction will rollback, else it will be committed
    pub fn with_transaction<T, F>(&mut self, closure: F) -> Result<T, FbError>
    where
        F: FnOnce(&mut Transaction<C>) -> Result<T, FbError>,
    {
        let mut tr = Transaction::new(self)?;

        let res = closure(&mut tr);

        if res.is_ok() {
            tr.commit_retaining()?;
        } else {
            tr.rollback_retaining()?;
        };

        res
    }
}



//impl<C> Queryable for Connection<C>
//where
//    C: FirebirdClient,
//{
//    fn query_iter<'a, P, R>(
//        &'a mut self,
//        sql: &str,
//        params: P,
//    ) -> Result<Box<dyn Iterator<Item = Result<R, FbError>> + 'a>, FbError>
//    where
//        P: IntoParams,
//        R: FromRow + 'static,
//    {
//        let mut tr = Transaction::new(self)?;
//        let params = params.to_params();
//
//        // Get a statement from the cache
//        let mut stmt_cache_data = StmtCache::get_or_prepare(&mut tr, sql, params.named())?;
//
//        match stmt_cache_data.stmt.query(tr.conn, &mut tr.data, params) {
//            Ok(_) => {
//                let iter = StmtIter {
//                    stmt_cache_data: Some(stmt_cache_data),
//                    tr,
//                    _marker: Default::default(),
//                };
//
//                Ok(Box::new(iter))
//            }
//            Err(e) => {
//                // Return the statement to the cache
//                StmtCache::insert_and_close(tr.conn, stmt_cache_data)?;
//
//                Err(e)
//            }
//        }
//    }
//}
//
//impl<C> Execute for Connection<C>
//where
//    C: FirebirdClient,
//{
//    fn execute<P>(&mut self, sql: &str, params: P) -> Result<(), FbError>
//    where
//        P: IntoParams,
//    {
//        let mut tr = Transaction::new(self)?;
//        let params = params.to_params();
//
//        // Get a statement from the cache
//        let mut stmt_cache_data = StmtCache::get_or_prepare(&mut tr, sql, params.named())?;
//
//        // Do not return now in case of error, because we need to return the statement to the cache
//        let res = stmt_cache_data.stmt.execute(tr.conn, &mut tr.data, params);
//
//        // Return the statement to the cache
//        StmtCache::insert_and_close(tr.conn, stmt_cache_data)?;
//
//        res?;
//
//        tr.commit()?;
//
//        Ok(())
//    }
//
//    fn execute_returnable<P, R>(&mut self, sql: &str, params: P) -> Result<R, FbError>
//    where
//        P: IntoParams,
//        R: FromRow + 'static,
//    {
//        let mut tr = Transaction::new(self)?;
//        let params = params.to_params();
//
//        // Get a statement from the cache
//        let mut stmt_cache_data = StmtCache::get_or_prepare(&mut tr, sql, params.named())?;
//
//        // Do not return now in case of error, because we need to return the statement to the cache
//        let res = stmt_cache_data.stmt.execute2(tr.conn, &mut tr.data, params);
//
//        // Return the statement to the cache
//        StmtCache::insert_and_close(tr.conn, stmt_cache_data)?;
//
//        let f_res = FromRow::try_from(res?)?;
//
//        tr.commit()?;
//
//        Ok(f_res)
//    }
//}

#[cfg(test)]
mk_tests_default! {
    use crate::*;

    #[test]
    fn remote_connection() -> Result<(), FbError> {
        let conn = cbuilder().connect()?;

        conn.close().expect("error closing the connection");

        Ok(())
    }

    #[test]
    fn query_iter() -> Result<(), FbError> {
        let mut conn = cbuilder().connect()?;

        let mut rows = 0;

        for row in conn
            .query_iter("SELECT -3 FROM RDB$DATABASE WHERE 1 = ?", (1,))?
        {
            let (v,): (i32,) = row?;

            assert_eq!(v, -3);

            rows += 1;
        }

        assert_eq!(rows, 1);

        Ok(())
    }
}
