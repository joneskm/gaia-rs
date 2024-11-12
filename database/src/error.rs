//! Errors
#![allow(missing_docs)]

#[cfg(any(feature = "rocksdb", feature = "sled"))]
pub use self::inner::*;

#[cfg(any(feature = "rocksdb", feature = "sled"))]
mod inner {
    use thiserror::Error;

    /// Database related errors
    #[derive(Error, Debug, PartialEq, Eq)]
    pub enum DatabaseError {
        #[cfg(feature = "rocksdb")]
        #[error(transparent)]
        Rocks(#[from] rocksdb::Error),
        #[cfg(feature = "sled")]
        #[error(transparent)]
        Sleb(#[from] sled::Error),
    }
}
