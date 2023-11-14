use database::Database;
use gears::types::context::context::Context;
use gears::types::context::init_context::InitContext;
use gears::types::context::query_context::QueryContext;
use gears::{error::AppError, x::params::ParamsSubspaceKey};
use ibc_proto::protobuf::Protobuf;
use proto_messages::cosmos::auth::v1beta1::QueryAccountRequest;
use store::StoreKey;

use crate::{GenesisState, Keeper, Message};

#[derive(Debug, Clone)]
pub struct Handler<SK: StoreKey, PSK: ParamsSubspaceKey> {
    keeper: Keeper<SK, PSK>,
}

impl<SK: StoreKey, PSK: ParamsSubspaceKey> Handler<SK, PSK> {
    pub fn new(keeper: Keeper<SK, PSK>) -> Self {
        Handler { keeper }
    }

    pub fn handle<DB: Database>(
        &self,
        _ctx: &mut Context<'_, '_, DB, SK>,
        _msg: &Message,
    ) -> Result<(), AppError> {
        Ok(())
    }

    pub fn handle_query<DB: Database>(
        &self,
        ctx: &QueryContext<'_, DB, SK>,
        query: tendermint_proto::abci::RequestQuery,
    ) -> std::result::Result<bytes::Bytes, AppError> {
        match query.path.as_str() {
            "/cosmos.auth.v1beta1.Query/Account" => {
                let req = QueryAccountRequest::decode(query.data)
                    .map_err(|e| AppError::InvalidRequest(e.to_string()))?;

                self.keeper
                    .query_account(ctx, req)
                    .map(|res| res.encode_vec().into())
            }
            _ => Err(AppError::InvalidRequest("query path not found".into())),
        }
    }

    pub fn init_genesis<DB: Database>(
        &self,
        ctx: &mut InitContext<'_, DB, SK>,
        genesis: GenesisState,
    ) {
        self.keeper.init_genesis(ctx, genesis)
    }
}
