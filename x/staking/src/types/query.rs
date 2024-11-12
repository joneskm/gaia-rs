use crate::{
    Delegation, HistoricalInfo, IbcV046Validator, Pool, Redelegation, RedelegationEntry,
    StakingParams, UnbondingDelegation, Validator,
};
use gears::{
    core::{errors::CoreError, Protobuf},
    derive::{Protobuf, Query, Raw},
    extensions::pagination::PaginationKey,
    types::{
        address::{AccAddress, AddressError, ValAddress},
        base::coin::UnsignedCoin,
        pagination::{request::PaginationRequest, response::PaginationResponse},
        uint::Uint256,
    },
    x::types::validator::BondStatus,
};
use ibc_proto::cosmos::base::query::v1beta1::PageRequest;
use serde::{Deserialize, Serialize};

mod inner {
    pub use ibc_proto::cosmos::staking::v1beta1::DelegationResponse;
    pub use ibc_proto::cosmos::staking::v1beta1::RedelegationEntryResponse;
    pub use ibc_proto::cosmos::staking::v1beta1::RedelegationResponse;
    pub use ibc_proto::cosmos::staking::v1beta1::{
        QueryDelegationRequest, QueryDelegationResponse,
    };
    pub use ibc_proto::cosmos::staking::v1beta1::{
        QueryDelegatorDelegationsRequest, QueryDelegatorDelegationsResponse,
    };
    pub use ibc_proto::cosmos::staking::v1beta1::{
        QueryDelegatorUnbondingDelegationsRequest, QueryDelegatorUnbondingDelegationsResponse,
    };
    pub use ibc_proto::cosmos::staking::v1beta1::{
        QueryDelegatorValidatorRequest, QueryDelegatorValidatorResponse,
    };
    pub use ibc_proto::cosmos::staking::v1beta1::{
        QueryDelegatorValidatorsRequest, QueryDelegatorValidatorsResponse,
    };
    pub use ibc_proto::cosmos::staking::v1beta1::{
        QueryHistoricalInfoRequest, QueryHistoricalInfoResponse,
    };
    pub use ibc_proto::cosmos::staking::v1beta1::{QueryParamsRequest, QueryParamsResponse};
    pub use ibc_proto::cosmos::staking::v1beta1::{QueryPoolRequest, QueryPoolResponse};
    pub use ibc_proto::cosmos::staking::v1beta1::{
        QueryRedelegationsRequest, QueryRedelegationsResponse,
    };
    pub use ibc_proto::cosmos::staking::v1beta1::{
        QueryUnbondingDelegationRequest, QueryUnbondingDelegationResponse,
    };
    pub use ibc_proto::cosmos::staking::v1beta1::{
        QueryValidatorDelegationsRequest, QueryValidatorDelegationsResponse,
    };
    pub use ibc_proto::cosmos::staking::v1beta1::{QueryValidatorRequest, QueryValidatorResponse};
    pub use ibc_proto::cosmos::staking::v1beta1::{
        QueryValidatorUnbondingDelegationsRequest, QueryValidatorUnbondingDelegationsResponse,
    };
    pub use ibc_proto::cosmos::staking::v1beta1::{
        QueryValidatorsRequest, QueryValidatorsResponse,
    };
}

// ===
// requests
// ===

/// QueryValidatorRequest is the request type for the Query/Validator RPC method.
#[derive(Clone, Debug, PartialEq, Query, Protobuf)]
#[query(url = "/cosmos.staking.v1beta1.Query/Validator")]
#[proto(raw = "inner::QueryValidatorRequest")]
pub struct QueryValidatorRequest {
    /// Address of queried validator.
    pub validator_addr: ValAddress,
}

/// QueryValidatorsRequest is request type for Query/Validators RPC method.
#[derive(Clone, Debug, PartialEq, Query, Protobuf)]
#[query(url = "/cosmos.staking.v1beta1.Query/Validators")]
#[proto(raw = "inner::QueryValidatorsRequest")]
pub struct QueryValidatorsRequest {
    /// status enables to query for validators matching a given status.
    pub status: BondStatus,
    /// pagination defines an optional pagination for the request.
    #[proto(optional)]
    pub pagination: Option<PaginationRequest>,
}

