//! `FirebirdConnection` implementation for the native fbclient

mod connection;
pub(crate) mod ibase;
pub(crate) mod params;
pub(crate) mod row;
pub(crate) mod status;
pub(crate) mod varchar;
pub(crate) mod xsqlda;

//mod builder_native;
//pub use builder_native::*;

pub use connection::{NativeFbAttachmentConfig, NativeFbClient, RemoteConfig};
pub use connection::{DynLink, DynLoad, LinkageMarker};
