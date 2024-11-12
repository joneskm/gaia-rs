use crate::types::proto::{consensus::ConsensusParams, validator::ValidatorUpdate};

#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct ResponseInitChain {
    pub consensus_params: Option<ConsensusParams>,
    pub validators: Vec<ValidatorUpdate>,
    pub app_hash: ::prost::bytes::Bytes,
}

impl From<ResponseInitChain> for super::inner::ResponseInitChain {
    fn from(
        ResponseInitChain {
            consensus_params,
            validators,
            app_hash,
        }: ResponseInitChain,
    ) -> Self {
        Self {
            consensus_params: consensus_params.map(Into::into),
            validators: validators.into_iter().map(Into::into).collect(),
            app_hash,
        }
    }
}
