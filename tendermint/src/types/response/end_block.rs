use crate::types::proto::{consensus::ConsensusParams, event::Event, validator::ValidatorUpdate};

#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct ResponseEndBlock {
    pub validator_updates: Vec<ValidatorUpdate>,
    pub consensus_param_updates: Option<ConsensusParams>,
    pub events: Vec<Event>,
}

impl From<ResponseEndBlock> for super::inner::ResponseEndBlock {
    fn from(
        ResponseEndBlock {
            validator_updates,
            consensus_param_updates,
            events,
        }: ResponseEndBlock,
    ) -> Self {
        Self {
            validator_updates: validator_updates.into_iter().map(Into::into).collect(),
            consensus_param_updates: consensus_param_updates.map(Into::into),
            events: events.into_iter().map(Into::into).collect(),
        }
    }
}
