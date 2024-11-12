use crate::query::{
    QueryAccountRequest, QueryAccountResponse, QueryAccountsRequest, QueryAccountsResponse,
    QueryParamsRequest, QueryParamsResponse,
};
use bytes::Bytes;
use clap::{Args, Subcommand};
use gears::core::Protobuf;
use gears::derive::Query;
use gears::extensions::try_map::FallibleMapExt;
use gears::types::address::AccAddress;
use gears::types::pagination::request::PaginationRequest;
use gears::{application::handlers::client::QueryHandler, cli::pagination::CliPaginationRequest};
use serde::{Deserialize, Serialize};

#[derive(Args, Debug)]
pub struct AuthQueryCli {
    #[command(subcommand)]
    pub command: AuthCommands,
}

#[derive(Subcommand, Debug)]
pub enum AuthCommands {
    Account(AccountCommand),
    Accounts(AccountsCommand),
    Params,
}

/// Query for account by address
#[derive(Args, Debug, Clone)]
pub struct AccountCommand {
    /// address
    pub address: AccAddress,
}

/// Query all the accounts
#[derive(Args, Debug, Clone)]
pub struct AccountsCommand {
    #[command(flatten)]
    pub pagination: Option<CliPaginationRequest>,
}

#[derive(Clone, Debug, PartialEq, Query)]
#[query(request)]
pub enum AuthQuery {
    Account(QueryAccountRequest),
    Accounts(QueryAccountsRequest),
    Params(QueryParamsRequest),
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Query)]
#[query(response)]
#[serde(untagged)]
pub enum AuthQueryResponse {
    Account(QueryAccountResponse),
    Accounts(QueryAccountsResponse),
    Params(QueryParamsResponse),
}

#[derive(Debug, Clone)]
pub struct AuthQueryHandler;

impl QueryHandler for AuthQueryHandler {
    type QueryRequest = AuthQuery;

    type QueryCommands = AuthQueryCli;

    type QueryResponse = AuthQueryResponse;

    fn prepare_query_request(
        &self,
        command: &Self::QueryCommands,
    ) -> anyhow::Result<Self::QueryRequest> {
        let res = match &command.command {
            AuthCommands::Account(AccountCommand { address }) => {
                AuthQuery::Account(QueryAccountRequest {
                    address: address.clone(),
                })
            }
            AuthCommands::Accounts(cmd) => {
                let pagination = cmd
                    .pagination
                    .to_owned()
                    .try_map(PaginationRequest::try_from)?;
                AuthQuery::Accounts(QueryAccountsRequest { pagination })
            }
            AuthCommands::Params => AuthQuery::Params(QueryParamsRequest {}),
        };

        Ok(res)
    }

    fn handle_raw_response(
        &self,
        query_bytes: Vec<u8>,
        command: &Self::QueryCommands,
    ) -> anyhow::Result<Self::QueryResponse> {
        let res = match command.command {
            AuthCommands::Account(_) => {
                AuthQueryResponse::Account(QueryAccountResponse::decode::<Bytes>(
                    query_bytes.into(),
                )?)
            }
            AuthCommands::Accounts(_) => AuthQueryResponse::Accounts(
                QueryAccountsResponse::decode::<Bytes>(query_bytes.into())?,
            ),
            AuthCommands::Params => {
                AuthQueryResponse::Params(QueryParamsResponse::decode::<Bytes>(query_bytes.into())?)
            }
        };

        Ok(res)
    }
}
