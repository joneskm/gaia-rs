use crate::{
    baseapp::Query,
    core::errors::CoreError,
    core::Protobuf,
    types::{
        response::{
            tx::{TxResponse, TxResponseRaw},
            tx_event::{SearchTxsResult, SearchTxsResultRaw},
        },
        tx::TxMessage,
    },
};
use protobuf_derive::Raw;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tendermint::informal::Hash;

mod inner {
    pub use ibc_proto::cosmos::tx::v1beta1::GetTxRequest;
}

/// QueryTxResponse is the response type for tendermint rpc query
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Raw)]
pub struct QueryTxResponse<M: TxMessage> {
    #[raw(kind(message), raw = TxResponseRaw, optional)]
    pub tx: TxResponse<M>,
}

impl<M: TxMessage> From<QueryTxResponse<M>> for RawQueryTxResponse {
    fn from(QueryTxResponse { tx }: QueryTxResponse<M>) -> Self {
        Self {
            tx: Some(tx.into()),
        }
    }
}

impl<M: TxMessage> TryFrom<RawQueryTxResponse> for QueryTxResponse<M> {
    type Error = CoreError;

    fn try_from(RawQueryTxResponse { tx }: RawQueryTxResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            tx: tx
                .ok_or(CoreError::MissingField("Missed field 'tx'".to_string()))?
                .try_into()
                .map_err(|e| CoreError::DecodeGeneral(format!("{e}")))?,
        })
    }
}

impl<M: TxMessage> Protobuf<RawQueryTxResponse> for QueryTxResponse<M> {}

/// QueryTxsResponse is the response type for tendermint rpc query
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Raw)]
pub struct QueryTxsResponse<M: TxMessage> {
    #[raw(kind(message), raw = SearchTxsResultRaw, optional)]
    pub txs: SearchTxsResult<M>,
}

impl<M: TxMessage> From<QueryTxsResponse<M>> for RawQueryTxsResponse {
    fn from(QueryTxsResponse { txs }: QueryTxsResponse<M>) -> Self {
        Self {
            txs: Some(txs.into()),
        }
    }
}

impl<M: TxMessage> TryFrom<RawQueryTxsResponse> for QueryTxsResponse<M> {
    type Error = CoreError;

    fn try_from(RawQueryTxsResponse { txs }: RawQueryTxsResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            txs: txs
                .ok_or(CoreError::MissingField("Field 'txs' is missed".to_string()))?
                .try_into()?,
        })
    }
}

impl<M: TxMessage> Protobuf<RawQueryTxsResponse> for QueryTxsResponse<M> {}

#[derive(Clone, Debug, PartialEq)]
pub struct QueryGetTxRequest {
    pub hash: Hash,
}

impl Query for QueryGetTxRequest {
    fn query_url(&self) -> &'static str {
        "/cosmos.auth.v1beta1.Query/QueryGetTx"
    }

    fn into_bytes(self) -> Vec<u8> {
        self.encode_vec()
    }
}

impl From<QueryGetTxRequest> for inner::GetTxRequest {
    fn from(QueryGetTxRequest { hash }: QueryGetTxRequest) -> Self {
        inner::GetTxRequest {
            hash: hash.to_string(),
        }
    }
}

impl TryFrom<inner::GetTxRequest> for QueryGetTxRequest {
    type Error = CoreError;

    fn try_from(inner::GetTxRequest { hash }: inner::GetTxRequest) -> Result<Self, Self::Error> {
        Ok(QueryGetTxRequest {
            hash: Hash::from_str(&hash).map_err(|e| CoreError::DecodeGeneral(format!("{e}")))?,
        })
    }
}

impl Protobuf<inner::GetTxRequest> for QueryGetTxRequest {}

#[derive(Clone, Debug, PartialEq, Raw, protobuf_derive::Protobuf)]
#[proto(gears)]
pub struct QueryGetTxsEventRequest {
    #[raw(raw = String, kind(string), repeated)]
    #[proto(repeated)]
    pub events: Vec<String>,
    #[raw(raw = String, kind(string))]
    pub order_by: String,
    #[raw(raw = u32, kind(uint32))]
    pub page: u32,
    #[raw(raw = u32, kind(uint32))]
    pub limit: u32,
}

impl Query for QueryGetTxsEventRequest {
    fn query_url(&self) -> &'static str {
        "/cosmos.auth.v1beta1.Query/QueryGetTxsEvent"
    }

    fn into_bytes(self) -> Vec<u8> {
        self.encode_vec()
    }
}
