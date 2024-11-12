use gears::{
    error::NumericError, gas::store::errors::GasStoreErrors, types::{
        address::{AccAddress, ValAddress},
        base::errors::CoinsError,
    }, x::errors::{AccountNotFound, BankKeeperError}
};

#[derive(Debug, Clone, thiserror::Error)]
pub enum DistributionTxError {
    #[error(transparent)]
    DelegatorValidator(#[from] DistributionError),
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum TokenAllocationError {
    #[error(transparent)]
    AccountNotFound(#[from] AccountNotFound),
    #[error("Stored fee pool should not have been none")]
    FeePoolNone,
    #[error("{0}")]
    Numeric(#[from] NumericError),
    #[error("{0}")]
    Coins(#[from] CoinsError),
    #[error("{0}")]
    BankSend(#[from] BankKeeperError),
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum DistributionError {
    #[error("Stored fee pool should not have been none")]
    FeePoolNone,
    #[error("validator current rewards are not found: {0}")]
    ValidatorCurrentRewardsNotFound(ValAddress),
    #[error("validator outstanding rewards are not found: {0}")]
    ValidatorOutstandingRewardsNotFound(ValAddress),
    #[error("validator starting rewards are not found: {0}")]
    ValidatorHistoricalRewardsNotFound(ValAddress),
    #[error("{0}")]
    ValidatorHistoricalRewardsCounterError(#[from] ValidatorHistoricalRewardsReferenceCountError),
    #[error("validator accumulated commission is not found: {0}")]
    ValidatorAccumulatedCommissionNotFound(ValAddress),
    #[error("validator accumulated commission is not found: {0}")]
    ValidatorAccumulatedCommissionNotSet(ValAddress),
    #[error("delegation rewards of delegator {0} to validator {1} is not found")]
    DelegationRewardsNotFound(AccAddress, ValAddress),
    #[error("delegator starting info of delegator {0} and validator {1} is not found")]
    DelegatorStartingInfoNotFound(AccAddress, ValAddress),
    #[error("delegation of delegator {0} to validator {1} is not found")]
    DelegationNotFound(AccAddress, ValAddress),
    #[error("cannot set negative reference count")]
    NegativeHistoricalInfoCount,
    #[error(transparent)]
    AccountNotFound(#[from] AccountNotFound),
    #[error("{0}")]
    BankSend(#[from] BankKeeperError),
    #[error("{0}")]
    Coins(#[from] CoinsError),
    #[error(transparent)]
    Numeric(#[from] NumericError),
    #[error("{0}")]
    Gas(#[from] GasStoreErrors),
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ValidatorHistoricalRewardsReferenceCountError {
    #[error("cannot create counter with value higher than upper bound.\ngot: {0}, expected: {1}")]
    CounterValueOutOfBounds(u32, u32),
    #[error("the counter reached upper bound: {0}")]
    IncrementBound(u32),
    #[error("the counter reached lower value 0")]
    DecrementBound,
}
