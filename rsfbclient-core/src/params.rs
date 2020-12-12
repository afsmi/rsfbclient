//! Sql parameter types and traits

use crate::SqlType;

pub use SqlType::*;

/// Implemented for types that can be sent as parameters
pub trait IntoStmtArg: Into<SqlType> {}

impl From<Vec<u8>> for SqlType {
    fn from(val: Vec<u8>) -> SqlType {
        Binary(val)
    }
}

impl From<String> for SqlType {
    fn from(val: String) -> SqlType {
        Text(val)
    }
}

impl From<bool> for SqlType {
    fn from(val: bool) -> SqlType {
        Boolean(val)
    }
}

impl From<i64> for SqlType {
    fn from(val: i64) -> SqlType {
        Integer(val as i64)
    }
}
impl From<i32> for SqlType {
    fn from(val: i32) -> SqlType {
        Integer(val as i64)
    }
}
impl From<i16> for SqlType {
    fn from(val: i16) -> SqlType {
        Integer(val as i64)
    }
}
impl From<i8> for SqlType {
    fn from(val: i8) -> SqlType {
        Integer(val as i64)
    }
}
impl From<u32> for SqlType {
    fn from(val: u32) -> SqlType {
        Integer(val as i64)
    }
}
impl From<u16> for SqlType {
    fn from(val: u16) -> SqlType {
        Integer(val as i64)
    }
}
impl From<u8> for SqlType {
    fn from(val: u8) -> SqlType {
        Integer(val as i64)
    }
}

impl From<f64> for SqlType {
    fn from(val: f64) -> SqlType {
        Floating(val as f64)
    }
}
impl From<f32> for SqlType {
    fn from(val: f32) -> SqlType {
        Floating(val as f64)
    }
}

/// Implements `IntoStmtArg` for all nullable variants
impl<T> From<Option<T>> for SqlType
where
    T: Into<SqlType>,
{
    fn from(val: Option<T>) -> SqlType {
        match val {
            None => Null,
            Some(v) => v.into(),
        }
    }
}

///// Implements `IntoStmtArg` for all borrowed variants (&str, Cow and etc)
//impl<T, B> From<B> for SqlType
//where
//    B: ToOwned<Owned = T> + ?Sized,
//    T: core::borrow::Borrow<B>,
//    T: Into<SqlType>,
//{
//    fn from(val:&B) -> SqlType {
//        val.to_owned().into()
//    }
//}

/// A set of statements indexed by the type Idx
/// Takes an an IntoIterator over the statement parameters
/// and uses it to provide positional parameters
pub trait IntoStmtArgs {
    type Idx;
    //TODO: Consider adding a "Res" associated type for the result type
    //return empty vec if no args
    fn with_indices<Indices: IntoIterator<Item = Self::Idx>>(
        self,
        indices: Indices,
    ) -> Vec<SqlType>;
}

//represents dynamic positional parameters
//(parameters that cannot actually be obtained, so the args just have to provide themselves)
pub enum DynParam {}

pub struct DynParams;

impl DynParams {
    pub fn params<P>(params: P) -> Vec<SqlType>
    where
        P: IntoStmtArgs<Idx = DynParam>,
    {
        params.with_indices(std::iter::empty::<DynParam>())
    }
}

impl IntoStmtArgs for Vec<SqlType> {
    type Idx = DynParam;
    fn with_indices<Indices: IntoIterator<Item = DynParam>>(
        self,
        _indices: Indices,
    ) -> Vec<SqlType> {
        self
    }
}

impl IntoStmtArgs for () {
    type Idx = DynParam;
    fn with_indices<Indices: IntoIterator<Item = DynParam>>(
        self,
        _indices: Indices,
    ) -> Vec<SqlType> {
        vec![]
    }
}
