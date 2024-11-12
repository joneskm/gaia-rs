use std::borrow::Cow;

use clap::{Args, Subcommand};
use gears::baseapp::Query;
use ibc::core::client::types::proto::v1::{
    QueryClientParamsRequest, QueryClientParamsResponse, QueryClientStateRequest,
    QueryClientStateResponse, QueryClientStatesRequest, QueryClientStatusRequest,
    QueryClientStatusResponse, QueryConsensusStateHeightsResponse, QueryConsensusStateRequest,
    QueryConsensusStateResponse, QueryConsensusStatesRequest, QueryConsensusStatesResponse,
};
use prost::Message;

use serde::{Deserialize, Serialize};

use crate::ics02_client::types::query::QueryClientStatesResponse;

use self::{
    client_params::PARAMS_URL, client_state::STATE_URL, client_states::STATES_URL,
    client_status::STATUS_URL, consensus_state::CONSENSUS_STATE_URL,
    consensus_states::CONSENSUS_STATES_URL,
};

pub mod client_params;
pub mod client_state;
pub mod client_states;
pub mod client_status;
pub mod consensus_state;
pub mod consensus_states;
pub mod query_handler;
#[allow(dead_code)]
pub mod query_header;
#[allow(dead_code)]
pub mod self_consensus_state;

/// IBC client query subcommands
#[derive(Args, Debug, Clone)]
pub struct ClientQueryCli {
    #[command(subcommand)]
    pub command: ClientQueryCommands,
}

#[derive(Subcommand, Debug, Clone)]
pub enum ClientQueryCommands {
    #[command(name = "params")]
    ClientParams(client_params::CliClientParams),
    #[command(name = "state")]
    ClientState(client_state::CliClientState),
    #[command(name = "states")]
    ClientStates(client_states::CliClientStates),
    #[command(name = "status")]
    ClientStatus(client_status::CliClientStatus),
    #[command(name = "consensus-state")]
    ConsensusState(consensus_state::CliConsensusState),
    #[command(name = "consensus-states")]
    ConsensusStates(consensus_states::CliConsensusStates),
    // Header(query_header::CliClientParams),
    // SelfConsensusState(self_consensus_state::CliClientParams),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ClientQuery {
    ClientParams(QueryClientParamsRequest),
    ClientState(QueryClientStateRequest),
    ClientStates(QueryClientStatesRequest),
    ClientStatus(QueryClientStatusRequest),
    ConsensusState(QueryConsensusStateRequest),
    ConsensusStates(QueryConsensusStatesRequest),
}

impl Query for ClientQuery {
    fn query_url(&self) -> &'static str {
        match self {
            ClientQuery::ClientParams(_) => PARAMS_URL,
            ClientQuery::ClientState(_) => STATE_URL,
            ClientQuery::ClientStates(_) => STATES_URL,
            ClientQuery::ClientStatus(_) => STATUS_URL,
            ClientQuery::ConsensusState(_) => CONSENSUS_STATE_URL,
            ClientQuery::ConsensusStates(_) => CONSENSUS_STATES_URL,
        }
    }

    fn into_bytes(self) -> Vec<u8> {
        match self {
            ClientQuery::ClientParams(var) => var.encode_to_vec(),
            ClientQuery::ClientState(var) => var.encode_to_vec(),
            ClientQuery::ClientStates(var) => var.encode_to_vec(),
            ClientQuery::ClientStatus(var) => var.encode_to_vec(),
            ClientQuery::ConsensusState(var) => var.encode_to_vec(),
            ClientQuery::ConsensusStates(var) => var.encode_to_vec(),
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum ClientQueryResponse {
    ClientParams(QueryClientParamsResponse),
    ClientState(QueryClientStateResponse),
    ClientStates(QueryClientStatesResponse),
    ClientStatus(QueryClientStatusResponse),
    ConsensusState(QueryConsensusStateResponse),
    ConsensusStates(QueryConsensusStatesResponse),
    ConsensusStateHeights(QueryConsensusStateHeightsResponse),
}
