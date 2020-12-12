
use rsfbclient_core::{
    Column, Dialect, FbError, FirebirdClient, FirebirdClientDbOps, FreeStmtOp, FromRow, Row,
    SqlType, TrIsolationLevel, TrOp,
};
use std::collections::HashSet;
use std::marker::PhantomData;

// The state of a an open transaction
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum TrState {
    // Indicates that a commit or rollback completed successfully
    // and was the last operation on the transaction
    Clean,

    // Indicates that the transaction was left in a
    // dirty state. That is, some query/statement was executed but
    // the transaction was not then commited or rolled back, or
    // an attempt to commit failed
    Dirty,
}

// Curried form of FirebirdClient which encapsulates some
// additional state:
//   - State of transaction
//   - List of loaned-out (prepared) statements
//   - List of open cursors
//
// Many checks are elided for the different handles
// This type is meant to be exposed only through wrappers
// ..which provide safety guarantees relating to the state of the
// ..set of handles
pub(super) struct HandleData<C: FirebirdClient> {

    pub(super) cli: C,
    pub(super) db_hdl: Option<<C as FirebirdClientDbOps>::DbHandle>,
    pub(super) tr_hdl: Option<C::TrHandle>,
    pub(super) stmt_hdl: Option<C::StmtHandle>,

    pub(super) tr_state: TrState,
    pub(super) cursors: HashSet<usize>,

    //prepared statements which the user hasn't returned yet
    //TODO: use a proper arena or used a fixed size set of slots, stop leaking the memory of freed handles
    pub(super) prepared_arena: Vec<Option<C::StmtHandle>>,
    //count of items in prepared_arena which are Some(x)
    pub(super) prepared_count: usize,

    //slot number of the currently loaded statement
    //used to swap a different statement into the active position
    pub(super) current_stmt: usize,
}

impl<C:FirebirdClient> Drop for HandleData<C> {
    fn drop(&mut self) {
        if self.db_hdl.is_some() {
          self.detach_database();
        }
    }
}

impl<C: FirebirdClient> HandleData<C> {

    /// Create a new instance from a client
    pub(super) fn new(client: C) -> Self {
      HandleData {
        cli: client,
        db_hdl: None,
        tr_hdl: None,
        stmt_hdl: None,
        tr_state: TrState::Clean,
        cursors: HashSet::new(),
        prepared_arena: Vec::new(),
        prepared_count: 0,
        current_stmt: 0,
      }
    }

    /// Connect to the database
    pub(super) fn attach_database(&mut self, config: &C::AttachmentConfig) -> Result<(), FbError> {
        let db_hdl = self.cli.attach_database(config)?;

        self.db_hdl = Some(db_hdl);

        Ok(())
    }


    /// Disconnect from the database
    pub(super) fn detach_database(&mut self) -> Result<(), FbError> {
        let mut db_hdl = self.db_hdl.take().unwrap();

        let res = self.cli.detach_database(&mut db_hdl);

        if res.is_err() {
            self.db_hdl = Some(db_hdl);
        }

        res
    }

    /// Drop the database
    pub(super) fn drop_database(&mut self) -> Result<(), FbError> {
        let mut db_hdl = self.db_hdl.take().unwrap();

        let res = self.cli.drop_database(&mut db_hdl);

        if res.is_err() {
            self.db_hdl = Some(db_hdl);
        }

        res
    }

    /// Execute a sql immediately, without returning rows
    pub(super) fn exec_immediate(&mut self, dialect: Dialect, sql: &str) -> Result<(), FbError> {
        self.cli.exec_immediate(
            self.db_hdl.as_mut().unwrap(),
            self.tr_hdl.as_mut().unwrap(),
            dialect,
            sql,
        )
    }

    /// Loads a statement from the arena based on index
    /// or does nothing if it's already loaded
    /// panics if the index is valid, so be careful
    pub(super) fn load_statement(&mut self, idx: usize) -> () {
        if idx != self.current_stmt {
            std::mem::swap(
                &mut self.stmt_hdl,
                &mut self.prepared_arena[self.current_stmt],
            );
            std::mem::swap(&mut self.stmt_hdl, &mut self.prepared_arena[idx]);
            self.current_stmt = idx;
        }
    }

    /// Allocate and prepare a statement
    /// Returns a statement handle
    pub(super) fn prepare_statement(&mut self, dialect: Dialect, sql: &str) -> Result<(), FbError> {
        let (stmt_type, stmt_hdl) = self.cli.prepare_statement(
            self.db_hdl.as_mut().unwrap(),
            self.tr_hdl.as_mut().unwrap(),
            dialect,
            sql,
        )?;

        self.prepared_arena.push(None);
        let new_idx = self.prepared_arena.len() - 1;
        self.load_statement(new_idx);
        self.stmt_hdl.replace(stmt_hdl);
        self.prepared_count += 1;
        self.current_stmt = new_idx;


        Ok(())
    }

