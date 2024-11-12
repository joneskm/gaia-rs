use crate::types::{
    QueryAllEvidenceRequest, QueryAllEvidenceResponse, QueryEvidenceRequest, QueryEvidenceResponse,
};
use clap::Args;
use gears::{
    application::handlers::client::QueryHandler, cli::pagination::CliPaginationRequest,
    core::Protobuf, derive::Query, tendermint::informal::hash::Hash,
    types::pagination::request::PaginationRequest,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Args, Debug)]
pub struct EvidenceQueryCli {
    #[command(flatten)]
    pub command: EvidenceCommands,
}

/// Query evidences.
#[derive(Args, Debug, Clone)]
pub struct EvidenceCommands {
    /// validator public key
    pub hash: Option<String>,
    #[command(flatten)]
    pub pagination: CliPaginationRequest,
}

#[derive(Debug, Clone)]
pub struct EvidenceQueryHandler;

impl QueryHandler for EvidenceQueryHandler {
    type QueryRequest = EvidenceQueryRequest;

    type QueryResponse = EvidenceQueryResponse;

    type QueryCommands = EvidenceQueryCli;

    fn prepare_query_request(
        &self,
        command: &Self::QueryCommands,
    ) -> anyhow::Result<Self::QueryRequest> {
        let res = {
            if let Some(hash) = &command.command.hash {
                Self::QueryRequest::Evidence(QueryEvidenceRequest {
                    evidence_hash: Hash::from_str(hash)?,
                })
            } else {
                let pagination = PaginationRequest::try_from(command.command.pagination.clone())?;
                Self::QueryRequest::AllEvidence(QueryAllEvidenceRequest { pagination })
            }
        };

        Ok(res)
    }

    fn handle_raw_response(
        &self,
        query_bytes: Vec<u8>,
        command: &Self::QueryCommands,
    ) -> anyhow::Result<Self::QueryResponse> {
        let res = {
            if command.command.hash.is_some() {
                Self::QueryResponse::Evidence(QueryEvidenceResponse::decode_vec(&query_bytes)?)
            } else {
                Self::QueryResponse::AllEvidence(QueryAllEvidenceResponse::decode_vec(
                    &query_bytes,
                )?)
            }
        };

        Ok(res)
    }
}

#[derive(Clone, Debug, PartialEq, Query)]
pub enum EvidenceQueryRequest {
    Evidence(QueryEvidenceRequest),
    AllEvidence(QueryAllEvidenceRequest),
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Query)]
#[serde(untagged)]
pub enum EvidenceQueryResponse {
    Evidence(QueryEvidenceResponse),
    AllEvidence(QueryAllEvidenceResponse),
}
