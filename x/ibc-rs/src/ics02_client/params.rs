use std::collections::HashMap;
use std::collections::HashSet;

use gears::context::InfallibleContext;
use gears::context::InfallibleContextMut;
use gears::gas::store::errors::GasStoreErrors;
use gears::params::gas;
use gears::params::infallible_subspace;
use gears::params::infallible_subspace_mut;
use gears::params::ParamKind;
use gears::params::ParamsDeserialize;
use gears::params::ParamsSerialize;
use gears::params::ParamsSubspaceKey;
use gears::{
    context::{QueryableContext, TransactionalContext},
    store::{
        database::{prefix::PrefixDB, Database},
        StoreKey,
    },
};
use serde::{Deserialize, Serialize};

const KEY_ALLOWED_CLIENTS: &str = "AllowedClients";

/// Params defines the set of IBC light client parameters.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message, Serialize, Deserialize)]
pub struct ClientParams {
    /// allowed_clients defines the list of allowed client state types which can be created
    /// and interacted with. If a client type is removed from the allowed clients list, usage
    /// of this client will be disabled until it is added again to the list.
    #[prost(string, repeated, tag = "1")]
    pub allowed_clients: Vec<String>,
}

impl ParamsSerialize for ClientParams {
    fn keys() -> HashSet<&'static str> {
        [KEY_ALLOWED_CLIENTS].into_iter().collect()
    }

    fn to_raw(&self) -> Vec<(&'static str, Vec<u8>)> {
        vec![(
            KEY_ALLOWED_CLIENTS,
            serde_json::to_vec(&self.allowed_clients).expect("conversion to json won't fail"),
        )]
    }
}

impl ParamsDeserialize for ClientParams {
    fn from_raw(fields: HashMap<&'static str, Vec<u8>>) -> Self {
        Self {
            allowed_clients: serde_json::from_slice(
                fields.get(KEY_ALLOWED_CLIENTS).expect("expected to exists"),
            )
            .expect("conversion from json won't fail"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClientParamsKeeper<PSK> {
    pub params_subspace_key: PSK,
}

impl<PSK: ParamsSubspaceKey> ClientParamsKeeper<PSK> {
    pub fn get<DB: Database, SK: StoreKey, KV: InfallibleContext<DB, SK>>(
        &self,
        ctx: &KV,
    ) -> ClientParams {
        let store = infallible_subspace(ctx, &self.params_subspace_key);

        store.params().unwrap_or_default()
    }

    pub fn set<DB: Database, SK: StoreKey, CTX: InfallibleContextMut<DB, SK>>(
        &self,
        ctx: &mut CTX,
        params: ClientParams,
    ) {
        let mut store = infallible_subspace_mut(ctx, &self.params_subspace_key);

        store.params_set(&params)
    }

    pub fn try_get<DB: Database, SK: StoreKey, KV: QueryableContext<DB, SK>>(
        &self,
        ctx: &KV,
    ) -> Result<ClientParams, GasStoreErrors> {
        let store = gas::subspace(ctx, &self.params_subspace_key);

        Ok(store.params()?.unwrap_or_default())
    }

    pub fn try_set<DB: Database, SK: StoreKey, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        params: ClientParams,
    ) -> Result<(), GasStoreErrors> {
        let mut store = gas::subspace_mut(ctx, &self.params_subspace_key);

        store.params_set(&params)?;

        Ok(())
    }
}
