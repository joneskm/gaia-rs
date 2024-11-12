use crate::{
    QueryCommunityPoolRequest, QueryCommunityPoolResponse, QueryDelegationRewardsRequest,
    QueryDelegationRewardsResponse, QueryParamsRequest, QueryParamsResponse,
    QueryValidatorCommissionRequest, QueryValidatorCommissionResponse,
    QueryValidatorOutstandingRewardsRequest, QueryValidatorOutstandingRewardsResponse,
    QueryValidatorSlashesRequest, QueryValidatorSlashesResponse,
};
use clap::{Args, Subcommand};
use gears::{
    application::handlers::client::QueryHandler,
    baseapp::Query,
    cli::pagination::CliPaginationRequest,
    core::Protobuf,
    extensions::try_map::FallibleMapExt,
    types::{
        address::{AccAddress, ValAddress},
        pagination::request::PaginationRequest,
    },
};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Args, Debug)]
pub struct DistributionQueryCli {
    #[command(subcommand)]
    pub command: DistributionCommands,
}

#[derive(Subcommand, Debug)]
pub enum DistributionCommands {
    ValidatorOutstandingRewards(ValidatorOutstandingRewardsCommand),
    ValidatorCommission(ValidatorCommissionCommand),
    ValidatorSlashes(ValidatorSlashesCommand),
    Rewards(DelegationRewardsCommand),
    /// Query the amount of coins in the community pool
    CommunityPool,
    /// Query distribution params
    Params,
}

/// Query distribution outstanding (un-withdrawn) rewards for a validator and all their delegations
#[derive(Args, Debug, Clone)]
pub struct ValidatorOutstandingRewardsCommand {
    /// validator address
    pub address: ValAddress,
}

/// Query distribution validator commission
#[derive(Args, Debug, Clone)]
pub struct ValidatorCommissionCommand {
    /// validator address
    pub address: ValAddress,
}

/// Query distribution validator slashes
#[derive(Args, Debug, Clone)]
pub struct ValidatorSlashesCommand {
    /// validator address
    pub address: ValAddress,
    /// start height for slash events
    pub start_height: u64,
    /// end height for slash events
    pub end_height: u64,
    #[command(flatten)]
    pub pagination: Option<CliPaginationRequest>,
}

/// Query all distribution delegator rewards or rewards from a particular validator
#[derive(Args, Debug, Clone)]
pub struct DelegationRewardsCommand {
    /// delegator_address defines the delegator address to query for.
    pub delegator_address: AccAddress,
    /// validator_address defines the validator address to query for
    pub validator_address: ValAddress,
}

#[derive(Debug, Clone)]
pub struct DistributionQueryHandler;

impl QueryHandler for DistributionQueryHandler {
    type QueryRequest = DistributionQueryRequest;

    type QueryResponse = DistributionQueryResponse;

    type QueryCommands = DistributionQueryCli;

    fn prepare_query_request(
        &self,
        command: &Self::QueryCommands,
    ) -> anyhow::Result<Self::QueryRequest> {
        let res = match &command.command {
            DistributionCommands::ValidatorOutstandingRewards(
                ValidatorOutstandingRewardsCommand { address },
            ) => Self::QueryRequest::ValidatorOutstandingRewards(
                QueryValidatorOutstandingRewardsRequest {
                    validator_address: address.clone(),
                },
            ),
            DistributionCommands::ValidatorCommission(ValidatorCommissionCommand { address }) => {
                Self::QueryRequest::ValidatorCommission(QueryValidatorCommissionRequest {
                    validator_address: address.clone(),
                })
            }
            DistributionCommands::ValidatorSlashes(ValidatorSlashesCommand {
                address,
                start_height,
                end_height,
                pagination,
            }) => Self::QueryRequest::ValidatorSlashes(QueryValidatorSlashesRequest {
                validator_address: address.clone(),
                starting_height: *start_height,
                ending_height: *end_height,
                pagination: pagination.to_owned().try_map(PaginationRequest::try_from)?,
            }),
            DistributionCommands::Rewards(DelegationRewardsCommand {
                delegator_address,
                validator_address,
            }) => Self::QueryRequest::DelegationRewards(QueryDelegationRewardsRequest {
                delegator_address: delegator_address.clone(),
                validator_address: validator_address.clone(),
            }),
            DistributionCommands::CommunityPool => {
                Self::QueryRequest::CommunityPool(QueryCommunityPoolRequest {})
            }
            DistributionCommands::Params => Self::QueryRequest::Params(QueryParamsRequest {}),
        };

        Ok(res)
    }

