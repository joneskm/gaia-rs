use gears::baseapp::errors::QueryError;
use gears::context::init::InitContext;
use gears::context::query::QueryContext;
use gears::error::IBC_ENCODE_UNWRAP;
use gears::params::ParamsSubspaceKey;
use gears::store::database::Database;
use gears::store::StoreKey;
use gears::tendermint::types::proto::Protobuf;
use gears::tendermint::types::request::query::RequestQuery;
use gears::types::query::account::{QueryAccountRequest, QueryAccountResponse};
use gears::x::module::Module;
use serde::Serialize;

use crate::{GenesisState, Keeper};

#[derive(Clone, Debug)]
pub enum AuthNodeQueryRequest {
    Account(QueryAccountRequest),
}

#[derive(Clone, Serialize)]
#[serde(untagged)]
pub enum AuthNodeQueryResponse {
    Account(QueryAccountResponse),
}

#[derive(Debug, Clone)]
pub struct ABCIHandler<SK: StoreKey, PSK: ParamsSubspaceKey, M: Module> {
    keeper: Keeper<SK, PSK, M>,
}

impl<SK: StoreKey, PSK: ParamsSubspaceKey, M: Module> ABCIHandler<SK, PSK, M> {
    pub fn new(keeper: Keeper<SK, PSK, M>) -> Self {
        ABCIHandler { keeper }
    }

    pub fn typed_query<DB: Database + Send + Sync>(
        &self,
        ctx: &QueryContext<DB, SK>,
        query: AuthNodeQueryRequest,
    ) -> AuthNodeQueryResponse {
        match query {
            AuthNodeQueryRequest::Account(req) => {
                let res = self.keeper.query_account(ctx, req);
                AuthNodeQueryResponse::Account(res)
            }
        }
    }

    pub fn query<DB: Database>(
        &self,
        ctx: &QueryContext<DB, SK>,
        query: RequestQuery,
    ) -> std::result::Result<bytes::Bytes, QueryError> {
        match query.path.as_str() {
            "/cosmos.auth.v1beta1.Query/Account" => {
                let req = QueryAccountRequest::decode(query.data)?;

                Ok(self
                    .keeper
                    .query_account(ctx, req)
                    .encode_vec()
                    .expect(IBC_ENCODE_UNWRAP)
                    .into())
                // TODO:IBC
            }
            _ => Err(QueryError::PathNotFound),
        }
    }

    pub fn genesis<DB: Database>(&self, ctx: &mut InitContext<'_, DB, SK>, genesis: GenesisState) {
        self.keeper.init_genesis(ctx, genesis)
    }
}
