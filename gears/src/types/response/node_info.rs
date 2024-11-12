use serde::{Deserialize, Serialize};
use tendermint::types::proto::p2p::DefaultNodeInfo;

/// GetNodeInfoResponse is the response type for the Query/GetNodeInfo RPC method.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct GetNodeInfoResponse {
    pub default_node_info: Option<DefaultNodeInfo>,
    pub application_version: Option<VersionInfo>,
}

/// VersionInfo is the type for the GetNodeInfoResponse message.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct VersionInfo {
    pub name: String,
    pub app_name: String,
    pub version: String,
    pub git_commit: String,
    pub build_tags: String,
    pub rust_version: String,
    // pub go_version: String,
    pub build_deps: Vec<Module>,
    /// Since: cosmos-sdk 0.43
    pub cosmos_sdk_version: String,
}

/// Module is the type for VersionInfo
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Module {
    /// module path
    pub path: String,
    /// module version
    pub version: String,
    /// checksum
    pub sum: String,
}
