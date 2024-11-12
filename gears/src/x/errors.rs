use std::{fmt::Display, num::NonZero};

use address::{AccAddress, BaseAddress};
use cosmwasm_std::Uint256;
use gas::{
    metering::GasMeteringErrors,
    store::errors::{GasStoreErrorKinds, GasStoreErrors},
};
use thiserror::Error;

use crate::{
    application::handlers::node::TxError,
    signing::{errors::SigningErrors, renderer::amino_renderer::RenderError},
    types::{base::errors::CoinsError, denom::Denom},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, thiserror::Error)]
#[error("account: {0} does not exist")]
pub struct AccountNotFound(String);

impl AccountNotFound {
    pub fn new(addr: impl Into<String>) -> Self {
        Self(addr.into())
    }
}

impl<T: address::AddressKind> From<BaseAddress<T>> for AccountNotFound {
    fn from(value: BaseAddress<T>) -> Self {
        Self(value.to_string())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SignVerificationError {
    #[error("signature list is empty")]
    EmptySignatureList,
    #[error("wrong number of signatures; expected {expected}, got {got}")]
    WrongSignatureList { expected: usize, got: usize },
    #[error("{0}")]
    AccountNotFound(#[from] AccountNotFound),
    #[error("pubkey on account is not set")]
    PubKeyNotSet,
    #[error("account sequence mismatch, expected {expected}, got {got}")]
    AccountSequence { expected: u64, got: u64 },
}

#[derive(Debug)]
pub(crate) enum AnteGasError {
    Overflow(String),
    OutOfGas(String),
}

impl Display for AnteGasError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnteGasError::Overflow(msg) => write!(f, "{msg}"),
            AnteGasError::OutOfGas(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for AnteGasError {}

impl From<GasMeteringErrors> for AnteGasError {
    fn from(error: GasMeteringErrors) -> Self {
        match error {
            GasMeteringErrors::ErrorGasOverflow(msg) => AnteGasError::Overflow(msg),
            GasMeteringErrors::ErrorOutOfGas(msg) => AnteGasError::OutOfGas(msg),
        }
    }
}

impl From<GasStoreErrors> for AnteGasError {
    fn from(error: GasStoreErrors) -> Self {
        match error.kind {
            GasStoreErrorKinds::Metering(e) => e.into(),
            GasStoreErrorKinds::Gas(e) => AnteGasError::Overflow(e.to_string()),
        }
    }
}

#[derive(Error, Debug)]
pub(crate) enum AnteError {
    #[error("insufficient fees; got: {got} required: {required}")]
    InsufficientFees { got: String, required: String },
    #[error("fee required")]
    MissingFee,
    #[error("{0}")]
    Validation(String), //TODO: consider breaking this down into more specific errors
    #[error("tx has timed out; timeout height: {timeout}, current height: {current}")]
    Timeout { timeout: u32, current: u32 },
    #[error("{0}")]
    GasError(#[from] AnteGasError),
    #[error("memo is too long, max length is {0}")]
    Memo(u64),
    #[error("tx is too long")]
    TxLen,
    #[error("account not found {0}")]
    AccountNotFound(#[from] AccountNotFound),
    #[error("{0}")]
    Gas(#[from] GasStoreErrors),
    #[error("failed to send coins: {0}")]
    CoinsSend(#[from] BankKeeperError),
    #[error("legacy amino json encoding failed")]
    LegacyAminoJson(#[from] RenderError),
    #[error("failed get sign bytes from tx: {0}")]
    Signing(#[from] SigningErrors),
}

impl From<AnteError> for TxError {
    fn from(error: AnteError) -> Self {
        let code = match &error {
            AnteError::InsufficientFees {
                got: _,
                required: _,
            } => 1,
            AnteError::MissingFee => 2,

            AnteError::Validation(_) => 3,
            AnteError::Timeout {
                timeout: _,
                current: _,
            } => 4,
            AnteError::GasError(_) => 5,
            AnteError::Memo(_) => 6,
            AnteError::TxLen => 7,
            AnteError::AccountNotFound(_) => 8,
            AnteError::CoinsSend(_) => 9,
            AnteError::Gas(_) => 10,
            AnteError::LegacyAminoJson(_) => 11,
            AnteError::Signing(_) => 12,
        };

        TxError {
            msg: format!("{error}").into(),
            code: NonZero::new(code).expect("all > 0"),
            codespace: "ante",
        }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum AuthKeeperError {
    #[error("{0}")]
    GasError(#[from] GasStoreErrors),
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum InsufficientFundsError {
    #[error("account: {account} doesn't have sufficient funds: {funds}")]
    Account { account: AccAddress, funds: Denom },
    #[error("insufficient funds, required: {required}, actual: {actual}")]
    RequiredActual { required: Uint256, actual: Uint256 },
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum BankCoinsError {
    #[error(transparent)]
    Parse(#[from] CoinsError),
    #[error("{smaller} is smaller than {bigger}")]
    Amount { smaller: Uint256, bigger: Uint256 },
    #[error("{0}")]
    Math(String),
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum BankKeeperError {
    #[error(transparent)]
    Coins(#[from] BankCoinsError),
    #[error("failed to delegate; {smaller} is smaller than {bigger}")]
    Delegation { smaller: Uint256, bigger: Uint256 },
    #[error("permission error: {0}")]
    Permission(String),
    #[error(transparent)]
    InsufficientFunds(#[from] InsufficientFundsError),
    #[error("{0}")]
    AccountNotFound(#[from] AccountNotFound),
    #[error("account doesn't have enough permission")]
    AccountPermission,
    #[error("module doesn't have enough permission")]
    ModulePermission,
    #[error("{0} is not allowed to receive funds. Probably tried send to module as account")]
    Blocked(AccAddress),
    #[error("Send disabled for denom: {0}")]
    SendDisabled(Denom),
    #[error("{0}")]
    GasError(#[from] GasStoreErrors),
}

impl From<CoinsError> for BankKeeperError {
    fn from(value: CoinsError) -> Self {
        Self::Coins(BankCoinsError::Parse(value))
    }
}
