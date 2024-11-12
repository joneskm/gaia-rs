use core_types::errors::CoreError;
use core_types::Protobuf;
use prost::Message;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;

use super::tx::TxResponse;
use super::tx::TxResponseRaw;
use crate::types::pagination::response::PaginationResponse;
use crate::types::tx::Tx;
use crate::types::tx::TxMessage;

/// GetTxsEventResponse is the response type for the Service.TxsByEvents
/// RPC method.
#[serde_as]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GetTxsEventResponse<M: TxMessage> {
    /// txs is the list of queried transactions.
    pub txs: Vec<Tx<M>>,
    /// tx_responses is the list of queried TxResponses.
    pub tx_responses: Vec<TxResponse<M>>,
    /// pagination defines a pagination for the response.
    /// Deprecated post v0.46.x: use total instead.
    // TODO: doesn't serialize correctly - has been deprecated
    pub pagination: Option<PaginationResponse>,
    /// total is total number of results available
    #[serde_as(as = "DisplayFromStr")]
    pub total: u64,
}

/// SearchTxsResult defines a structure for querying txs pageable
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchTxsResult<M: TxMessage> {
    /// Count of all txs
    pub total_count: u64,
    /// Count of txs in current page
    pub count: u64,
    /// Index of current page, start from 1
    pub page_number: u64,
    /// Count of total pages
    pub page_total: u64,
    /// Max count txs per page
    pub limit: u64,
    /// List of txs in current page
    pub txs: Vec<TxResponse<M>>,
}

impl<M: TxMessage> TryFrom<SearchTxsResultRaw> for SearchTxsResult<M> {
    type Error = CoreError;

    fn try_from(
        SearchTxsResultRaw {
            total_count,
            count,
            page_number,
            page_total,
            limit,
            txs,
        }: SearchTxsResultRaw,
    ) -> Result<Self, Self::Error> {
        let mut txs_res = Vec::with_capacity(txs.len());
        for tx in txs {
            txs_res.push(tx.try_into()?);
        }
        Ok(Self {
            total_count,
            count,
            page_number,
            page_total,
            limit,
            txs: txs_res,
        })
    }
}

impl<M: TxMessage> Protobuf<SearchTxsResultRaw> for SearchTxsResult<M> {}

// TODO: may be replaced with ibc_proto struct but has some type versions conflicts in structure
// TxResponse
#[derive(Clone, PartialEq, Deserialize, Serialize, Message)]
pub struct SearchTxsResultRaw {
    /// Count of all txs
    #[prost(uint64, tag = "1")]
    pub total_count: u64,
    /// Count of txs in current page
    #[prost(uint64, tag = "2")]
    pub count: u64,
    /// Index of current page, start from 1
    #[prost(uint64, tag = "3")]
    pub page_number: u64,
    /// Count of total pages
    #[prost(uint64, tag = "4")]
    pub page_total: u64,
    /// Max count txs per page
    #[prost(uint64, tag = "5")]
    pub limit: u64,
    /// List of txs in current page
    #[prost(message, repeated, tag = "6")]
    pub txs: Vec<TxResponseRaw>,
}

impl<M: TxMessage> From<SearchTxsResult<M>> for SearchTxsResultRaw {
    fn from(
        SearchTxsResult {
            total_count,
            count,
            page_number,
            page_total,
            limit,
            txs,
        }: SearchTxsResult<M>,
    ) -> Self {
        Self {
            total_count,
            count,
            page_number,
            page_total,
            limit,
            txs: txs.into_iter().map(Into::into).collect(),
        }
    }
}
