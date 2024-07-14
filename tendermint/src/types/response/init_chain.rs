use crate::types::proto::{consensus::ConsensusParams, validator::ValidatorUpdate};

#[derive(Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct ResponseInitChain {
    pub consensus_params: Option<ConsensusParams>,
    pub validators: Vec<ValidatorUpdate>,
    pub app_hash: [u8; 32],
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
            app_hash: app_hash.to_vec().into(),
        }
    }
}
