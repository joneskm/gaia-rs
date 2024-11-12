pub mod begin_block;
pub mod check_tx;
pub mod deliver_tx;
pub mod echo;
pub mod end_block;
pub mod info;
pub mod init_chain;
pub mod query;
pub mod snapshot;

pub(crate) mod inner {
    pub use tendermint_informal::abci::response::DeliverTx;
    pub use tendermint_proto::abci::ResponseApplySnapshotChunk;
    pub use tendermint_proto::abci::ResponseBeginBlock;
    pub use tendermint_proto::abci::ResponseCheckTx;
    pub use tendermint_proto::abci::ResponseCommit;
    pub use tendermint_proto::abci::ResponseDeliverTx;
    pub use tendermint_proto::abci::ResponseEcho;
    pub use tendermint_proto::abci::ResponseEndBlock;
    pub use tendermint_proto::abci::ResponseFlush;
    pub use tendermint_proto::abci::ResponseInfo;
    pub use tendermint_proto::abci::ResponseInitChain;
    pub use tendermint_proto::abci::ResponseListSnapshots;
    pub use tendermint_proto::abci::ResponseLoadSnapshotChunk;
    pub use tendermint_proto::abci::ResponseOfferSnapshot;
    pub use tendermint_proto::abci::ResponseQuery;
}

#[derive(Clone, PartialEq, Eq, ::prost::Message, serde::Serialize, serde::Deserialize)]
pub struct ResponseFlush {}

impl From<inner::ResponseFlush> for ResponseFlush {
    fn from(_: inner::ResponseFlush) -> Self {
        Self {}
    }
}

impl From<ResponseFlush> for inner::ResponseFlush {
    fn from(_: ResponseFlush) -> Self {
        Self {}
    }
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResponseCommit {
    /// reserve 1
    pub data: ::prost::bytes::Bytes,
    pub retain_height: u32,
}

impl From<ResponseCommit> for inner::ResponseCommit {
    fn from(
        ResponseCommit {
            data,
            retain_height,
        }: ResponseCommit,
    ) -> Self {
        Self {
            data,
            retain_height: retain_height.into(),
        }
    }
}
