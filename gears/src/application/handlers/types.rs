use address::AccAddress;
use core_types::{errors::CoreError, Protobuf};
use serde::{Deserialize, Serialize};

use crate::types::{
    account::Account, denom::Denom, pagination::request::PaginationRequest, tx::metadata::Metadata,
};

mod inner {
    pub use core_types::bank::Metadata;
    pub use core_types::query::request::auth::QueryAccountRequest;
    pub use core_types::query::request::bank::QueryDenomMetadataRequest;
    pub use core_types::query::request::bank::QueryDenomsMetadataRequest;
    pub use core_types::query::response::auth::QueryAccountResponse;
}

#[derive(Clone, PartialEq, Debug)]
pub struct QueryDenomsMetadataRequest {
    pub pagination: Option<PaginationRequest>,
}

impl QueryDenomsMetadataRequest {
    pub const TYPE_URL: &'static str = "/cosmos.bank.v1beta1.Query/DenomsMetadata";
}

impl TryFrom<inner::QueryDenomsMetadataRequest> for QueryDenomsMetadataRequest {
    type Error = CoreError;

    fn try_from(
        inner::QueryDenomsMetadataRequest { pagination }: inner::QueryDenomsMetadataRequest,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            pagination: pagination.map(PaginationRequest::from),
        })
    }
}

impl From<QueryDenomsMetadataRequest> for inner::QueryDenomsMetadataRequest {
    fn from(QueryDenomsMetadataRequest { pagination }: QueryDenomsMetadataRequest) -> Self {
        Self {
            pagination: pagination.map(PaginationRequest::into),
        }
    }
}

impl Protobuf<inner::QueryDenomsMetadataRequest> for QueryDenomsMetadataRequest {}

#[derive(Clone, Debug, PartialEq)]
pub struct QueryDenomMetadataRequest {
    /// denom is the coin denom to query metadata for.
    pub denom: Denom,
}

impl TryFrom<inner::QueryDenomMetadataRequest> for QueryDenomMetadataRequest {
    type Error = core_types::errors::CoreError;

    fn try_from(raw: inner::QueryDenomMetadataRequest) -> Result<Self, Self::Error> {
        let denom = raw
            .denom
            .try_into()
            .map_err(|_| Self::Error::Coin(String::from("invalid denom")))?;

        Ok(QueryDenomMetadataRequest { denom })
    }
}

impl From<QueryDenomMetadataRequest> for inner::QueryDenomMetadataRequest {
    fn from(query: QueryDenomMetadataRequest) -> inner::QueryDenomMetadataRequest {
        Self {
            denom: query.denom.to_string(),
        }
    }
}

impl Protobuf<inner::QueryDenomMetadataRequest> for QueryDenomMetadataRequest {}

/// We use our own version of the QueryDenomMetadataResponse struct because the
/// Metadata struct in ibc_proto has additional fields that were added in SDK
/// v46 (uri and uri_hash).
#[derive(Clone, PartialEq, prost::Message)]
pub struct RawQueryDenomMetadataResponse {
    /// metadata describes and provides all the client information for the requested token.
    #[prost(message, optional, tag = "1")]
    pub metadata: Option<inner::Metadata>,
}

#[derive(Clone, Debug, Serialize)]
pub struct QueryDenomMetadataResponse {
    /// metadata describes and provides all the client information for the requested token.
    pub metadata: Option<Metadata>,
}

impl TryFrom<RawQueryDenomMetadataResponse> for QueryDenomMetadataResponse {
    type Error = core_types::errors::CoreError;

    fn try_from(raw: RawQueryDenomMetadataResponse) -> Result<Self, Self::Error> {
        let metadata = raw
            .metadata
            .map(Metadata::try_from)
            .transpose()
            .map_err(|_| core_types::errors::CoreError::Coin(String::from("invalid metadata")))?;

        Ok(QueryDenomMetadataResponse { metadata })
    }
}

impl From<QueryDenomMetadataResponse> for RawQueryDenomMetadataResponse {
    fn from(query: QueryDenomMetadataResponse) -> RawQueryDenomMetadataResponse {
        RawQueryDenomMetadataResponse {
            metadata: query.metadata.map(inner::Metadata::from),
        }
    }
}

impl Protobuf<RawQueryDenomMetadataResponse> for QueryDenomMetadataResponse {}

/// QueryAccountResponse is the response type for the Query/Account RPC method.
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct QueryAccountResponse {
    /// account defines the account of the corresponding address.
    pub account: Option<Account>,
}

impl TryFrom<inner::QueryAccountResponse> for QueryAccountResponse {
    type Error = core_types::errors::CoreError;

    fn try_from(raw: inner::QueryAccountResponse) -> Result<Self, Self::Error> {
        let account = raw.account.map(|a| a.try_into()).transpose()?;
        Ok(QueryAccountResponse { account })
    }
}

impl From<QueryAccountResponse> for inner::QueryAccountResponse {
    fn from(query: QueryAccountResponse) -> inner::QueryAccountResponse {
        Self {
            account: query.account.map(Into::into),
        }
    }
}

impl Protobuf<inner::QueryAccountResponse> for QueryAccountResponse {}

/// QueryAccountRequest is the request type for the Query/Account RPC method.
#[derive(Clone, PartialEq, Debug)]
pub struct QueryAccountRequest {
    /// address defines the address to query for.
    pub address: AccAddress,
}

impl TryFrom<inner::QueryAccountRequest> for QueryAccountRequest {
    type Error = core_types::errors::CoreError;

    fn try_from(raw: inner::QueryAccountRequest) -> Result<Self, Self::Error> {
        let address = AccAddress::from_bech32(&raw.address)
            .map_err(|e| Self::Error::DecodeAddress(e.to_string()))?;

        Ok(QueryAccountRequest { address })
    }
}

impl From<QueryAccountRequest> for inner::QueryAccountRequest {
    fn from(query: QueryAccountRequest) -> inner::QueryAccountRequest {
        Self {
            address: query.address.to_string(),
        }
    }
}

impl Protobuf<inner::QueryAccountRequest> for QueryAccountRequest {}
