// the alternatives are place below, for now we don't need conversions
// mod inner {
//     pub use tendermint_proto::p2p::DefaultNodeInfo;
//     pub use tendermint_proto::p2p::DefaultNodeInfoOther;
//     pub use tendermint_proto::p2p::ProtocolVersion;
// }

use crate::informal::node::{Info, OtherInfo, ProtocolVersionInfo};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct DefaultNodeInfo {
    pub protocol_version: Option<ProtocolVersion>,
    pub default_node_id: String,
    pub listen_addr: String,
    pub network: String,
    pub version: String,
    pub channels: Vec<u8>,
    pub moniker: String,
    pub other: Option<DefaultNodeInfoOther>,
}

impl From<Info> for DefaultNodeInfo {
    fn from(
        Info {
            protocol_version,
            id,
            listen_addr,
            network,
            version,
            channels,
            moniker,
            other,
        }: Info,
    ) -> Self {
        Self {
            protocol_version: Some(protocol_version.into()),
            default_node_id: id.to_string(),
            listen_addr: listen_addr.to_string(),
            network: network.to_string(),
            version: version.to_string(),
            channels: channels.to_string().into_bytes(),
            moniker: moniker.to_string(),
            other: Some(other.into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ProtocolVersion {
    pub p2p: u64,
    pub block: u64,
    pub app: u64,
}

impl From<ProtocolVersionInfo> for ProtocolVersion {
    fn from(ProtocolVersionInfo { p2p, block, app }: ProtocolVersionInfo) -> Self {
        Self { p2p, block, app }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct DefaultNodeInfoOther {
    // TODO: original is a String
    pub tx_index: bool,
    // pub tx_index: String,
    pub rpc_address: String,
}

impl From<OtherInfo> for DefaultNodeInfoOther {
    fn from(
        OtherInfo {
            tx_index,
            rpc_address,
        }: OtherInfo,
    ) -> Self {
        Self {
            tx_index: tx_index.into(),
            rpc_address,
        }
    }
}