    fn handle_raw_response(
        &self,
        query_bytes: Vec<u8>,
        command: &Self::QueryCommands,
    ) -> anyhow::Result<Self::QueryResponse> {
        let res = match &command.command {
            DistributionCommands::ValidatorOutstandingRewards(_) => {
                DistributionQueryResponse::ValidatorOutstandingRewards(
                    QueryValidatorOutstandingRewardsResponse::decode_vec(&query_bytes)?,
                )
            }
            DistributionCommands::ValidatorCommission(_) => {
                DistributionQueryResponse::ValidatorCommission(
                    QueryValidatorCommissionResponse::decode_vec(&query_bytes)?,
                )
            }
            DistributionCommands::ValidatorSlashes(_) => {
                DistributionQueryResponse::ValidatorSlashes(
                    QueryValidatorSlashesResponse::decode_vec(&query_bytes)?,
                )
            }
            DistributionCommands::Rewards(_) => DistributionQueryResponse::DelegationRewards(
                QueryDelegationRewardsResponse::decode_vec(&query_bytes)?,
            ),
            DistributionCommands::CommunityPool => DistributionQueryResponse::CommunityPool(
                QueryCommunityPoolResponse::decode_vec(&query_bytes)?,
            ),
            DistributionCommands::Params => {
                DistributionQueryResponse::Params(QueryParamsResponse::decode_vec(&query_bytes)?)
            }
        };

        Ok(res)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum DistributionQueryRequest {
    ValidatorOutstandingRewards(QueryValidatorOutstandingRewardsRequest),
    ValidatorCommission(QueryValidatorCommissionRequest),
    ValidatorSlashes(QueryValidatorSlashesRequest),
    DelegationRewards(QueryDelegationRewardsRequest),
    CommunityPool(QueryCommunityPoolRequest),
    Params(QueryParamsRequest),
}

impl Query for DistributionQueryRequest {
    fn query_url(&self) -> &'static str {
        match self {
            DistributionQueryRequest::ValidatorOutstandingRewards(_) => {
                "/cosmos.distribution.v1beta1.Query/ValidatorOutstandingRewards"
            }
            DistributionQueryRequest::ValidatorCommission(_) => {
                "/cosmos.distribution.v1beta1.Query/ValidatorCommission"
            }
            DistributionQueryRequest::ValidatorSlashes(_) => {
                "/cosmos.distribution.v1beta1.Query/ValidatorSlashes"
            }
            DistributionQueryRequest::DelegationRewards(_) => {
                "/cosmos.distribution.v1beta1.Query/DelegationRewards"
            }
            DistributionQueryRequest::CommunityPool(_) => {
                "/cosmos.distribution.v1beta1.Query/CommunityPool"
            }
            DistributionQueryRequest::Params(_) => "/cosmos.distribution.v1beta1.Query/Params",
        }
    }

    fn into_bytes(self) -> Vec<u8> {
        match self {
            DistributionQueryRequest::ValidatorOutstandingRewards(var) => var.encode_vec(),
            DistributionQueryRequest::ValidatorCommission(var) => var.encode_vec(),
            DistributionQueryRequest::ValidatorSlashes(var) => var.encode_vec(),
            DistributionQueryRequest::DelegationRewards(var) => var.encode_vec(),
            DistributionQueryRequest::CommunityPool(var) => var.encode_vec(),
            DistributionQueryRequest::Params(var) => var.encode_vec(),
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum DistributionQueryResponse {
    ValidatorOutstandingRewards(QueryValidatorOutstandingRewardsResponse),
    ValidatorCommission(QueryValidatorCommissionResponse),
    ValidatorSlashes(QueryValidatorSlashesResponse),
    DelegationRewards(QueryDelegationRewardsResponse),
    CommunityPool(QueryCommunityPoolResponse),
    Params(QueryParamsResponse),
}
