use gas::metering::GasMeteringErrors;

use crate::application::handlers::node::TxError;

// We start at u16::MAX + 1 to ensure that the error codes don't collide with the Application codes
const OUT_OF_GAS_CODE: u32 = u16::MAX as u32 + 1;
const INVALID_TRANSACTION_CODE: u32 = u16::MAX as u32 + 2;
const INVALID_MESSAGE_CODE: u32 = u16::MAX as u32 + 3;
const GAS_ERRORS_CODE: u32 = u16::MAX as u32 + 4;

#[derive(Debug, Clone, thiserror::Error)]
pub enum RunTxError {
    #[error("there is no block gas left to run the transaction")]
    OutOfBlockGas,
    #[error("{0}")]
    InvalidTransaction(String),
    #[error("{0}")]
    InvalidMessage(String),
    #[error("{0}")]
    GasErrors(#[from] GasMeteringErrors),
    #[error(transparent)]
    Application(#[from] TxError),
}

impl RunTxError {
    pub fn code(&self) -> u32 {
        match self {
            RunTxError::OutOfBlockGas => OUT_OF_GAS_CODE,
            RunTxError::InvalidTransaction(_) => INVALID_TRANSACTION_CODE,
            RunTxError::InvalidMessage(_) => INVALID_MESSAGE_CODE,
            RunTxError::GasErrors(_) => GAS_ERRORS_CODE,
            RunTxError::Application(e) => e.code.get() as u32,
        }
    }

    pub fn codespace(&self) -> &'static str {
        match self {
            RunTxError::Application(e) => e.codespace,
            _ => "base",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("query path not found")]
    PathNotFound,
    #[error("Block height must be greater than or equal to zero")]
    InvalidHeight,
    #[error(transparent)]
    Store(#[from] kv_store::error::KVStoreError),
    #[error("error decoding query: {0}")]
    Proto(String),
    #[error("TODO: {0}")]
    TODO(#[from] anyhow::Error),
}
impl From<prost::DecodeError> for QueryError {
    fn from(value: prost::DecodeError) -> Self {
        Self::Proto(value.to_string())
    }
}

impl From<tendermint::error::proto::Error> for QueryError {
    fn from(value: tendermint::error::proto::Error) -> Self {
        Self::Proto(value.to_string())
    }
}

impl From<core_types::errors::ibc::Error> for QueryError {
    fn from(value: core_types::errors::ibc::Error) -> Self {
        Self::Proto(value.to_string())
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum TxValidation {
    #[error("must contain at least one message")]
    Empty,
    #[error("{0}")]
    Validation(String),
}
