use crate::{
    DistributionParams, DistributionParamsRaw, ValidatorAccumulatedCommission,
    ValidatorAccumulatedCommissionRaw, ValidatorOutstandingRewards, ValidatorOutstandingRewardsRaw,
    ValidatorSlashEvent, ValidatorSlashEventRaw,
};
use gears::{
    core::{errors::CoreError, query::request::PageRequest, Protobuf},
    types::{
        address::{AccAddress, AddressError, ValAddress},
        base::coins::DecimalCoins,
        errors::StdError,
        pagination::{request::PaginationRequest, response::PaginationResponse},
    },
};
use prost::Message;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize, Message)]
pub struct QueryValidatorOutstandingRewardsRequestRaw {
    #[prost(bytes, tag = "1")]
    pub validator_address: Vec<u8>,
}

impl From<QueryValidatorOutstandingRewardsRequest> for QueryValidatorOutstandingRewardsRequestRaw {
    fn from(
        QueryValidatorOutstandingRewardsRequest { validator_address }: QueryValidatorOutstandingRewardsRequest,
    ) -> Self {
        Self {
            validator_address: validator_address.into(),
        }
    }
}

/// QueryValidatorOutstandingRewardsRequest is the request type for the
/// Query/ValidatorOutstandingRewards RPC method.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QueryValidatorOutstandingRewardsRequest {
    /// validator_address defines the validator address to query for.
    pub validator_address: ValAddress,
}

impl TryFrom<QueryValidatorOutstandingRewardsRequestRaw>
    for QueryValidatorOutstandingRewardsRequest
{
    type Error = AddressError;

    fn try_from(
        QueryValidatorOutstandingRewardsRequestRaw { validator_address }: QueryValidatorOutstandingRewardsRequestRaw,
    ) -> Result<Self, Self::Error> {
        Ok(QueryValidatorOutstandingRewardsRequest {
            validator_address: ValAddress::try_from(validator_address)?,
        })
    }
}

impl Protobuf<QueryValidatorOutstandingRewardsRequestRaw>
    for QueryValidatorOutstandingRewardsRequest
{
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Message)]
pub struct QueryValidatorCommissionRequestRaw {
    #[prost(bytes, tag = "1")]
    pub validator_address: Vec<u8>,
}

impl From<QueryValidatorCommissionRequest> for QueryValidatorCommissionRequestRaw {
    fn from(
        QueryValidatorCommissionRequest { validator_address }: QueryValidatorCommissionRequest,
    ) -> Self {
        Self {
            validator_address: validator_address.into(),
        }
    }
}

/// QueryValidatorCommissionRequest is the request type for the
/// Query/ValidatorCommission RPC method
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QueryValidatorCommissionRequest {
    /// validator_address defines the validator address to query for.
    pub validator_address: ValAddress,
}

impl TryFrom<QueryValidatorCommissionRequestRaw> for QueryValidatorCommissionRequest {
    type Error = AddressError;

    fn try_from(
        QueryValidatorCommissionRequestRaw { validator_address }: QueryValidatorCommissionRequestRaw,
    ) -> Result<Self, Self::Error> {
        Ok(QueryValidatorCommissionRequest {
            validator_address: ValAddress::try_from(validator_address)?,
        })
    }
}

impl Protobuf<QueryValidatorCommissionRequestRaw> for QueryValidatorCommissionRequest {}

#[derive(Clone, Serialize, Message)]
pub struct QueryValidatorSlashesRequestRaw {
    #[prost(bytes, tag = "1")]
    pub validator_address: Vec<u8>,
    #[prost(uint64, tag = "2")]
    pub starting_height: u64,
    #[prost(uint64, tag = "3")]
    pub ending_height: u64,
    #[prost(message, optional, tag = "4")]
    pub pagination: Option<PageRequest>,
}

impl From<QueryValidatorSlashesRequest> for QueryValidatorSlashesRequestRaw {
    fn from(
        QueryValidatorSlashesRequest {
            validator_address,
            starting_height,
            ending_height,
            pagination,
        }: QueryValidatorSlashesRequest,
    ) -> Self {
        Self {
            validator_address: validator_address.into(),
            starting_height,
            ending_height,
            pagination: pagination.map(Into::into),
        }
    }
}

impl Protobuf<QueryValidatorSlashesRequestRaw> for QueryValidatorSlashesRequest {}

/// QueryValidatorSlashesRequest is the response type for the
/// Query/ValidatorSlashes RPC method.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct QueryValidatorSlashesRequest {
    /// validator_address defines the validator address to query for.
    pub validator_address: ValAddress,
    /// starting_height defines the optional starting height to query the slashes.
    pub starting_height: u64,
    /// ending_height defines the optional ending height to query the slashes.
    pub ending_height: u64,
    /// pagination defines an optional pagination for the request.
    pub pagination: Option<PaginationRequest>,
}