/// QueryValidatorDelegationsRequest is request type for the Query/ValidatorDelegations RPC method
#[derive(Clone, Debug, PartialEq, Query, Protobuf)]
#[query(url = "/cosmos.staking.v1beta1.Query/ValidatorDelegations")]
#[proto(raw = "inner::QueryValidatorDelegationsRequest")]
pub struct QueryValidatorDelegationsRequest {
    /// validator_addr defines the validator address to query for.
    pub validator_addr: ValAddress,
    /// pagination defines an optional pagination for the request.
    #[proto(optional)]
    pub pagination: Option<PaginationRequest>,
}

/// QueryValidatorUnbondingDelegationsRequest is required type for the Query/ValidatorUnbondingDelegations RPC method
#[derive(Clone, Debug, PartialEq, Query, Protobuf)]
#[query(url = "/cosmos.staking.v1beta1.Query/ValidatorUnbondingDelegations")]
#[proto(raw = "inner::QueryValidatorUnbondingDelegationsRequest")]
pub struct QueryValidatorUnbondingDelegationsRequest {
    /// validator_addr defines the validator address to query for.
    pub validator_addr: ValAddress,
    /// pagination defines an optional pagination for the request.
    #[proto(optional)]
    pub pagination: Option<PaginationRequest>,
}

/// QueryDelegationRequest is request type for the Query/Delegation RPC method.
#[derive(Clone, Debug, PartialEq, Query, Protobuf)]
#[query(url = "/cosmos.staking.v1beta1.Query/Delegation")]
#[proto(raw = "inner::QueryDelegationRequest")]
pub struct QueryDelegationRequest {
    /// delegator_addr defines the delegator address to query for.
    pub delegator_addr: AccAddress,
    /// validator_addr defines the validator address to query for.
    pub validator_addr: ValAddress,
}

/// QueryDelegatorDelegationsRequest is request type for the
/// Query/DelegatorDelegations RPC method.
#[derive(Clone, Debug, PartialEq, Query, Protobuf)]
#[query(url = "/cosmos.staking.v1beta1.Query/DelegatorDelegations")]
#[proto(raw = "inner::QueryDelegatorDelegationsRequest")]
pub struct QueryDelegatorDelegationsRequest {
    /// delegator_addr defines the delegator address to query for.
    pub delegator_addr: AccAddress,
    /// pagination defines an optional pagination for the request.
    #[proto(optional)]
    pub pagination: Option<PaginationRequest>,
}

/// QueryUnbondingDelegationRequest is request type for the
/// Query/UnbondingDelegation RPC method.
#[derive(Clone, Debug, PartialEq, Query, Protobuf)]
#[query(url = "/cosmos.staking.v1beta1.Query/UnbondingDelegation")]
#[proto(raw = "inner::QueryUnbondingDelegationRequest")]
pub struct QueryUnbondingDelegationRequest {
    /// delegator_addr defines the delegator address to query for.
    pub delegator_addr: AccAddress,
    /// validator_addr defines the validator address to query for.
    pub validator_addr: ValAddress,
}

/// QueryDelegatorUnbondingDelegationsRequest is request type for the
/// Query/DelegatorUnbondingDelegations RPC method.
#[derive(Clone, Debug, PartialEq, Query, Protobuf)]
#[query(url = "/cosmos.staking.v1beta1.Query/DelegatorUnbondingDelegations")]
#[proto(raw = "inner::QueryDelegatorUnbondingDelegationsRequest")]
pub struct QueryDelegatorUnbondingDelegationsRequest {
    /// delegator_addr defines the delegator address to query for.
    pub delegator_addr: AccAddress,
    /// pagination defines an optional pagination for the request.
    #[proto(optional)]
    pub pagination: Option<PaginationRequest>,
}

