//! Types, traits and constants to abstract over the different
//! implementations of the firebird client

pub mod charset;
mod connection;
#[cfg(feature = "date_time")]
pub mod date_time;
pub(crate) mod error;
pub mod ibase;
mod params;
mod row;

pub mod extras;

pub use charset::Charset;
pub use connection::*;
pub use error::FbError;
pub use params::*;
pub use row::*;

/// Max length that can be sent without creating a BLOB
pub const MAX_TEXT_LENGTH: usize = 32767;

#[derive(Debug, Clone)]
/// Sql parameter / column data
pub enum SqlType {
    Text(String),

    Integer(i64),

    Floating(f64),

    #[cfg(feature = "date_time")]
    Timestamp(chrono::NaiveDateTime),

    Binary(Vec<u8>),

    /// Only works in fb >= 3.0
    Boolean(bool),

    Null,
}

/// Convert the sql value to interbase format
impl From<SqlType> for (u32, u32) {
    fn from(val: SqlType) -> (u32, u32) {
        match val {
            Text(s) => {
                let is_very_long = s.len() > MAX_TEXT_LENGTH;

                if is_very_long {
                    (ibase::SQL_BLOB + 1, 1)
                } else {
                    (ibase::SQL_TEXT + 1, 0)
                }
            }
            Integer(_) => (ibase::SQL_INT64 + 1, 0),
            Floating(_) => (ibase::SQL_DOUBLE + 1, 0),
            Timestamp(_) => (ibase::SQL_TIMESTAMP + 1, 0),
            Null => (ibase::SQL_TEXT + 1, 0),
            Binary(_) => (ibase::SQL_BLOB + 1, 0),
            Boolean(_) => (ibase::SQL_BOOLEAN + 1, 0),
        }
    }
}

impl SqlType {
    /// Returns `true` if the type is `NULL`
    pub fn is_null(&self) -> bool {
        matches!(self, Null)
    }
}