impl TryFrom<QueryValidatorSlashesRequestRaw> for QueryValidatorSlashesRequest {
    type Error = AddressError;

    fn try_from(
        QueryValidatorSlashesRequestRaw {
            validator_address,
            starting_height,
            ending_height,
            pagination,
        }: QueryValidatorSlashesRequestRaw,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            validator_address: ValAddress::try_from(validator_address)?,
            starting_height,
            ending_height,
            pagination: pagination.map(Into::into),
        })
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Message)]
pub struct QueryDelegationRewardsRequestRaw {
    #[prost(bytes, tag = "1")]
    pub delegator_address: Vec<u8>,
    #[prost(bytes, tag = "2")]
    pub validator_address: Vec<u8>,
}

impl From<QueryDelegationRewardsRequest> for QueryDelegationRewardsRequestRaw {
    fn from(
        QueryDelegationRewardsRequest {
            delegator_address,
            validator_address,
        }: QueryDelegationRewardsRequest,
    ) -> Self {
        Self {
            delegator_address: delegator_address.into(),
            validator_address: validator_address.into(),
        }
    }
}

/// QueryDelegationRewardsRequest is the request type for the
/// Query/DelegationRewards RPC method.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QueryDelegationRewardsRequest {
    /// delegator_address defines the delegator address to query for.
    pub delegator_address: AccAddress,
    /// validator_address defines the validator address to query for.
    pub validator_address: ValAddress,
}

impl TryFrom<QueryDelegationRewardsRequestRaw> for QueryDelegationRewardsRequest {
    type Error = AddressError;

    fn try_from(
        QueryDelegationRewardsRequestRaw {
            delegator_address,
            validator_address,
        }: QueryDelegationRewardsRequestRaw,
    ) -> Result<Self, Self::Error> {
        Ok(QueryDelegationRewardsRequest {
            delegator_address: AccAddress::try_from(delegator_address)?,
            validator_address: ValAddress::try_from(validator_address)?,
        })
    }
}

impl Protobuf<QueryDelegationRewardsRequestRaw> for QueryDelegationRewardsRequest {}

#[derive(Clone, PartialEq, Serialize, Deserialize, Message)]
pub struct QueryWithdrawAllRewardsRequestRaw {
    #[prost(bytes, tag = "1")]
    pub delegator_address: Vec<u8>,
}

impl From<QueryWithdrawAllRewardsRequest> for QueryWithdrawAllRewardsRequestRaw {
    fn from(
        QueryWithdrawAllRewardsRequest { delegator_address }: QueryWithdrawAllRewardsRequest,
    ) -> Self {
        Self {
            delegator_address: delegator_address.into(),
        }
    }
}

/// QueryDelegatorValidatorsRequest is the request type for the
/// Query/DelegatorValidators RPC method.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QueryWithdrawAllRewardsRequest {
    /// delegator_address defines the delegator address to query for.
    pub delegator_address: AccAddress,
}

impl TryFrom<QueryWithdrawAllRewardsRequestRaw> for QueryWithdrawAllRewardsRequest {
    type Error = AddressError;

    fn try_from(
        QueryWithdrawAllRewardsRequestRaw { delegator_address }: QueryWithdrawAllRewardsRequestRaw,
    ) -> Result<Self, Self::Error> {
        Ok(QueryWithdrawAllRewardsRequest {
            delegator_address: AccAddress::try_from(delegator_address)?,
        })
    }
}

impl Protobuf<QueryWithdrawAllRewardsRequestRaw> for QueryWithdrawAllRewardsRequest {}

#[derive(Clone, PartialEq, Message)]
pub struct QueryCommunityPoolRequest {}

impl Protobuf<QueryCommunityPoolRequest> for QueryCommunityPoolRequest {}

#[derive(Clone, PartialEq, Message)]
pub struct QueryParamsRequest {}

impl Protobuf<QueryParamsRequest> for QueryParamsRequest {}

// ====
// responses
// ====

#[derive(Clone, Serialize, Message)]
pub struct QueryValidatorOutstandingRewardsResponseRaw {
    #[prost(message, optional, tag = "1")]
    pub rewards: Option<ValidatorOutstandingRewardsRaw>,
}

impl From<QueryValidatorOutstandingRewardsResponse>
    for QueryValidatorOutstandingRewardsResponseRaw
{
    fn from(
        QueryValidatorOutstandingRewardsResponse { rewards }: QueryValidatorOutstandingRewardsResponse,
    ) -> Self {
        Self {
            rewards: rewards.map(Into::into),
        }
    }
}

