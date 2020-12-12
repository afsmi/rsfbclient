//!
//! Rust Firebird Client
//!
//! Statement Cache
//!

use lru_cache::LruCache;

use crate::{statement::StatementData};
use rsfbclient_core::{FirebirdClient,FreeStmtOp};
/// Cache of prepared statements.
///
/// Must be emptied by calling `close_all` before dropping.
pub struct StmtCache<C:FirebirdClient> {
    cache: LruCache<String, StatementData<C>>,
    pub cli: C,
}


/// General functions
impl<C: FirebirdClient> StmtCache<C> {
    pub fn new(capacity: usize, cli: C) -> Self {
        Self {
            cli,
            cache: LruCache::new(capacity),
        }
    }

    /// Get a prepared statement from the cache
    pub fn get<S: AsRef<str>>(&mut self, sql: S) -> Option<StatementData<C>> {
        self.cache.remove(&sql.as_ref().to_string())
    }

    /// Adds a prepared statement to the cache, returning the previous one for this sql
    /// or another if the cache is full
    pub fn insert(&mut self, data: StatementData<C>) -> () {

      let is_cache_full = self.cache.len() == self.cache.capacity();


      //possibilities:
      //  Cache not full, does not already contain sql (return None)
      //  Cache full, does not already contain sql (remove lru, insert new, return the evicted lru)
      //  already contains sql (return old value for key)
      //
      //  if data is returned in the above, it is dropped as it represents either an evicted value or
      //  one that has been replaced

      let mut res = None;

      let sql_str = data.as_ref().to_string();
      
      if is_cache_full {
        if let Some((_k, evicted)) = self.cache.remove_lru(){
          self.cache.insert(sql_str, data);
          res = Some(evicted)
        }
      } else {
        res = self.cache.insert(sql_str, data);
      }

      if let Some(mut data) = res {
        self.cli.free_statement(&mut data.handle, FreeStmtOp::Drop);
      }

    }

}

impl<C:FirebirdClient> Drop for StmtCache<C> {
  fn drop(&mut self) {
    for (_, stmt) in self.cache.iter_mut() {
      self.cli.free_statement(
        &mut stmt.handle,
        FreeStmtOp::Drop
      );
    }
  }
}


#[test]
fn stmt_cache_test() {
    let mut cache = StmtCache::new(2);

    let mk_test_data = |n: usize| StmtCacheData {
        sql: format!("sql {}", n),
        stmt: n,
    };

    let sql1 = mk_test_data(1);
    let sql2 = mk_test_data(2);
    let sql3 = mk_test_data(3);
    let sql4 = mk_test_data(4);
    let sql5 = mk_test_data(5);
    let sql6 = mk_test_data(6);

    assert!(cache.get(&sql1.sql).is_none());

    assert!(cache.insert(sql1).is_none());

    assert!(cache.insert(sql2).is_none());

    let stmt = cache.insert(sql3).expect("sql1 not returned");
    assert_eq!(stmt, 1);

    assert!(cache.get("sql 1").is_none());

    // Marks sql2 as recently used, so 3 must be removed in the next insert
    let sql2 = cache.get("sql 2").expect("Sql 2 not in the cache");
    assert!(cache.insert(sql2).is_none());

    let stmt = cache.insert(sql4).expect("sql3 not returned");
    assert_eq!(stmt, 3);

    let stmt = cache.insert(sql5).expect("sql2 not returned");
    assert_eq!(stmt, 2);

    let stmt = cache.insert(sql6).expect("sql4 not returned");
    assert_eq!(stmt, 4);

    assert_eq!(cache.get("sql 5").expect("sql5 not in the cache").stmt, 5);
    assert_eq!(cache.get("sql 6").expect("sql6 not in the cache").stmt, 6);

    assert!(cache.cache.is_empty());
    assert!(cache.sqls.is_empty());
}
