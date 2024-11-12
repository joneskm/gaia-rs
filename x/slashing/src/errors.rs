use gears::{
    application::handlers::node::TxError,
    error::NumericError,
    gas::store::errors::GasStoreErrors,
    types::{
        address::{ConsAddress, ValAddress},
        decimal256::{Decimal256, Decimal256RangeExceeded},
        uint::Uint256,
    },
    x::errors::AccountNotFound,
};

#[derive(thiserror::Error, Debug)]
pub enum SlashingTxError {
    #[error(transparent)]
    Unjail(#[from] UnjailError),
}

impl From<SlashingTxError> for TxError {
    fn from(value: SlashingTxError) -> Self {
        match value {
            SlashingTxError::Unjail(e) => TxError {
                msg: format!("{e}").into(),
                code: nz::u16!(1),
                codespace: "",
            },
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ValidatorHandlingError {
    #[error("validator consensus address not found")]
    ConsensusNotFound,
    #[error("Expected signing info for validator but it is not found")]
    SigningInfoNotFound,
    #[error("{0}")]
    Params(#[from] anyhow::Error),
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum UnjailError {
    #[error("validator is jailed: {0}")]
    Jailed(ConsAddress),
    #[error("validator is not jailed: {0}")]
    NotJailed(ValAddress),
    #[error(transparent)]
    AccountNotFound(#[from] AccountNotFound),
    #[error("self delegation is not found")]
    DelegationNotFound,
    #[error(transparent)]
    Numeric(#[from] NumericError),
    #[error("SelfDelegationTooLowToUnjail: {lower} less than {bigger}")]
    LowDelegation { lower: Decimal256, bigger: Uint256 },
    #[error("{0}")]
    Gas(#[from] GasStoreErrors),
}

impl From<Decimal256RangeExceeded> for UnjailError {
    fn from(value: Decimal256RangeExceeded) -> Self {
        Self::Numeric(NumericError::DecimalRange(value))
    }
}
