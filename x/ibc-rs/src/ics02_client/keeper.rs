use gears::context::init::InitContext;
use gears::context::query::QueryContext;
use gears::gas::store::errors::GasStoreErrors;
use gears::params::ParamsSubspaceKey;
use gears::store::database::prefix::PrefixDB;
use gears::store::store::prefix::mutable::MutablePrefixStore;
use gears::types::store::prefix::mutable::PrefixStoreMut;
use gears::{
    context::QueryableContext,
    store::{database::Database, StoreKey},
};
use ibc::primitives::ToVec;
use ibc::{core::host::types::path::ClientStatePath, primitives::proto::Protobuf};

use crate::ics02_client::types::{client_state::ClientState, query::IdentifiedClientState};
use crate::types::context::CLIENT_STATE_KEY;

use super::{params::ClientParamsKeeper, types::query::QueryClientStatesResponse, GenesisState};
use gears::context::{InfallibleContextMut, TransactionalContext};
use ibc::core::{
    client::types::proto::v1::QueryClientStatesRequest, host::types::identifiers::ClientId,
};

pub const KEY_NEXT_CLIENT_SEQUENCE: &[u8; 18] = b"nextClientSequence";
pub const KEY_CLIENT_STORE_PREFIX: &str = "clients";

#[derive(Debug, Clone)]
pub struct Keeper<SK, PSK> {
    store_key: SK,
    client_params_keeper: ClientParamsKeeper<PSK>,
}

impl<SK: StoreKey, PSK: ParamsSubspaceKey> Keeper<SK, PSK> {
    pub fn new(store_key: SK, params_subspace_key: PSK) -> Self {
        let client_params_keeper = ClientParamsKeeper {
            params_subspace_key,
        };
        Self {
            store_key,
            client_params_keeper,
        }
    }

    pub fn init_genesis<DB: Database>(
        &self,
        ctx: &mut InitContext<'_, DB, SK>,
        genesis: GenesisState,
    ) {
        self.client_params_keeper.set(ctx, genesis.params.clone());

        // TODO: the following lines(from ibc-go) have not been implemented yet:

        // // Set all client metadata first. This will allow client keeper to overwrite client and consensus state keys
        // // if clients accidentally write to ClientKeeper reserved keys.
        // if len(gs.ClientsMetadata) != 0 {
        // 	k.SetAllClientMetadata(ctx, gs.ClientsMetadata)
        // }

        // for _, client := range gs.Clients {
        // 	cs, ok := client.ClientState.GetCachedValue().(exported.ClientState)
        // 	if !ok {
        // 		panic("invalid client state")
        // 	}

        // 	if !gs.Params.IsAllowedClient(cs.ClientType()) {
        // 		panic(fmt.Sprintf("client state type %s is not registered on the allowlist", cs.ClientType()))
        // 	}

        // 	k.SetClientState(ctx, client.ClientId, cs)
        // }

        // for _, cs := range gs.ClientsConsensus {
        // 	for _, consState := range cs.ConsensusStates {
        // 		consensusState, ok := consState.ConsensusState.GetCachedValue().(exported.ConsensusState)
        // 		if !ok {
        // 			panic(fmt.Sprintf("invalid consensus state with client ID %s at height %s", cs.ClientId, consState.Height))
        // 		}

        // 		k.SetClientConsensusState(ctx, cs.ClientId, consState.Height, consensusState)
        // 	}
        // }

        self.set_next_client_sequence(ctx, genesis.next_client_sequence);
    }

    pub fn set_next_client_sequence<DB: Database>(
        &self,
        ctx: &mut InitContext<'_, DB, SK>,
        sequence: u64,
    ) {
        let mut ibc_store = ctx.kv_store_mut(&self.store_key);
        ibc_store.set(KEY_NEXT_CLIENT_SEQUENCE.to_owned(), sequence.to_be_bytes())
    }

    /// Query all client states
    pub fn client_states<DB: Database>(
        &self,
        ctx: &QueryContext<DB, SK>,
        _req: QueryClientStatesRequest,
    ) -> QueryClientStatesResponse {
        let store = ctx
            .kv_store(&self.store_key)
            .prefix_store(KEY_CLIENT_STORE_PREFIX.to_string().into_bytes());

        let mut client_states = vec![];

        for (key, raw_state) in store.into_range(..) {
            let Ok(key) = String::from_utf8(key.to_vec()) else {
                continue;
            };

            let key_split: Vec<&str> = key.split('/').collect();
            let [_, client_id, this_client_state_key] = key_split[..] else {
                continue;
            };

            if this_client_state_key != CLIENT_STATE_KEY {
                continue;
            }

            let Ok(client_id) = client_id.parse::<ClientId>() else {
                continue;
            };

            let Ok(client_state) = ClientState::decode_vec(&raw_state) else {
                continue;
            };

            let identified = IdentifiedClientState {
                client_id,
                client_state,
            };

            client_states.push(identified);
        }

        // sort client_states (as is done in ibc-go) https://github.com/cosmos/ibc-go/blob/46e020640e66f9043c14c53a4d215a5b457d6703/modules/core/02-client/keeper/grpc_query.go#L91
        client_states.sort();

        QueryClientStatesResponse {
            client_states,
            pagination: None,
        }
    }

    /// Writes the client state to the store
    pub fn client_state_set<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        client_state_path: ClientStatePath,
        client_state: ClientState,
    ) -> Result<(), GasStoreErrors> {
        let mut store = Self::client_store_mut(self, ctx, &client_state_path.0);
        store.set(CLIENT_STATE_KEY.bytes(), client_state.encode_vec())
    }

    /// Returns an isolated mutable prefix store for each client so they can read/write in separate
    /// namespaces without being able to read/write other client's data
    fn client_store_mut<'a, DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &'a mut CTX,
        client_id: &ClientId,
    ) -> PrefixStoreMut<'a, PrefixDB<DB>> {
        let prefix = format!("{KEY_CLIENT_STORE_PREFIX}/{}/", client_id).into_bytes();
        ctx.kv_store_mut(&self.store_key).prefix_store_mut(prefix)
    }
}
