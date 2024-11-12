use gears::{
    gas::store::errors::GasStoreErrors, tendermint::informal::hash::Hash,
    types::address::ConsAddress,
};

#[derive(Debug, thiserror::Error)]
pub enum GenesisStateError {
    #[error("{0}")]
    Decode(#[from] DecodeError),
    #[error("{0}")]
    Validation(#[from] anyhow::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum TxEvidenceError {
    #[error("{0}")]
    Decode(#[from] DecodeError),
    #[error("{0}")]
    Gas(#[from] GasStoreErrors),
    #[error("Cannot handle evidence.\n{0}")]
    Handle(String),
    #[error(transparent)]
    AlreadyExists(#[from] EvidenceAlreadyExistsError),
}

#[derive(Debug, thiserror::Error)]
#[error("failure in conversion of any type into concrete evidence")]
pub struct DecodeError;

#[derive(Debug, thiserror::Error)]
#[error("Evidence with hash {0} already exists")]
pub struct EvidenceAlreadyExistsError(pub Hash);

#[derive(Debug, thiserror::Error)]
pub enum EquivocationEvidenceError {
    #[error("expected signing info for validator {0} but not found")]
    SigningInfoNotExists(ConsAddress),
}