/// QueryDelegatorValidatorRequest is request type for the Query/DelegatorValidator RPC method.
#[derive(Clone, Debug, PartialEq, Query, Protobuf)]
#[query(url = "/cosmos.staking.v1beta1.Query/DelegatorUnbondingDelegations")]
#[proto(raw = "inner::QueryDelegatorValidatorRequest")]
pub struct QueryDelegatorValidatorRequest {
    /// delegator_addr defines the delegator address to query for.
    pub delegator_addr: AccAddress,
    /// validator_addr defines the validator address to query for.
    pub validator_addr: ValAddress,
}

/// QueryRedelegationRequest is request type for the Query/Redelegation RPC method.
#[derive(Clone, Debug, PartialEq, Query, Raw, Protobuf)]
#[query(url = "/cosmos.staking.v1beta1.Query/Redelegations")]
pub struct QueryRedelegationsRequest {
    /// delegator_addr defines the delegator address to query for.
    #[raw(kind(string), raw = String, optional)]
    #[proto(optional)]
    pub delegator_address: Option<AccAddress>,
    /// src_validator_addr defines the validator address to redelegate from.
    #[raw(kind(string), raw = String, optional)]
    #[proto(optional)]
    pub src_validator_address: Option<ValAddress>,
    /// dst_validator_addr defines the validator address to redelegate to.
    #[raw(kind(string), raw = String, optional)]
    #[proto(optional)]
    pub dst_validator_address: Option<ValAddress>,
    /// pagination defines an optional pagination for the request.
    #[raw(kind(message), raw = PageRequest, optional)]
    #[proto(optional)]
    pub pagination: Option<PaginationRequest>,
}

impl TryFrom<inner::QueryRedelegationsRequest> for QueryRedelegationsRequest {
    type Error = AddressError;

    fn try_from(
        inner::QueryRedelegationsRequest {
            delegator_addr,
            src_validator_addr,
            dst_validator_addr,
            pagination,
        }: inner::QueryRedelegationsRequest,
    ) -> Result<Self, Self::Error> {
        let delegator_address = if delegator_addr.is_empty() {
            None
        } else {
            Some(delegator_addr.try_into()?)
        };
        let src_validator_address = if src_validator_addr.is_empty() {
            None
        } else {
            Some(src_validator_addr.try_into()?)
        };
        let dst_validator_address = if dst_validator_addr.is_empty() {
            None
        } else {
            Some(dst_validator_addr.try_into()?)
        };
        Ok(Self {
            delegator_address,
            src_validator_address,
            dst_validator_address,
            pagination: pagination.map(Into::into),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Query, Protobuf)]
#[query(url = "/cosmos.staking.v1beta1.Query/DelegatorValidators")]
#[proto(raw = "inner::QueryDelegatorValidatorsRequest")]
pub struct QueryDelegatorValidatorsRequest {
    pub delegator_addr: AccAddress,
    #[proto(optional)]
    pub pagination: Option<PaginationRequest>,
}

#[derive(Clone, Debug, PartialEq, Query, Protobuf)]
#[query(url = "/cosmos.staking.v1beta1.Query/HistoricalInfo")]
#[proto(raw = "inner::QueryHistoricalInfoRequest")]
pub struct QueryHistoricalInfoRequest {
    pub height: i64,
}

#[derive(Clone, Debug, PartialEq, Query, Protobuf)]
#[query(url = "/cosmos.staking.v1beta1.Query/Pool")]
#[proto(raw = "inner::QueryPoolRequest")]
pub struct QueryPoolRequest {}

#[derive(Clone, Debug, PartialEq, Query, Protobuf)]
#[query(url = "/cosmos.staking.v1beta1.Query/Params")]
#[proto(raw = "inner::QueryParamsRequest")]
pub struct QueryParamsRequest {}

// ===
// responses
// ===

/// QueryValidatorResponse is the response type for the Query/Validator RPC method.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Query, Protobuf)]
#[proto(raw = "inner::QueryValidatorResponse")]
pub struct QueryValidatorResponse {
    /// Full data about validator.
    #[proto(optional)]
    pub validator: Option<IbcV046Validator>,
}

/// QueryValidatorsResponse is response type for the Query/Validators RPC method
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Query, Protobuf)]
#[proto(raw = "inner::QueryValidatorsResponse")]
pub struct QueryValidatorsResponse {
    /// validators contains all the queried validators.
    #[proto(repeated)]
    pub validators: Vec<IbcV046Validator>,
    /// pagination defines the pagination in the response.
    #[proto(optional)]
    pub pagination: Option<PaginationResponse>,
}

