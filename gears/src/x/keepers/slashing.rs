use crate::{
    context::{QueryableContext, TransactionalContext},
    x::module::Module,
};
use address::ConsAddress;
use cosmwasm_std::Decimal256;
use database::Database;
use gas::store::errors::GasStoreErrors;
use kv_store::StoreKey;
use tendermint::types::{proto::crypto::PublicKey, time::timestamp::Timestamp};

/// EvidenceSlashingKeeper defines the slashing module interface contract needed by the
/// evidence module.
pub trait EvidenceSlashingKeeper<SK: StoreKey, M: Module>: Clone + Send + Sync + 'static {
    /// get a particular validator by operator address
    fn pubkey<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
        addr: &ConsAddress,
    ) -> Result<Option<PublicKey>, GasStoreErrors>;
    /// get a particular validator by operator address
    fn has_validator_signing_info<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
        addr: &ConsAddress,
    ) -> Result<bool, GasStoreErrors>;
    fn is_tombstoned<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
        addr: &ConsAddress,
    ) -> Result<bool, GasStoreErrors>;
    fn slash_fraction_double_sign<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &CTX,
    ) -> Result<Decimal256, GasStoreErrors>;
    fn slash<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &CTX,
        addr: &ConsAddress,
        amount: Decimal256,
        validator_power: i64,
        height: i64,
    ) -> Result<(), GasStoreErrors>;
    fn jail<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &CTX,
        addr: &ConsAddress,
    ) -> Result<(), GasStoreErrors>;
    fn jail_until<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &CTX,
        addr: &ConsAddress,
        time: Timestamp,
    ) -> Result<(), GasStoreErrors>;
    fn tombstone<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &CTX,
        addr: &ConsAddress,
    ) -> Result<(), GasStoreErrors>;
}
