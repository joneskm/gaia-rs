use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum StoreError {
    #[error(transparent)]
    Database(#[from] trees::Error),
}

pub const KEY_EXISTS_MSG: &str = "a store for every key is guaranteed to exist";
