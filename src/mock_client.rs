use rsfbclient_core::*;


pub enum ResponseStrategy {
  FailRandomly,
  AlwaysSucceed,
  AlwaysFail,
}


struct MockFbClient{
  //TODO: Vary responses based on strategy
  //      Right now it's essentially the "AlwaysSucceed" strategy
  //      and all statements are of type "StmtType::Select"
  //      and return no rows
  strategy: ResponseStrategy,
}


impl FirebirdClientDbOps for MockFbClient {
    type DbHandle = ();
    type AttachmentConfig = ();

    fn attach_database(
        &mut self,
        config: &Self::AttachmentConfig,
    ) -> Result<Self::DbHandle, FbError> {
      Ok(())
    }

    /// Disconnect from the database
    fn detach_database(&mut self, db_handle: &mut Self::DbHandle) -> Result<(), FbError> {
      Ok(())
    }

    /// Drop the database
    fn drop_database(&mut self, db_handle: &mut Self::DbHandle) -> Result<(), FbError> {
      Ok(())
    }
}

///Responsible for actual transaction and statement execution
impl FirebirdClientSqlOps for MockFbClient {

    /// A database handle
    type DbHandle = ();
    /// A transaction handle
    type TrHandle = ();
    /// A statement handle
    type StmtHandle = ();

    /// Start a new transaction, with the specified transaction parameter buffer
    fn begin_transaction(
        &mut self,
        db_handle: &mut Self::DbHandle,
        isolation_level: TrIsolationLevel,
    ) -> Result<Self::TrHandle, FbError> {
      Ok(())
    }

    /// Commit / Rollback a transaction
    fn transaction_operation(
        &mut self,
        tr_handle: &mut Self::TrHandle,
        op: TrOp,
    ) -> Result<(), FbError> {
      Ok(())
    }

    /// Execute a sql immediately, without returning rows
    fn exec_immediate(
        &mut self,
        db_handle: &mut Self::DbHandle,
        tr_handle: &mut Self::TrHandle,
        dialect: Dialect,
        sql: &str,
    ) -> Result<(), FbError> {
      Ok(())
    }

    /// Allocate and prepare a statement
    /// Returns the statement type and handle
    fn prepare_statement(
        &mut self,
        db_handle: &mut Self::DbHandle,
        tr_handle: &mut Self::TrHandle,
        dialect: Dialect,
        sql: &str,
    ) -> Result<(StmtType, Self::StmtHandle), FbError> {
      Ok((StmtType::Select,()))
    }

    /// Closes or drops a statement
    fn free_statement(
        &mut self,
        stmt_handle: &mut Self::StmtHandle,
        op: FreeStmtOp,
    ) -> Result<(), FbError> {
      Ok(())
    }

    /// Execute the prepared statement with parameters
    fn execute(
        &mut self,
        db_handle: &mut Self::DbHandle,
        tr_handle: &mut Self::TrHandle,
        stmt_handle: &mut Self::StmtHandle,
        params: Vec<SqlType>,
    ) -> Result<(), FbError> {
      Ok(()) 
    }

    /// Execute the prepared statement
    /// with input and output parameters.
    ///
    /// The output parameters will be returned
    /// as in the Result
    fn execute2(
        &mut self,
        db_handle: &mut Self::DbHandle,
        tr_handle: &mut Self::TrHandle,
        stmt_handle: &mut Self::StmtHandle,
        params: Vec<SqlType>,
    ) -> Result<Vec<Column>, FbError> {
      Ok(vec!())
    }

    /// Fetch rows from the executed statement, coercing the types
    /// according to the provided blr
    fn fetch(
        &mut self,
        db_handle: &mut Self::DbHandle,
        tr_handle: &mut Self::TrHandle,
        stmt_handle: &mut Self::StmtHandle,
    ) -> Result<Option<Vec<Column>>, FbError> {
      Ok(None)
    }
}
