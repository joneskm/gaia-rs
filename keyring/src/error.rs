use thiserror::Error;

/// Error type for keyring.
#[derive(Error, Debug)]
pub enum Error {
    #[error("there was an error accessing the file at {path}: {msg}")]
    FileIO {
        source: std::io::Error,
        path: String,
        msg: String,
    },

    #[error("there was an error reading the password from stdin: {msg}")]
    IO { source: std::io::Error, msg: String },

    #[error("a key with the name {name} already exists at {location}")]
    AlreadyExists { name: String, location: String },

    #[error("could not find key ring at {0}")]
    KeyringDoesNotExist(String),

    #[error("a key with the name {name} does not exist at {location}")]
    DoesNotExist { name: String, location: String },

    #[error("the key file at {path} is corrupted: {msg}")]
    InvalidUTF8 {
        source: std::string::FromUtf8Error,
        path: String,
        msg: String,
    },

    #[error("the key file at {path} is corrupted: {msg}")]
    PKCS8 {
        source: k256::pkcs8::Error,
        path: String,
        msg: String,
    },

    #[error("the key file at {path} is corrupted: {msg}")]
    KEYSTORE {
        source: eth_keystore::KeystoreError,
        path: String,
        msg: String,
    },

    #[error("the key file at {path} is corrupted: {msg}")]
    JSON {
        source: serde_json::Error,
        path: String,
        msg: String,
    },

    #[error("the key hash file at {path} is corrupted: {msg}")]
    KeyHash {
        source: argon2::password_hash::Error,
        path: String,
        msg: String,
    },

    #[error("incorrect password")]
    IncorrectPassword,

    #[error("invalid password: {msg}")]
    InvalidPassword {
        source: argon2::password_hash::Error,
        msg: String,
    },

    #[error("could not set readonly permission on file {path}: {msg}")]
    ReadOnly {
        source: std::io::Error,
        path: String,
        msg: String,
    },

    #[error("unexpected keyring type found at {path}, expected: {expected}, found: {found}")]
    IncorrectBackend {
        path: String,
        expected: String,
        found: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, thiserror::Error)]
#[error("invalid key: {0}")]
pub struct DecodeError(pub String);
