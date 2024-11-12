use gears::application::handlers::client::QueryHandler;
use ibc::primitives::proto::Protobuf as _;

use crate::ics02_client::client::cli::query::query_handler::ClientQueryHandler;

use std::borrow::Cow;

use clap::{Args, Subcommand};
use gears::baseapp::Query;
use prost::Message;
use serde::{Deserialize, Serialize};

use crate::ics02_client::client::cli::query::{ClientQuery, ClientQueryCli, ClientQueryResponse};

/// Querying commands for the ibc module
#[derive(Args, Debug)]
pub struct IbcQueryCli {
    #[command(subcommand)]
    pub command: IbcQueryCommands,
}

#[derive(Subcommand, Debug, Clone)]
pub enum IbcQueryCommands {
    Client(ClientQueryCli),
}

#[derive(Clone, Debug, PartialEq)]
pub enum IbcQuery {
    Client(ClientQuery),
}

impl Query for IbcQuery {
    fn query_url(&self) -> &'static str {
        match self {
            IbcQuery::Client(query) => query.query_url(),
        }
    }

    fn into_bytes(self) -> Vec<u8> {
        match self {
            IbcQuery::Client(query) => query.into_bytes(),
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum IbcQueryResponse {
    Client(ClientQueryResponse),
}

impl IbcQueryResponse {
    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            IbcQueryResponse::Client(q) => match q {
                ClientQueryResponse::ClientParams(q) => q.encode_to_vec(),
                ClientQueryResponse::ClientState(q) => q.encode_to_vec(),
                ClientQueryResponse::ClientStates(q) => q.encode_vec(),
                ClientQueryResponse::ClientStatus(q) => q.encode_to_vec(),
                ClientQueryResponse::ConsensusState(q) => q.encode_to_vec(),
                ClientQueryResponse::ConsensusStates(q) => q.encode_to_vec(),
                ClientQueryResponse::ConsensusStateHeights(q) => q.encode_to_vec(),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct IbcQueryHandler;

impl QueryHandler for IbcQueryHandler {
    type QueryRequest = IbcQuery;
    type QueryCommands = IbcQueryCli;
    type QueryResponse = IbcQueryResponse;

    fn prepare_query_request(
        &self,
        command: &Self::QueryCommands,
    ) -> anyhow::Result<Self::QueryRequest> {
        let res = match &command.command {
            IbcQueryCommands::Client(command) => {
                Self::QueryRequest::Client(ClientQueryHandler.prepare_query_request(command)?)
            }
        };

        Ok(res)
    }

    fn handle_raw_response(
        &self,
        query_bytes: Vec<u8>,
        command: &Self::QueryCommands,
    ) -> anyhow::Result<Self::QueryResponse> {
        let res = match &command.command {
            IbcQueryCommands::Client(command) => Self::QueryResponse::Client(
                ClientQueryHandler.handle_raw_response(query_bytes, command)?,
            ),
        };

        Ok(res)
    }
}
