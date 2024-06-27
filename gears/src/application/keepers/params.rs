use database::Database;
use kv_store::StoreKey;

use crate::{
    context::{InfallibleContext, InfallibleContextMut, QueryableContext, TransactionalContext},
    params::{
        gas::{subspace, subspace_mut},
        infallible_subspace, infallible_subspace_mut, ParamsDeserialize, ParamsSerialize,
        ParamsSubspaceKey,
    },
    types::store::gas::errors::GasStoreErrors,
};

pub trait ParamsKeeper<PSK: ParamsSubspaceKey> {
    type Param: ParamsSerialize + ParamsDeserialize + Default;

    fn psk(&self) -> &PSK;

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
