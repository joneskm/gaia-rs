use crate::{errors::EvidenceAlreadyExistsError, types::Evidence, GenesisState};
use gears::extensions::gas::GasResultExt;
use gears::gas::store::errors::GasStoreErrors;
use gears::{
    context::{init::InitContext, QueryableContext, TransactionalContext},
    core::any::google::Any,
    extensions::corruption::UnwrapCorrupt,
    store::{database::Database, StoreKey},
    tendermint::informal::hash::Hash,
    x::{
        keepers::{slashing::EvidenceSlashingKeeper, staking::SlashingStakingKeeper},
        module::Module,
    },
};

use std::marker::PhantomData;

mod infraction;
mod query;
mod tx;

const KEY_PREFIX_EVIDENCE: [u8; 1] = [0x0];

/// Keeper of the evidence store
#[derive(Debug, Clone)]
pub struct Keeper<
    SK: StoreKey,
    StkK: SlashingStakingKeeper<SK, M>,
    SlsK: EvidenceSlashingKeeper<SK, M>,
    E: Evidence + Default,
    M: Module,
> where
    <E as std::convert::TryFrom<Any>>::Error: std::fmt::Debug,
{
    store_key: SK,
    // TODO
    #[allow(dead_code)]
    staking_keeper: StkK,
    #[allow(dead_code)]
    slashing_keeper: SlsK,
    #[allow(dead_code)]
    evidence_handler: Option<E>,
    _module: PhantomData<(M, E)>,
}

impl<
        SK: StoreKey,
        StkK: SlashingStakingKeeper<SK, M>,
        SlsK: EvidenceSlashingKeeper<SK, M>,
        E: Evidence + Default,
        M: Module,
    > Keeper<SK, StkK, SlsK, E, M>
where
    <E as std::convert::TryFrom<Any>>::Error: std::fmt::Debug,
{
    pub fn new(
        store_key: SK,
        staking_keeper: StkK,
        slashing_keeper: SlsK,
        evidence_handler: Option<E>,
    ) -> Self {
        Self {
            store_key,
            staking_keeper,
            slashing_keeper,
            evidence_handler,
            _module: PhantomData,
        }
    }

    /// genesis initializes the evidence module's state from a provided genesis
    /// state.
    pub fn init_genesis<DB: Database>(
        &self,
        ctx: &mut InitContext<'_, DB, SK>,
        genesis: GenesisState<E>,
    ) -> Result<(), EvidenceAlreadyExistsError> {
        for e in genesis.evidence {
            if self
                .evidence::<InitContext<'_, DB, SK>, DB, E>(ctx, e.hash())
                .unwrap_gas()
                .is_some()
            {
                return Err(EvidenceAlreadyExistsError(e.hash()));
            }
            self.set_evidence(ctx, &e).unwrap_gas();
        }
        Ok(())
    }

    /// evidence gets Evidence by hash in the module's KVStore.
    pub fn evidence<CTX: QueryableContext<DB, SK>, DB: Database, Ev: Evidence + Default>(
        &self,
        ctx: &CTX,
        evidence_hash: Hash,
    ) -> Result<Option<Ev>, GasStoreErrors> {
        let store = ctx.kv_store(&self.store_key);
        let store = store.prefix_store(KEY_PREFIX_EVIDENCE);
        Ok(store
            .get(evidence_hash.as_bytes())?
            .map(|bytes| Ev::decode(bytes.as_slice()).unwrap_or_corrupt()))
    }

    /// evidence_non_fallible gets Evidence by hash in the module's KVStore and doesn't panic on
    /// wrong decoding.
    pub fn evidence_non_fallible<
        CTX: QueryableContext<DB, SK>,
        DB: Database,
        Ev: Evidence + Default,
    >(
        &self,
        ctx: &CTX,
        evidence_hash: Hash,
    ) -> Result<Option<Ev>, GasStoreErrors> {
        let store = ctx.kv_store(&self.store_key);
        let store = store.prefix_store(KEY_PREFIX_EVIDENCE);
        let evidence = if let Some(bytes) = store.get(evidence_hash.as_bytes())? {
            if let Ok(ev) = Ev::decode(bytes.as_slice()) {
                Some(ev)
            } else {
                None
            }
        } else {
            None
        };
        Ok(evidence)
    }

    /// all_evidence_non_fallible gets Evidence's in the module's KVStore and doesn't panic on
    /// wrong decoding.
    pub fn all_evidence_non_fallible<
        CTX: QueryableContext<DB, SK>,
        DB: Database,
        Ev: Evidence + Default,
    >(
        &self,
        ctx: &CTX,
    ) -> Result<Vec<Ev>, GasStoreErrors> {
        let store = ctx.kv_store(&self.store_key);
        let store = store.prefix_store(KEY_PREFIX_EVIDENCE);
        let mut evidences = vec![];
        for r in store.into_range(..) {
            let (_k, v) = r?;
            if let Ok(ev) = Ev::decode(v.as_slice()) {
                evidences.push(ev)
            }
        }
        Ok(evidences)
    }

    /// set_evidence sets Evidence by hash in the module's KVStore.
    pub fn set_evidence<CTX: TransactionalContext<DB, SK>, DB: Database, Ev: Evidence>(
        &self,
        ctx: &mut CTX,
        evidence: &Ev,
    ) -> Result<(), GasStoreErrors> {
        let store = ctx.kv_store_mut(&self.store_key);
        let mut store = store.prefix_store_mut(KEY_PREFIX_EVIDENCE);
        store.set(
            evidence.hash().as_bytes().to_vec(),
            evidence.encode_to_vec(),
        )
    }
}
