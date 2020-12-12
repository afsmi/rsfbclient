//!
//! Rust Firebird Client
//!

//#[cfg(test)]
//#[macro_use]
//pub(crate) mod tests;

//pub mod prelude {
//  pub use crate::query::{Execute, Queryable};
//}

mod cache_client;
mod mock_client;

mod raw_handles;
mod wrappers;


#[cfg(feature = "pool")]
mod pool;
#[cfg(feature = "pool")]
pub use pool::FirebirdConnectionManager;

//mod connection;
//mod statement;
//mod transaction;
//mod named_params;

//pub use crate::{
//    connection::{Connection, ConnectionConfiguration, FirebirdClientFactory},
//    query::{Execute, Queryable},
//    statement::Statement,
//    transaction::Transaction,
// //   namedparams::NamedParams,
//};
//pub use rsfbclient_core::{Column, Dialect, FbError, FromRow, Row, SqlType};

//#[doc(hidden)]
//pub use rsfbclient_core::{charset, Charset};