/// QueryValidatorOutstandingRewardsResponse is the response type for the
/// Query/ValidatorOutstandingRewards RPC method.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct QueryValidatorOutstandingRewardsResponse {
    pub rewards: Option<ValidatorOutstandingRewards>,
}

impl TryFrom<QueryValidatorOutstandingRewardsResponseRaw>
    for QueryValidatorOutstandingRewardsResponse
{
    type Error = CoreError;

    fn try_from(
        QueryValidatorOutstandingRewardsResponseRaw { rewards }: QueryValidatorOutstandingRewardsResponseRaw,
    ) -> Result<Self, Self::Error> {
        let rewards = if let Some(rew) = rewards {
            Some(rew.try_into()?)
        } else {
            None
        };
        Ok(Self { rewards })
    }
}

impl Protobuf<QueryValidatorOutstandingRewardsResponseRaw>
    for QueryValidatorOutstandingRewardsResponse
{
}

#[derive(Clone, Serialize, Message)]
pub struct QueryValidatorCommissionResponseRaw {
    #[prost(message, optional, tag = "1")]
    pub commission: Option<ValidatorAccumulatedCommissionRaw>,
}

impl From<QueryValidatorCommissionResponse> for QueryValidatorCommissionResponseRaw {
    fn from(
        QueryValidatorCommissionResponse { commission }: QueryValidatorCommissionResponse,
    ) -> Self {
        Self {
            commission: commission.map(Into::into),
        }
    }
}

/// QueryValidatorCommissionResponse is the response type for the
/// Query/ValidatorOutstandingRewards RPC method.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct QueryValidatorCommissionResponse {
    /// commission defines the commision the validator received.
    pub commission: Option<ValidatorAccumulatedCommission>,
}

impl TryFrom<QueryValidatorCommissionResponseRaw> for QueryValidatorCommissionResponse {
    type Error = CoreError;

    fn try_from(
        QueryValidatorCommissionResponseRaw { commission }: QueryValidatorCommissionResponseRaw,
    ) -> Result<Self, Self::Error> {
        let commission = if let Some(com) = commission {
            Some(com.try_into()?)
        } else {
            None
        };
        Ok(Self { commission })
    }
}

impl Protobuf<QueryValidatorCommissionResponseRaw> for QueryValidatorCommissionResponse {}

#[derive(Clone, Serialize, Message)]
pub struct QueryValidatorSlashesResponseRaw {
    #[prost(message, repeated, tag = "1")]
    pub slashes: Vec<ValidatorSlashEventRaw>,
    #[prost(message, optional, tag = "2")]
    pub pagination: Option<gears::core::query::response::PageResponse>,
}

impl From<QueryValidatorSlashesResponse> for QueryValidatorSlashesResponseRaw {
    fn from(
        QueryValidatorSlashesResponse {
            slashes,
            pagination,
        }: QueryValidatorSlashesResponse,
    ) -> Self {
        Self {
            slashes: slashes.into_iter().map(Into::into).collect(),
            pagination: pagination.map(Into::into),
        }
    }
}

/// QueryValidatorSlashesResponse is the response type for the
/// Query/ValidatorSlashes RPC method.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct QueryValidatorSlashesResponse {
    /// slashes defines the slashes the validator received.
    pub slashes: Vec<ValidatorSlashEvent>,
    /// pagination defines the pagination in the response.
    pub pagination: Option<PaginationResponse>,
}

impl TryFrom<QueryValidatorSlashesResponseRaw> for QueryValidatorSlashesResponse {
    type Error = StdError;

    fn try_from(
        QueryValidatorSlashesResponseRaw {
            slashes,
            pagination,
        }: QueryValidatorSlashesResponseRaw,
    ) -> Result<Self, Self::Error> {
        let mut slashes_res = vec![];
        for slash in slashes {
            slashes_res.push(slash.try_into()?);
        }

        Ok(Self {
            slashes: slashes_res,
            pagination: pagination.map(Into::into),
        })
    }
}

impl Protobuf<QueryValidatorSlashesResponseRaw> for QueryValidatorSlashesResponse {}

#[derive(Clone, Serialize, Message)]
pub struct QueryDelegationRewardsResponseRaw {
    #[prost(bytes, optional, tag = "1")]
    pub rewards: Option<Vec<u8>>,
}

impl From<QueryDelegationRewardsResponse> for QueryDelegationRewardsResponseRaw {
    fn from(QueryDelegationRewardsResponse { rewards }: QueryDelegationRewardsResponse) -> Self {
        Self {
            rewards: rewards.map(|rewards| {
                serde_json::to_vec(&rewards).expect("serialization of domain type can't fail")
            }),
        }
    }
}

/// QueryDelegationRewardsResponse is the response type for the Query/DelegationRewards RPC method
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct QueryDelegationRewardsResponse {
    pub rewards: Option<DecimalCoins>,
}

