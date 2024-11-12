use std::fmt::Display;

use crate::application::handlers::client::QueryHandler;
use crate::runtime::runtime;
use anyhow::anyhow;
use prost::Message;
use tendermint::{
    rpc::client::{Client, HttpClient},
    types::proto::block::Height,
};

#[derive(Debug, Clone, former::Former)]
pub struct QueryCommand<C> {
    pub node: url::Url,
    pub height: Option<Height>,

    pub inner: C,
}

/// Execute query
pub fn run_query<Q, QC, QR, H>(
    QueryCommand {
        node,
        height,
        inner,
    }: QueryCommand<QC>,
    handler: &H,
) -> anyhow::Result<QR>
where
    H: QueryHandler<QueryRequest = Q, QueryCommands = QC, QueryResponse = QR>,
{
    let query = handler.prepare_query_request(&inner)?;
    let query_bytes = handler.execute_query_request(query, node, height)?;

    let response = handler.handle_raw_response(query_bytes, &inner)?;

    Ok(response)
}

/// Convenience method for running queries.
/// After execution `Raw` will be serialized as protobuf
/// message and parsed to your `Response`.
///
/// This allows to omit [Default] bound on query structure and
/// single way to do a validation with prost crate
pub fn execute_query<
    Response: std::convert::TryFrom<Raw>,
    Raw: Message + Default + std::convert::From<Response>,
>(
    path: String,
    query_bytes: Vec<u8>,
    node: &str,
    height: Option<Height>,
) -> anyhow::Result<Response>
where
    <Response as TryFrom<Raw>>::Error: Display,
{
    let client = HttpClient::new(node)?;

    let res = runtime().block_on(client.abci_query(Some(path), query_bytes, height, false))?;

    if res.code.is_err() {
        return Err(anyhow!("node returned an error: {}", res.log));
    }

    Response::try_from(Raw::decode(&*res.value)?).map_err(|e| anyhow!(e.to_string()))
}
