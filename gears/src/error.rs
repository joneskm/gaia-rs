use address::AddressError;
use core_types::errors::CoreError;
use cosmwasm_std::Decimal256RangeExceeded;
use tendermint::{error::Error as TendermintError, types::time::timestamp::NewTimestampError};

use crate::{
    params::SubspaceParseError,
    types::{
        base::errors::{CoinError, CoinsError},
        errors::DenomError,
        tx::metadata::MetadataParseError,
    },
};

pub const POISONED_LOCK: &str = "poisoned lock";

#[derive(Debug, thiserror::Error)]
pub enum NumericError {
    #[error("overflow on {0}")]
    Overflow(MathOperation),
    #[error("{0}")]
    DecimalRange(#[from] Decimal256RangeExceeded),
}

impl Clone for NumericError {
    fn clone(&self) -> Self {
        match self {
            Self::Overflow(arg0) => Self::Overflow(arg0.clone()),
            Self::DecimalRange(_) => Self::DecimalRange(Decimal256RangeExceeded), // Why ZST is not clonable... Why?
        }
    }
}

#[derive(Debug, Clone, strum::Display)]
pub enum MathOperation {
    Add,
    Sub,
    Div,
    Mul,
}

#[derive(Debug, thiserror::Error)]
pub enum ProtobufError {
    #[error("{0}")]
    StdError(#[from] cosmwasm_std::StdError),
    #[error("{0}")]
    Coins(#[from] CoinsError),
    #[error("{0}")]
    Coin(#[from] CoinError),
    #[error("{0}")]
    MissingField(String),
    #[error("{0}")]
    Metadata(#[from] MetadataParseError),
    #[error("{0}")]
    Denom(#[from] DenomError),
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error("decode adress error: {0}")]
    Tendermint(#[from] TendermintError),
    #[error("decode adress error: {0}")]
    NewTimestamp(#[from] NewTimestampError),
    #[error("decode adress error: {0}")]
    AddressError(#[from] AddressError),
    #[error("{0}")]
    Custom(#[from] anyhow::Error),
}

impl From<ProtobufError> for tonic::Status {
    fn from(e: ProtobufError) -> Self {
        tonic::Status::invalid_argument(format!("{:?}", e))
    }
}

impl From<std::convert::Infallible> for ProtobufError {
    fn from(_: std::convert::Infallible) -> Self {
        unreachable!("who would return infallible error?")
    }
}

impl From<std::num::TryFromIntError> for ProtobufError {
    fn from(value: std::num::TryFromIntError) -> Self {
        Self::Custom(anyhow::anyhow!("{value}"))
    }
}

impl From<SubspaceParseError> for ProtobufError {
    fn from(value: SubspaceParseError) -> Self {
        Self::Core(CoreError::DecodeGeneral(value.to_string()))
    }
}

impl From<tendermint::types::time::duration::DurationError> for ProtobufError {
    fn from(value: tendermint::types::time::duration::DurationError) -> Self {
        Self::Core(CoreError::DecodeGeneral(value.to_string()))
    }
}