/// QueryValidatorDelegationsResponse is response type for the Query/ValidatorDelegations RPC method
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Query, Protobuf)]
#[proto(raw = "inner::QueryValidatorDelegationsResponse")]
pub struct QueryValidatorDelegationsResponse {
    #[proto(repeated)]
    pub delegation_responses: Vec<DelegationResponse>,
    /// pagination defines the pagination in the response.
    #[proto(optional)]
    pub pagination: Option<PaginationResponse>,
}

/// QueryValidatorUnbondingDelegationsResponse is response type for the Query/ValidatorUnbondingDelegations RPC method.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Query, Protobuf)]
#[proto(raw = "inner::QueryValidatorUnbondingDelegationsResponse")]
pub struct QueryValidatorUnbondingDelegationsResponse {
    #[proto(repeated)]
    pub unbonding_responses: Vec<UnbondingDelegation>,
    /// pagination defines the pagination in the response.
    #[proto(optional)]
    pub pagination: Option<PaginationResponse>,
}

/// DelegationResponse is equivalent to Delegation except that it contains a
/// balance in addition to shares which is more suitable for client responses.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Query, Protobuf)]
#[proto(raw = "inner::DelegationResponse")]
pub struct DelegationResponse {
    #[proto(optional)]
    pub delegation: Option<Delegation>,
    #[proto(optional)]
    pub balance: Option<UnsignedCoin>,
}

impl PaginationKey for DelegationResponse {
    fn iterator_key(&self) -> std::borrow::Cow<'_, [u8]> {
        std::borrow::Cow::Owned(
            self.delegation
                .clone()
                .map(|d| Vec::from(d.delegator_address))
                .unwrap_or_default(),
        )
    }
}

/// QueryDelegationResponse is the response type for the Query/Delegation RPC method.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Query, Protobuf)]
#[proto(raw = "inner::QueryDelegationResponse")]
pub struct QueryDelegationResponse {
    /// Delegation with balance.
    #[proto(optional)]
    pub delegation_response: Option<DelegationResponse>,
}

/// QueryDelegatorDelegationsResponse is response type for the
/// Query/DelegatorDelegations RPC method.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Query, Protobuf)]
#[proto(raw = "inner::QueryDelegatorDelegationsResponse")]
pub struct QueryDelegatorDelegationsResponse {
    /// delegation_responses defines all the delegations' info of a delegator.
    #[proto(repeated)]
    pub delegation_responses: Vec<DelegationResponse>,
    /// pagination defines the pagination in the response.
    #[proto(optional)]
    pub pagination: Option<PaginationResponse>,
}

/// QueryUnbondingDelegationResponse is the response type for the Query/UnbondingDelegation RPC method.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Query, Protobuf)]
#[proto(raw = "inner::QueryUnbondingDelegationResponse")]
pub struct QueryUnbondingDelegationResponse {
    /// UnbondingDelegation with balance.
    #[proto(optional)]
    pub unbond: Option<UnbondingDelegation>,
}

/// QueryUnbondingDelegatorDelegationsResponse is response type for the
/// Query/UnbondingDelegatorDelegations RPC method.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Query, Protobuf)]
#[proto(raw = "inner::QueryDelegatorUnbondingDelegationsResponse")]
pub struct QueryDelegatorUnbondingDelegationsResponse {
    #[proto(repeated)]
    pub unbonding_responses: Vec<UnbondingDelegation>,
    /// pagination defines the pagination in the response.
    #[proto(optional)]
    pub pagination: Option<PaginationResponse>,
}