    pub(super) fn prepare_statement_idx(
        &mut self,
        dialect: Dialect,
        sql: &str,
    ) -> Result<usize, FbError> {
        self.prepare_statement(dialect, sql)?;
        Ok(self.current_stmt)    
    }

    /// Closes or drops the current statement
    pub(super) fn free_statement(&mut self, op: FreeStmtOp) -> Result<(), FbError> {
        let hdl = self.stmt_hdl.as_mut().unwrap();

        let res = self.cli.free_statement(hdl, op);

        //On failure, just leave the handle in place
        //on success, take it and let it drop
        if res.is_ok() {
            self.stmt_hdl.take();
            self.prepared_count -= 1;
        }

        res
    }

    /// Attempts to close a cursor.
    pub(super) fn close_cursor(&mut self, cur_idx: usize) -> Result<(), FbError> {
        self.load_statement(cur_idx);
        let res = self.free_statement(FreeStmtOp::Close);

        if res.is_ok() {
            self.cursors.remove(&cur_idx);
        }

        res
    }

    pub(super) fn close_all_cursors(&mut self) {
        if self.cursors.len() > 0 {
            let mut old_cursors = std::mem::replace(&mut self.cursors, HashSet::new());

            for i in old_cursors.drain() {
                self.close_cursor(i);
            }
        }
    }
    /// Attempts to drop a statement.
    pub(super) fn drop_stmt(&mut self, stmt_idx: usize) -> Result<(), FbError> {
        self.load_statement(stmt_idx);
        self.free_statement(FreeStmtOp::Drop)
    }

    /// Execute the currently loaded prepared statement with parameters
    pub(super) fn execute(&mut self, params: Vec<SqlType>) -> Result<(), FbError> {
        self.cli.execute(
            self.db_hdl.as_mut().unwrap(),
            self.tr_hdl.as_mut().unwrap(),
            self.stmt_hdl.as_mut().unwrap(),
            params,
        )
    }

    ///Load and execute a statement from the arena
    pub(super) fn execute_by_idx(
        &mut self,
        stmt_idx: usize,
        params: Vec<SqlType>,
    ) -> Result<(), FbError> {
        self.load_statement(stmt_idx);
        self.execute(params)
    }

    /// Execute the prepared statement
    /// with input and output parameters.
    ///
    /// The output parameters will be returned
    /// as in the Result
    pub(super) fn execute2(&mut self, params: Vec<SqlType>) -> Result<Vec<Column>, FbError> {
        self.cli.execute2(
            self.db_hdl.as_mut().unwrap(),
            self.tr_hdl.as_mut().unwrap(),
            self.stmt_hdl.as_mut().unwrap(),
            params,
        )
    }

    pub(super) fn execute2_by_idx(
        &mut self,
        stmt_idx: usize,
        params: Vec<SqlType>,
    ) -> Result<Vec<Column>, FbError> {
        self.load_statement(stmt_idx);
        self.execute2(params)
    }

    /// Fetch rows from the executed statement, coercing the types
    /// according to the provided blr
    pub(super) fn fetch(&mut self) -> Result<Option<Vec<Column>>, FbError> {
        self.cli.fetch(
            self.db_hdl.as_mut().unwrap(),
            self.tr_hdl.as_mut().unwrap(),
            self.stmt_hdl.as_mut().unwrap(),
        )
    }

    pub(super) fn fetch_wrapped(
        &mut self,
        stmt_idx: usize
    ) -> Result<Option<Vec<Column>>, FbError> {
        self.load_statement(stmt_idx);
        self.fetch()
    }

    /// Start a new transaction
    pub(super) fn begin_transaction(&mut self, isolation_level: TrIsolationLevel) -> Result<(), FbError> {
        use TrState::*;

        let tr_hdl = self
            .cli
            .begin_transaction(&mut self.db_hdl.as_mut().unwrap(), isolation_level)?;

        self.tr_hdl = Some(tr_hdl);
        self.tr_state = Clean;

        Ok(())
    }

    /// Commit / Rollback a transaction
    pub(super) fn transaction_operation(&mut self, op: TrOp) -> Result<(), FbError> {
        self.cli
            .transaction_operation(self.tr_hdl.as_mut().unwrap(), op)
    }

    /// Commit the current transaction changes, not allowing to reuse the transaction
    pub(super) fn commit(&mut self) -> Result<(), FbError> {
        self.transaction_operation(TrOp::Commit)
    }

    /// Commit the current transaction changes, but allowing to reuse the transaction
    pub(super) fn commit_retaining(&mut self) -> Result<(), FbError> {
        self.transaction_operation(TrOp::CommitRetaining)
    }

    /// Rollback the current transaction changes, but allowing to reuse the transaction
    pub(super) fn rollback_retaining(&mut self) -> Result<(), FbError> {
        self.transaction_operation(TrOp::RollbackRetaining)
    }

    /// Rollback the transaction, invalidating it
    pub(super) fn rollback(&mut self) -> Result<(), FbError> {
        self.transaction_operation(TrOp::Rollback)
    }
}

