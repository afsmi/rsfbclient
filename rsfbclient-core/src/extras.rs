use crate::{FirebirdClient, FirebirdClientDbOps};


/// A type that can produce a configured client, for use in connection pools
pub trait FirebirdClientConfiguration : Sync {
  type Client: FirebirdClient;

  fn new_instance(&self) -> Self::Client;

}

/// If a given client allows cloning and is Sync, then that such a client
/// can be naively used to get a new one
impl<C: FirebirdClient + Clone + Sync> FirebirdClientConfiguration for C {
  type Client = Self;
  
  fn new_instance(&self) -> Self {
    self.clone()
  }

}
