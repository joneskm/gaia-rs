use gears::{
    core::{errors::CoreError, query::request::PageRequest, Protobuf},
    types::{
        address::{AddressError, ConsAddress},
        pagination::{request::PaginationRequest, response::PaginationResponse},
    },
};
use prost::Message;
use serde::{Deserialize, Serialize};

use crate::{SlashingParams, SlashingParamsRaw, ValidatorSigningInfo, ValidatorSigningInfoRaw};

// =====
// Requests
// =====

#[derive(Clone, PartialEq, Serialize, Deserialize, Message)]
pub struct QuerySigningInfoRequestRaw {
    #[prost(bytes)]
    pub cons_address: Vec<u8>,
}

impl From<QuerySigningInfoRequest> for QuerySigningInfoRequestRaw {
    fn from(value: QuerySigningInfoRequest) -> Self {
        Self {
            cons_address: value.cons_address.into(),
        }
    }
}

/// QuerySigningInfoRequest is the request type for the Query/SigningInfo RPC
/// method
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuerySigningInfoRequest {
    /// cons_address is the address to query signing info of
    pub cons_address: ConsAddress,
}

impl TryFrom<QuerySigningInfoRequestRaw> for QuerySigningInfoRequest {
    type Error = AddressError;

    fn try_from(value: QuerySigningInfoRequestRaw) -> Result<Self, Self::Error> {
        Ok(QuerySigningInfoRequest {
            cons_address: ConsAddress::try_from(value.cons_address)?,
        })
    }
}

impl Protobuf<QuerySigningInfoRequestRaw> for QuerySigningInfoRequest {}

#[derive(Clone, PartialEq, Message)]
pub struct QuerySigningInfosRequestRaw {
    /// pagination defines an optional pagination for the request.
    #[prost(message, optional)]
    pub pagination: Option<PageRequest>,
}

impl From<QuerySigningInfosRequest> for QuerySigningInfosRequestRaw {
    fn from(QuerySigningInfosRequest { pagination }: QuerySigningInfosRequest) -> Self {
        Self {
            pagination: Some(pagination.into()),
        }
    }
}

/// QuerySigningInfosRequest is the request type for the Query/SigningInfos RPC
/// method
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuerySigningInfosRequest {
    /// pagination defines an optional pagination for the request.
    pub pagination: PaginationRequest,
}

impl TryFrom<QuerySigningInfosRequestRaw> for QuerySigningInfosRequest {
    type Error = CoreError;

    fn try_from(
        QuerySigningInfosRequestRaw { pagination }: QuerySigningInfosRequestRaw,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            pagination: pagination
                .ok_or(CoreError::MissingField(
                    "Missing field 'pagination'.".into(),
                ))?
                .into(),
        })
    }
}

impl Protobuf<QuerySigningInfosRequestRaw> for QuerySigningInfosRequest {}

#[derive(Clone, PartialEq, Message)]
pub struct QueryParamsRequest {}
impl Protobuf<QueryParamsRequest> for QueryParamsRequest {}

// =====
// Responses
// =====

#[derive(Clone, PartialEq, Serialize, Deserialize, Message)]
pub struct QuerySigningInfoResponseRaw {
    #[prost(message, optional)]
    pub val_signing_info: Option<ValidatorSigningInfoRaw>,
}

impl From<QuerySigningInfoResponse> for QuerySigningInfoResponseRaw {
    fn from(value: QuerySigningInfoResponse) -> Self {
        Self {
            val_signing_info: Some(value.val_signing_info.into()),
        }
    }
}

/// QuerySigningInfoResponse is the response type for the Query/SigningInfo RPC
/// method
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuerySigningInfoResponse {
    /// val_signing_info is the signing info of requested val cons address
    pub val_signing_info: ValidatorSigningInfo,
}

impl TryFrom<QuerySigningInfoResponseRaw> for QuerySigningInfoResponse {
    type Error = CoreError;

    fn try_from(value: QuerySigningInfoResponseRaw) -> Result<Self, Self::Error> {
        Ok(QuerySigningInfoResponse {
            val_signing_info: value
                .val_signing_info
                .ok_or(CoreError::MissingField(
                    "Missing field 'val_signing_info'.".into(),
                ))?
                .try_into()?,
        })
    }
}

impl Protobuf<QuerySigningInfoResponseRaw> for QuerySigningInfoResponse {}

#[derive(Clone, PartialEq, Message)]
pub struct QuerySigningInfosResponseRaw {
    #[prost(message, repeated)]
    pub info: Vec<ValidatorSigningInfoRaw>,
    #[prost(message, optional)]
    pub pagination: Option<gears::core::query::response::PageResponse>,
}

impl From<QuerySigningInfosResponse> for QuerySigningInfosResponseRaw {
    fn from(QuerySigningInfosResponse { info, pagination }: QuerySigningInfosResponse) -> Self {
        Self {
            info: info.into_iter().map(|inf| inf.into()).collect(),
            pagination: pagination.map(|p| p.into()),
        }
    }
}

/// QuerySigningInfosResponse is the response type for the Query/SigningInfos RPC
/// method
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct QuerySigningInfosResponse {
    /// Info is the signing info of all validators
    pub info: Vec<ValidatorSigningInfo>,
    pub pagination: Option<PaginationResponse>,
}

impl TryFrom<QuerySigningInfosResponseRaw> for QuerySigningInfosResponse {
    type Error = CoreError;
    fn try_from(
        QuerySigningInfosResponseRaw { info, pagination }: QuerySigningInfosResponseRaw,
    ) -> Result<Self, Self::Error> {
        let mut info_res = vec![];
        for inf in info {
            info_res.push(inf.try_into()?);
        }
        Ok(Self {
            info: info_res,
            pagination: pagination.map(|p| p.into()),
        })
    }
}

impl Protobuf<QuerySigningInfosResponseRaw> for QuerySigningInfosResponse {}

/// QueryParamsResponse is the response type for the Query/Params RPC method
#[derive(Clone, Serialize, Message)]
pub struct QueryParamsResponseRaw {
    #[prost(message, optional)]
    pub params: Option<SlashingParamsRaw>,
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
    pub params: SlashingParams,
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
