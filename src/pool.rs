//!
//! Rust Firebird Client
//!
//! R2D2 Connection Pool
//!

use rsfbclient_core::{FirebirdClient, FirebirdClientDbOps, FirebirdClientSqlOps, FbError};
use rsfbclient_core::extras::FirebirdClientConfiguration;
use super::wrappers::{DisconnectedFbClient,ConnectedFbClient,DropBehavior};


/// A manager for connection pools. Requires the `pool` feature.
pub struct FirebirdConnectionManager<Conf: FirebirdClientConfiguration> {
    client_config: Conf,
    attachment_config: <Conf::Client as FirebirdClientDbOps>::AttachmentConfig,
}

impl<Conf: FirebirdClientConfiguration> FirebirdConnectionManager<Conf>
{
    pub fn new(client_config: Conf, attachment_config: <Conf::Client as FirebirdClientDbOps>::AttachmentConfig) -> Self {
        Self { client_config , attachment_config }
    }
}

impl<Conf: 'static +  FirebirdClientConfiguration> r2d2::ManageConnection for FirebirdConnectionManager<Conf>
where
  Conf: Send + Sync,
  <Conf::Client as FirebirdClientDbOps>::AttachmentConfig: Sync

{
    type Connection = ConnectedFbClient<Conf::Client>;
    type Error = FbError;

    fn connect(&self) -> Result< Self::Connection, Self::Error> {
        let disconnected = DisconnectedFbClient::new( self.client_config.new_instance() );
        
        let res = disconnected.connect(&self.attachment_config);
        
        res.map_err(|(_,e)| e)
    }

    fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        // If it can start a transaction, we are ok
        conn.with_transaction(DropBehavior::Ignore,|_| Ok(()) );
        Ok(())
    }

    fn has_broken(&self, _conn: &mut Self::Connection) -> bool {
        false
    }
}
