//! sled database implementation

use extensions::corruption::UnwrapCorrupt;

use crate::{error::DatabaseError, DBBuilder, Database, DatabaseBuilder};

impl DatabaseBuilder<SledDb> for DBBuilder {
    type Err = DatabaseError;

    fn build<P: AsRef<std::path::Path>>(self, path: P) -> Result<SledDb, DatabaseError> {
        SledDb::new(path)
    }
}

/// Database based on sled [crate](https://docs.rs/sled/latest/sled/)
#[derive(Debug, Clone)]
pub struct SledDb(::sled::Db);

impl SledDb {
    /// Create new `self`
    pub fn new<P: AsRef<std::path::Path>>(path: P) -> Result<Self, DatabaseError> {
        Ok(Self(::sled::open(path.as_ref())?))
    }
}

impl Database for SledDb {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.0
            .get(key)
            .unwrap_or_corrupt()
            .map(|this| this.to_vec())
    }

    fn put(&self, key: Vec<u8>, value: Vec<u8>) {
        let _ = self.0.insert(key, value).unwrap_or_corrupt();
    }

    fn iterator(&self) -> impl Iterator<Item = (Box<[u8]>, Box<[u8]>)> + '_ {
        self.0
            .iter()
            .map(|this| this.unwrap_or_corrupt())
            .map(|(key, value)| {
                (
                    key.to_vec().into_boxed_slice(),
                    value.to_vec().into_boxed_slice(),
                )
            })
    }

    fn prefix_iterator(
        &self,
        prefix: Vec<u8>,
    ) -> impl Iterator<Item = (Box<[u8]>, Box<[u8]>)> + '_ {
        self.0
            .scan_prefix(prefix)
            .map(|this| this.unwrap_or_corrupt())
            .map(|(key, value)| {
                (
                    key.to_vec().into_boxed_slice(),
                    value.to_vec().into_boxed_slice(),
                )
            })
    }
}
