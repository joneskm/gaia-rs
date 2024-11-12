use crate::{
    application::handlers::client::QueryHandler,
    baseapp::{Query, QueryResponse},
    cli::query_txs::{TxQueryCli, TxQueryType, TxsQueryCli},
    core::{errors::CoreError, Protobuf},
    rest::tendermint_events_handler::StrEventsHandler,
    types::{
        response::{tx::TxResponse, tx_event::SearchTxsResult},
        tx::TxMessage,
    },
    x::query::types::{
        QueryGetTxRequest, QueryGetTxsEventRequest, QueryTxResponse, QueryTxsResponse,
    },
};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, marker::PhantomData, str::FromStr};
use tendermint::{
    informal::Hash,
    rpc::{
        client::{Client, HttpClient},
        response::{
            block::Response as BlockResponse,
            tx::{search::Response as SearchResponse, Response as CosmosTxResponse},
        },
    },
    types::proto::block::Height,
};

#[derive(Clone, Debug, PartialEq)]
pub enum TxQuery {
    Tx(QueryGetTxRequest),
    Txs(QueryGetTxsEventRequest),
}

impl Query for TxQuery {
    fn query_url(&self) -> &'static str {
        match self {
            TxQuery::Tx(req) => req.query_url(),
            TxQuery::Txs(req) => req.query_url(),
        }
    }

    fn into_bytes(self) -> Vec<u8> {
        match self {
            TxQuery::Tx(req) => req.into_bytes(),
            TxQuery::Txs(req) => req.into_bytes(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum TxQueryResponse<M: TxMessage> {
    Tx(QueryTxResponse<M>),
    Txs(QueryTxsResponse<M>),
}

impl<M: TxMessage> QueryResponse for TxQueryResponse<M> {
    fn into_bytes(self) -> Vec<u8> {
        match self {
            TxQueryResponse::Tx(msg) => msg.encode_vec(),
            TxQueryResponse::Txs(msg) => msg.encode_vec(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TxQueryHandler<M: TxMessage> {
    msgs_type: PhantomData<M>,
}

impl<M: TxMessage> TxQueryHandler<M> {
    /// Constructor allows to handle tx messages received by query calls.
    #[allow(clippy::new_without_default)]
    pub fn new() -> TxQueryHandler<M> {
        TxQueryHandler {
            msgs_type: PhantomData,
        }
    }
}

impl<M: TxMessage> QueryHandler for TxQueryHandler<M> {
    type QueryRequest = TxQuery;

    type QueryCommands = TxQueryCli;

    type QueryResponse = TxQueryResponse<M>;

    fn prepare_query_request(
        &self,
        command: &Self::QueryCommands,
    ) -> anyhow::Result<Self::QueryRequest> {
        let TxQueryCli { hash, query_type } = command;
        let res = match query_type {
            TxQueryType::Hash => Self::QueryRequest::Tx(QueryGetTxRequest {
                hash: Hash::from_str(hash)?,
            }),
            TxQueryType::Signature => {
                let events = if hash.is_empty() {
                    return Err(anyhow!(
                        "Signatures list is empty. Please, provide at least one signature."
                    ));
                } else {
                    hash.split(',')
                        .map(|sig| format!("tx.signature='{sig}'"))
                        .collect()
                };
                Self::QueryRequest::Txs(QueryGetTxsEventRequest {
                    events,
                    order_by: "asc".to_string(),
                    // default page
                    // TODO: may be part of gears constants
                    page: 1,
                    // default limit
                    // TODO: may be part of gears constants
                    limit: 100,
                })
            }
            TxQueryType::AccSeq => {
                let events = if hash.is_empty() {
                    return Err(anyhow!(
                        "Account sequence is not set. Please, provide correct value."
                    ));
                } else {
                    vec![format!("tx.acc_seq='{hash}'")]
                };
                Self::QueryRequest::Txs(QueryGetTxsEventRequest {
                    events,
                    order_by: "asc".to_string(),
                    page: 1,
                    limit: 100,
                })
            }
        };
        Ok(res)
    }

    fn execute_query_request(
        &self,
        query: Self::QueryRequest,
        node: url::Url,
        _height: Option<tendermint::types::proto::block::Height>,
    ) -> anyhow::Result<Vec<u8>> {
        let client = HttpClient::new(node.as_str())?;
        let runtime = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");

        match query {
            Self::QueryRequest::Tx(req) => {
                let res: anyhow::Result<(CosmosTxResponse, BlockResponse)> =
                    runtime.block_on(async {
                        let tx_res = client.tx(req.hash, true).await?;
                        let block_res = client.block(tx_res.height).await?;
                        Ok((tx_res, block_res))
                    });
                let (tx_res, block_res) = res?;

                let res = make_tx_response::<M>(tx_res, block_res)?;
                Ok(res.encode_vec())
            }
            Self::QueryRequest::Txs(req) => query_txs_by_event::<M>(client, runtime, &req),
        }
    }

    fn handle_raw_response(
        &self,
        query_bytes: Vec<u8>,
        command: &Self::QueryCommands,
    ) -> anyhow::Result<Self::QueryResponse> {
        let TxQueryCli { query_type, .. } = command;
        match &query_type {
            TxQueryType::Hash => {
                let tx: TxResponse<M> = TxResponse::decode_vec(&query_bytes)?;
                Ok(Self::QueryResponse::Tx(QueryTxResponse { tx }))
            }
            _ => {
                let txs: SearchTxsResult<M> = SearchTxsResult::decode_vec(&query_bytes)?;

                if txs.txs.is_empty() {
                    return Err(anyhow!("found no txs matching given parameters"));
                }
                if txs.txs.len() > 1 {
                    return Err(anyhow!(
                        "found {} txs matching given parameters",
                        txs.txs.len()
                    ));
                }
                Ok(Self::QueryResponse::Txs(QueryTxsResponse { txs }))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct TxsQueryHandler<M: TxMessage> {
    msgs_type: PhantomData<M>,
}

impl<M: TxMessage> TxsQueryHandler<M> {
    /// Constructor allows to handle tx messages received by query calls.
    #[allow(clippy::new_without_default)]
    pub fn new() -> TxsQueryHandler<M> {
        TxsQueryHandler {
            msgs_type: PhantomData,
        }
    }
}

impl<M: TxMessage> QueryHandler for TxsQueryHandler<M> {
    type QueryRequest = QueryGetTxsEventRequest;

    type QueryCommands = TxsQueryCli;

    type QueryResponse = TxQueryResponse<M>;

    fn prepare_query_request(
        &self,
        command: &Self::QueryCommands,
    ) -> anyhow::Result<Self::QueryRequest> {
        let TxsQueryCli {
            events,
            page,
            limit,
        } = command;

        Ok(QueryGetTxsEventRequest {
            events: StrEventsHandler::new(events).try_parse_tendermint_events_vec()?,
            order_by: "asc".to_string(),
            page: *page,
            limit: *limit,
        })
    }

    fn execute_query_request(
        &self,
        query: Self::QueryRequest,
        node: url::Url,
        _height: Option<tendermint::types::proto::block::Height>,
    ) -> anyhow::Result<Vec<u8>> {
        let client = HttpClient::new(node.as_str())?;
        let runtime = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
        query_txs_by_event::<M>(client, runtime, &query)
    }

    fn handle_raw_response(
        &self,
        query_bytes: Vec<u8>,
        _command: &Self::QueryCommands,
    ) -> anyhow::Result<Self::QueryResponse> {
        let txs: SearchTxsResult<M> = SearchTxsResult::decode_vec(&query_bytes)?;

        if txs.txs.is_empty() {
            return Err(anyhow!("found no txs matching given parameters"));
        }
        Ok(Self::QueryResponse::Txs(QueryTxsResponse { txs }))
    }
}

fn query_txs_by_event<M: TxMessage>(
    client: HttpClient,
    runtime: tokio::runtime::Runtime,
    req: &QueryGetTxsEventRequest,
) -> anyhow::Result<Vec<u8>> {
    let query_str_events = req.events.join(" AND ");
    let query = tendermint::rpc::query::Query::from_str(&query_str_events)?;
    let res: anyhow::Result<(SearchResponse, HashMap<Height, BlockResponse>)> =
        runtime.block_on(async {
            let txs = client
                .tx_search(
                    query,
                    true,
                    req.page,
                    req.limit.try_into().unwrap_or(u8::MAX),
                    tendermint::rpc::Order::from_str(&req.order_by)?,
                )
                .await?;
            let mut blocks: HashMap<Height, BlockResponse> = HashMap::with_capacity(txs.txs.len());
            for tx in &txs.txs {
                blocks.insert(tx.height, client.block(tx.height).await?);
            }

            Ok((txs, blocks))
        });
    let (search_res, blocks_res) = res?;
    let mut tx_responses = vec![];
    for cosmos_tx in &search_res.txs {
        tx_responses.push(make_tx_response::<M>(
            cosmos_tx.clone(),
            blocks_res
                .get(&cosmos_tx.height)
                .expect("map contains only heights from txs")
                .clone(),
        )?);
    }

    let count = search_res
        .txs
        .len()
        .try_into()
        .map_err(|e| anyhow!("{e}"))?;

    let res = SearchTxsResult {
        total_count: search_res.total_count as u64,
        count,
        page_number: req.page as u64,
        // sdk uses conversion to f64
        page_total: (search_res.total_count as f64 / req.limit as f64).ceil() as u64,
        limit: req.limit as u64,
        txs: tx_responses,
    };
    Ok(res.encode_vec())
}

fn make_tx_response<M: TxMessage>(
    tx_response: CosmosTxResponse,
    block_response: BlockResponse,
) -> Result<TxResponse<M>, CoreError> {
    TxResponse::new_from_tx_response_and_string_time(
        tx_response,
        block_response.block.header.time.to_string(),
    )
}
