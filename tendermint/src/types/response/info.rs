#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResponseInfo {
    #[serde(default)]
    pub data: String,
    #[serde(default)]
    pub version: String,
    #[serde(with = "crate::types::serializers::from_str", default)]
    pub app_version: u64,
    #[serde(with = "crate::types::serializers::from_str", default)]
    pub last_block_height: u32,
    #[serde(default)]
    #[serde(skip_serializing_if = "::bytes::Bytes::is_empty")]
    pub last_block_app_hash: ::prost::bytes::Bytes,
}

impl From<ResponseInfo> for super::inner::ResponseInfo {
    fn from(
        ResponseInfo {
            data,
            version,
            app_version,
            last_block_height,
            last_block_app_hash,
        }: ResponseInfo,
    ) -> Self {
        Self {
            data,
            version,
            app_version,
            last_block_height: last_block_height.into(),
            last_block_app_hash,
        }
    }
}
