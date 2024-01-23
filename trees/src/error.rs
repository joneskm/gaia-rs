use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum Error {
    #[error("rotate error")]
    RotateError,
    #[error("could not find requested version in DB")]
    VersionNotFound,
    #[error("unable to deserialize bytes to Node")]
    NodeDeserialize,
    #[error("cannot overwrite existing version")]
    Overwrite,
    #[error("invalid data in database - possible database corruption")]
    DatabaseCorruption,
}
