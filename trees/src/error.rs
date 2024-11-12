#![allow(missing_docs)]

use thiserror::Error;

/// Error type for the AVL tree
#[derive(Error, Debug, PartialEq, Eq)]
pub enum Error {
    #[error("could not find requested version in DB: {0}")]
    VersionNotFound(u32),
    #[error("cannot overwrite existing version")]
    Overwrite,
}

#[derive(Error, Debug, PartialEq, Eq)]
pub(crate) enum InternalError {
    #[error("rotate error: {0}")]
    RotateError(String),
    // #[error("could not find requested version in DB: {0}")]
    // VersionNotFound(u32),
    #[error("unable to deserialize bytes to Node")]
    NodeDeserialize,
    #[error("cannot balance a node with balance factor >2 or <-2")]
    Balancing,
}

pub mod constants {
    pub const LEAF_ROTATE_ERROR: &str = "can't rotate a leaf node";
}
