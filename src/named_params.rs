/// Named params support
///
/// Works on top of firebird positional parameters (`?`)

use rsfbclient_core::{FbError, SqlType};
use regex::{Captures, Regex};


//#[derive(Clone)]
//pub struct NamedParams {
////TODO: delegate impls of PartialEq,Eq,Hash to sql field
//    pub sql: String,
//    params_names: Vec<String>
//}
//impl AsRef<str> for NamedParams {
//  fn as_ref(&self) -> &str {
//    self.sql.as_ref()
//  }
//}
//
//impl<'a> IntoIterator for &'a NamedParams {
//  type Item = &'a String;
//  type IntoIter = std::slice::Iter<'a, String>;
//
//  fn into_iter(self) -> Self::IntoIter {
//    self.params_names.into_iter()
//  }
//}
//
//impl<&'a, C: FirebirdClient> TryFrom<&'a Statement<'_,'_,C>> for &'a NamedParams {
//  type Error=FbError;
//  fn try_from(stmt: &'a Statement) -> Result<&'a NamedParams, FbError> {
//    if let NamedParamsSql(params) = stmt.data.sql {
//      Ok(&params)
//    } else {
//      Err(FbError::from("erroneous request for named parameters. statement's parameters are non-named")
//    }
//  }
//
//}
//
//impl<&'a, C: FirebirdClient> TryFrom<&'a Statement<'_,'_,C>> for DynParams {
//  type Error=FbError;
//  fn try_from(stmt: &'a Statement) -> Result<DynParams, FbError> {
//    if let PlainSql(_) = stmt.data.sql {
//      Ok(DynParams)
//    } else {
//      Err(FbError::from("erreneous request for positional parameters. statement's parameters are non-positional")
//    }
//  }
//
//}
//
//
//impl NamedParams {
//    /// Parse the sql statement and return a
//    /// structure representing the named parameters found
//    pub fn parse(raw_sql: &str) -> Result<Self, FbError> {
//        let rparams = Regex::new(r#"('[^']*')|:\w+"#)
//            .map_err(|e| FbError::from(format!("Error on start the regex for named params: {}", e)))
//            .unwrap();
//
//        let mut params_names = vec![];
//        let sql = rparams
//            .replace_all(raw_sql, |caps: &Captures| match caps.get(1) {
//                Some(same) => same.as_str().to_string(),
//                None => "?".to_string(),
//            })
//            .to_string();
//
//        for params in rparams.captures_iter(raw_sql) {
//            for param in params
//                .iter()
//                .filter(|p| p.is_some())
//                .map(|p| p.unwrap().as_str())
//                .filter(|p| p.starts_with(':'))
//            {
//                params_names.push(param.replace(":", ""));
//            }
//        }
//
//        Ok(NamedParams { sql, params_names })
//    }
//
//    /// Returns the sql as is, disabling named parameter function
//    pub fn empty(raw_sql: &str) -> Self {
//        Self {
//            sql: raw_sql.to_string(),
//            params_names: Default::default(),
//        }
//    }
//}
////    /// Re-sort/convert the parameters, applying
////    /// the named params support
////    pub fn convert<P>(&self, params: P) -> Result<Vec<SqlType>, FbError>
////    where
////        P: TryInto<NamedParams>,
////    {
////      
////        let names = params.try_into()?;
////
////        let mut new_params = vec![];
////        for qname in &self.params_names {
////            if let Some(param) = names.get(qname) {
////                new_params.push(param.clone());
////            } else {
////                return Err(FbError::from(format!(
////                    "Param :{} not found in the provided struct",
////                    qname
////                )));
////            }
////        }
////
////        Ok(new_params)
////    }
////}
