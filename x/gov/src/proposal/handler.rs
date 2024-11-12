use gears::context::InfallibleContextMut;
use gears::core::errors::CoreError;
use gears::gas::store::errors::GasStoreErrors;
use gears::store::database::Database;
use gears::store::StoreKey;

pub trait ProposalHandler<P, SK: StoreKey> {
    fn handle<CTX: InfallibleContextMut<DB, SK>, DB: Database>(
        &self,
        proposal: P,
        ctx: &mut CTX,
    ) -> Result<(), ProposalHandlingError>;

    fn check(proposal: &P) -> bool;
}

#[derive(Debug, thiserror::Error)]
pub enum ProposalHandlingError {
    #[error("Can't handle this proposal: decoding error")]
    Decode(#[from] CoreError),
    #[error("Can't handle this proposal: not supported subspace")]
    Subspace,
    #[error("Can't handle this proposal: no such keys in subspace")]
    KeyNotFound,
    #[error("Can't handle this proposal: invalid bytes")]
    InvalidProposal,
    #[error("Can't handle this proposal: {0}")]
    Gas(#[from] GasStoreErrors),
    #[error("{0}")]
    Other(String),
}