/// QueryDelegatorValidatorResponse response type for the Query/DelegatorValidator RPC method.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Query, Protobuf)]
#[proto(raw = "inner::QueryDelegatorValidatorResponse")]
pub struct QueryDelegatorValidatorResponse {
    /// validator defines the validator info.
    #[proto(optional)]
    pub validator: Option<Validator>,
}

/// RedelegationEntryResponse is equivalent to a RedelegationEntry except that it
/// contains a balance in addition to shares which is more suitable for client
/// responses.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Query)]
pub struct RedelegationEntryResponse {
    pub redelegation_entry: RedelegationEntry,
    pub balance: Uint256,
}

impl TryFrom<inner::RedelegationEntryResponse> for RedelegationEntryResponse {
    type Error = CoreError;

    fn try_from(raw: inner::RedelegationEntryResponse) -> Result<Self, Self::Error> {
        let redelegation_entry: RedelegationEntry = raw
            .redelegation_entry
            .ok_or(CoreError::MissingField(String::from("sum")))?
            .try_into()
            .map_err(|e| CoreError::DecodeGeneral(format!("{e}")))?;
        let balance = raw
            .balance
            .as_str()
            .try_into()
            .map_err(|e| CoreError::DecodeProtobuf(format!("{e}")))?;

        Ok(RedelegationEntryResponse {
            redelegation_entry,
            balance,
        })
    }
}

impl From<RedelegationEntryResponse> for inner::RedelegationEntryResponse {
    fn from(query: RedelegationEntryResponse) -> inner::RedelegationEntryResponse {
        Self {
            redelegation_entry: Some(query.redelegation_entry.into()),
            balance: query.balance.to_string(),
        }
    }
}

impl Protobuf<inner::RedelegationEntryResponse> for RedelegationEntryResponse {}

/// RedelegationResponse is equivalent to a Redelegation except that its entries
/// contain a balance in addition to shares which is more suitable for client responses.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Query, Protobuf)]
#[proto(raw = "inner::RedelegationResponse")]
pub struct RedelegationResponse {
    #[proto(optional)]
    pub redelegation: Redelegation,
    #[proto(repeated)]
    pub entries: Vec<RedelegationEntryResponse>,
}

/// QueryRedelegationResponse is the response type for the Query/Redelegation RPC method.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Query, Protobuf)]
#[proto(raw = "inner::QueryRedelegationsResponse")]
pub struct QueryRedelegationsResponse {
    /// Redelegation with balance
    #[proto(repeated)]
    pub redelegation_responses: Vec<RedelegationResponse>,
    #[proto(optional)]
    pub pagination: Option<PaginationResponse>,
}

/// QueryDelegatorValidatorsResponse is the response type for the Query/DelegatorValidators RPC method.
#[derive(Clone, Default, Debug, PartialEq, Deserialize, Serialize, Query, Protobuf)]
#[proto(raw = "inner::QueryDelegatorValidatorsResponse")]
pub struct QueryDelegatorValidatorsResponse {
    #[proto(repeated)]
    pub validators: Vec<Validator>,
    #[proto(optional)]
    pub pagination: Option<PaginationResponse>,
}

/// QueryHistoricalInfoResponse is response type for the Query/HistoricalInfo RPC method.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Query, Protobuf)]
#[proto(raw = "inner::QueryHistoricalInfoResponse")]
pub struct QueryHistoricalInfoResponse {
    #[proto(optional)]
    pub hist: Option<HistoricalInfo>,
}

/// QueryPoolResponse is response type for the Query/Pool RPC method.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Query, Protobuf)]
#[proto(raw = "inner::QueryPoolResponse")]
pub struct QueryPoolResponse {
    /// UnbondingDelegation with balance.
    #[proto(optional)]
    pub pool: Option<Pool>,
}

/// QueryParamsResponse is the response type for the Query/Params RPC method.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Query, Protobuf)]
#[proto(raw = "inner::QueryParamsResponse")]
pub struct QueryParamsResponse {
    #[proto(optional)]
    pub params: Option<StakingParams>,
}
