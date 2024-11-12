use gears::{
    application::handlers::node::{ModuleInfo, TxError},
    gas::store::errors::GasStoreErrors,
    types::base::errors::CoinsError,
    x::errors::BankKeeperError,
};

pub const SERDE_JSON_CONVERSION: &str = "conversion to json shouldn't fail";
pub const EXISTS: &str = "value guaranteed to exists";

#[derive(thiserror::Error, Debug)]
pub enum GovTxError {
    #[error(transparent)]
    Keeper(#[from] GovKeeperError),
}

impl GovTxError {
    pub fn into<MI: ModuleInfo>(self) -> TxError {
        TxError::new::<MI>(self.to_string(), nz::u16!(1))
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum TallyError {
    #[error("{0}")]
    Gas(#[from] GasStoreErrors),
    #[error("{0}")]
    Math(String),
}

#[derive(thiserror::Error, Debug, Clone)]
pub enum GovKeeperError {
    #[error("gov: no handler exists for proposal type")]
    NoHandler,
    #[error("{0}")]
    Bank(#[from] BankKeeperError),
    #[error("{0}")]
    Coins(#[from] CoinsError),
    #[error("inactive proposal {0}")]
    InactiveProposal(u64),
    #[error("unknown proposal {0}")]
    ProposalUnknown(u64),
    #[error("{0}")]
    Gas(#[from] GasStoreErrors),
    #[error("{0}")]
    Time(String),
    #[error("{0}")]
    Custom(String),
}
