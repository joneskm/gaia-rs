use std::collections::HashSet;

use database::Database;
use gas::store::errors::GasStoreErrors;
use kv_store::StoreKey;

use crate::{
    context::{InfallibleContext, InfallibleContextMut, QueryableContext, TransactionalContext},
    params::{
        gas::{subspace, subspace_mut},
        infallible_subspace, infallible_subspace_mut, ParamsDeserialize, ParamsSerialize,
        ParamsSubspaceKey,
    },
};

pub trait ParamsKeeper<PSK: ParamsSubspaceKey> {
    type Param: ParamsSerialize + ParamsDeserialize + Default;

    fn psk(&self) -> &PSK;

    fn check_key<SL: AsRef<[u8]>>(key: SL) -> bool {
        <Self::Param as ParamsSerialize>::keys()
            .iter()
            .map(|this| this.as_bytes())
            .collect::<HashSet<_>>()
            .contains(key.as_ref())
    }

    #[cfg(feature = "governance")]
    fn validate(key: impl AsRef<[u8]>, value: impl AsRef<[u8]>) -> bool;

    fn get<DB: Database, SK: StoreKey, CTX: InfallibleContext<DB, SK>>(
        &self,
        ctx: &CTX,
    ) -> Self::Param {
        let store = infallible_subspace(ctx, self.psk());

        store.params().unwrap_or_default()
    }

    fn try_get<DB: Database, SK: StoreKey, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
    ) -> Result<Self::Param, GasStoreErrors> {
        let store = subspace(ctx, self.psk());

        Ok(store.params()?.unwrap_or_default())
    }

    fn set<DB: Database, SK: StoreKey, KV: InfallibleContextMut<DB, SK>>(
        &self,
        ctx: &mut KV,
        params: Self::Param,
    ) {
        let mut store = infallible_subspace_mut(ctx, self.psk());

        store.params_set(&params)
    }

    fn try_set<DB: Database, SK: StoreKey, KV: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut KV,
        params: Self::Param,
    ) -> Result<(), GasStoreErrors> {
        let mut store = subspace_mut(ctx, self.psk());

        store.params_set(&params)
    }
}
