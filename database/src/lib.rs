//! Implementation of persistent storage or simply database

#[cfg(all(feature = "sled", feature = "rocksdb"))]
fn compile_check() {
    compile_error!("Can't use `sled` and `rocksdb` at one time. Chose only one DB")
}

pub mod error;
mod memory;
pub mod prefix;
#[cfg(feature = "rocksdb")]
pub mod rocks;
#[cfg(feature = "sled")]
pub mod sled;

use std::fmt::Debug;

pub use memory::*;

/// Default builder which implements(if enable) builds for all db's
#[derive(Debug, Clone, Default)]
pub struct DBBuilder;

/// Interface which all key value database implements
pub trait Database: Clone + Send + Sync + 'static {
    /// Return value of specific key
    fn get(&self, key: &[u8]) -> Option<Vec<u8>>;

    /// Add or overwrite new key - value
    fn put(&self, key: Vec<u8>, value: Vec<u8>);

    /// Iterate over values in database. Uses lexicographical order
    fn iterator(&self) -> impl Iterator<Item = (Box<[u8]>, Box<[u8]>)> + '_;

    /// Iterate over values in database which starts with prefix. Uses lexicographical order
    fn prefix_iterator(&self, prefix: Vec<u8>)
        -> impl Iterator<Item = (Box<[u8]>, Box<[u8]>)> + '_;
}

/// Builder for database
pub trait DatabaseBuilder<DB> {
    /// Error
    type Err: Debug;

    /// Build new database instance
    fn build<P: AsRef<std::path::Path>>(self, path: P) -> Result<DB, Self::Err>;
}