impl TryFrom<QueryDelegationRewardsResponseRaw> for QueryDelegationRewardsResponse {
    type Error = CoreError;

    fn try_from(
        QueryDelegationRewardsResponseRaw { rewards }: QueryDelegationRewardsResponseRaw,
    ) -> Result<Self, Self::Error> {
        let rewards = if let Some(rew) = rewards {
            serde_json::from_slice(&rew).map_err(|e| CoreError::DecodeGeneral(e.to_string()))?
        } else {
            None
        };
        Ok(Self { rewards })
    }
}

impl Protobuf<QueryDelegationRewardsResponseRaw> for QueryDelegationRewardsResponse {}

#[derive(Clone, Serialize, Message)]
pub struct QueryWithdrawAllRewardsResponseRaw {
    #[prost(bytes, tag = "1")]
    pub validators: Vec<u8>,
}

impl From<QueryWithdrawAllRewardsResponse> for QueryWithdrawAllRewardsResponseRaw {
    fn from(
        QueryWithdrawAllRewardsResponse { validators }: QueryWithdrawAllRewardsResponse,
    ) -> Self {
        Self {
            validators: serde_json::to_vec(&validators)
                .expect("serialization of domain type can't fail"),
        }
    }
}

/// QueryWithdrawAllRewardsResponse is the response type for the Query/DelegationRewards RPC method
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct QueryWithdrawAllRewardsResponse {
    pub validators: Vec<String>,
}

impl TryFrom<QueryWithdrawAllRewardsResponseRaw> for QueryWithdrawAllRewardsResponse {
    type Error = CoreError;

    fn try_from(
        QueryWithdrawAllRewardsResponseRaw { validators }: QueryWithdrawAllRewardsResponseRaw,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            validators: serde_json::from_slice(&validators)
                .map_err(|e| CoreError::DecodeGeneral(e.to_string()))?,
        })
    }
}

impl Protobuf<QueryWithdrawAllRewardsResponseRaw> for QueryWithdrawAllRewardsResponse {}
// TODO: For compatibility with the version in method. Use single Protobuf for module
impl gears::tendermint::types::proto::Protobuf<QueryWithdrawAllRewardsResponseRaw>
    for QueryWithdrawAllRewardsResponse
{
}

#[derive(Clone, Serialize, Message)]
pub struct QueryCommunityPoolResponseRaw {
    #[prost(bytes, optional, tag = "1")]
    pub pool: Option<Vec<u8>>,
}

impl From<QueryCommunityPoolResponse> for QueryCommunityPoolResponseRaw {
    fn from(QueryCommunityPoolResponse { pool }: QueryCommunityPoolResponse) -> Self {
        Self {
            pool: pool.map(|pool| {
                serde_json::to_vec(&pool).expect("serialization of domain type can't fail")
            }),
        }
    }
}

/// QueryCommunityPoolResponse is the response type for the Query/CommunityPool RPC method.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct QueryCommunityPoolResponse {
    /// pool defines community pool's coins.
    pub pool: Option<DecimalCoins>,
}

impl TryFrom<QueryCommunityPoolResponseRaw> for QueryCommunityPoolResponse {
    type Error = CoreError;

    fn try_from(
        QueryCommunityPoolResponseRaw { pool }: QueryCommunityPoolResponseRaw,
    ) -> Result<Self, Self::Error> {
        let pool = if let Some(rew) = pool {
            serde_json::from_slice(&rew).map_err(|e| CoreError::DecodeGeneral(e.to_string()))?
        } else {
            None
        };
        Ok(Self { pool })
    }
}

impl Protobuf<QueryCommunityPoolResponseRaw> for QueryCommunityPoolResponse {}

#[derive(Clone, Serialize, Message)]
pub struct QueryParamsResponseRaw {
    #[prost(message, optional, tag = "1")]
    pub params: Option<DistributionParamsRaw>,
}

impl From<QueryParamsResponse> for QueryParamsResponseRaw {
    fn from(QueryParamsResponse { params }: QueryParamsResponse) -> Self {
        Self {
            params: Some(params.into()),
        }
    }
}

/// QueryParamsResponse is the response type for the Query/Params RPC method
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct QueryParamsResponse {
    pub params: DistributionParams,
}

impl TryFrom<QueryParamsResponseRaw> for QueryParamsResponse {
    type Error = CoreError;

    fn try_from(
        QueryParamsResponseRaw { params }: QueryParamsResponseRaw,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            params: params
                .ok_or(CoreError::MissingField("Missing field 'params'.".into()))?
                .try_into()
                .map_err(|e| CoreError::DecodeGeneral(format!("{e}")))?,
        })
    }
}

impl Protobuf<QueryParamsResponseRaw> for QueryParamsResponse {}
