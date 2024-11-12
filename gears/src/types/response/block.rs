use serde::{Deserialize, Serialize};
use tendermint::{informal::Block, types::proto::block::BlockId};

/// GetBlockByHeightResponse is the response type for the Query/GetBlockByHeight RPC method.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct GetBlockByHeightResponse {
    pub block_id: Option<BlockId>,
    /// Deprecated: please use `sdk_block` instead
    pub block: Option<Block>,
    /// Since: cosmos-sdk 0.47
    pub sdk_block: Option<Block>,
}
