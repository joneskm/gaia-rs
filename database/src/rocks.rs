//! RocksDB database implementation

use crate::{error::DatabaseError, DBBuilder, Database, DatabaseBuilder};
use std::{path::Path, sync::Arc};

use rocksdb::{DBWithThreadMode, SingleThreaded};

impl DatabaseBuilder<RocksDB> for DBBuilder {
    type Err = DatabaseError;

    fn build<P: AsRef<std::path::Path>>(self, path: P) -> Result<RocksDB, DatabaseError> {
        RocksDB::new(path)
    }
}

/// Database based on [rocksdb](https://rocksdb.org/) with help of [crate](https://docs.rs/rocksdb/latest/rocksdb/)
#[derive(Debug, Clone)]
pub struct RocksDB {
    db: Arc<DBWithThreadMode<SingleThreaded>>, // QA: Are we sure? Probably
}

// TODO: remove panics

impl RocksDB {
    /// Create new `self`
    pub fn new<P>(path: P) -> Result<RocksDB, DatabaseError>
    where
        P: AsRef<Path>,
    {
        Ok(RocksDB {
            db: Arc::new(rocksdb::DB::open_default(path)?),
        })
    }
}

impl Database for RocksDB {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.db
            .get(key)
            .unwrap_or_else(|e| panic!("unrecoverable database error {}", e)) //TODO: this is probably not the right thing to do when handling a abci or REST query. Perhaps we should have a must_get method?
    }

    fn put(&self, key: Vec<u8>, value: Vec<u8>) {
        self.db
            .put(key, value)
            .unwrap_or_else(|e| panic!("unrecoverable database error {}", e))
    }

    fn iterator(&self) -> impl Iterator<Item = (Box<[u8]>, Box<[u8]>)> + '_ {
        self.db
            .iterator(rocksdb::IteratorMode::Start)
            .map(|res| res.unwrap_or_else(|e| panic!("unrecoverable database error {}", e)))
    }

    fn prefix_iterator(
        &self,
        prefix: Vec<u8>,
    ) -> impl Iterator<Item = (Box<[u8]>, Box<[u8]>)> + '_ {
        self.db
            .prefix_iterator(&prefix)
            .map(|res| res.unwrap_or_else(|e| panic!("unrecoverable database error {}", e)))
            .take_while(move |(k, _)| k.starts_with(&prefix)) //rocks db returns keys beyond the prefix see https://github.com/rust-rocksdb/rust-rocksdb/issues/577
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn iterator_works() {
        let db = RocksDB::new("tmp/1").expect("hardcoded is valid");
        db.put(vec![1], vec![1]);
        db.put(vec![2], vec![2]);
        let got_pairs: Vec<(Box<[u8]>, Box<[u8]>)> = db.iterator().collect();

        let expected_pairs: Vec<(Box<[u8]>, Box<[u8]>)> = vec![
            (vec![1].into_boxed_slice(), vec![1].into_boxed_slice()),
            (vec![2].into_boxed_slice(), vec![2].into_boxed_slice()),
        ];

        assert_eq!(expected_pairs.len(), got_pairs.len());
        assert!(got_pairs.iter().all(|e| { expected_pairs.contains(e) }));
    }

    #[test]
    fn prefix_iterator_works() {
        let db = RocksDB::new("tmp/2").expect("hardcoded is valid");
        db.put(vec![1, 1], vec![1]);
        db.put(vec![2, 1], vec![2]);
        db.put(vec![3, 1], vec![3]);
        db.put(vec![4, 1], vec![4]);

        let got_pairs: Vec<(Box<[u8]>, Box<[u8]>)> = db.prefix_iterator(vec![2]).collect();
        let expected_pairs: Vec<(Box<[u8]>, Box<[u8]>)> =
            vec![(vec![2, 1].into_boxed_slice(), vec![2].into_boxed_slice())];

        assert_eq!(expected_pairs.len(), got_pairs.len());
        assert!(got_pairs.iter().all(|e| { expected_pairs.contains(e) }));
    }
}
