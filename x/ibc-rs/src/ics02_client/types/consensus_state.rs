use derive_more::{From, TryInto};
use ibc::clients::tendermint::consensus_state::ConsensusState as TmConsensusState;
use ibc::clients::tendermint::types::{
    ConsensusState as ConsensusStateType, TENDERMINT_CONSENSUS_STATE_TYPE_URL,
};

use ibc::core::client::types::error::ClientError;
use ibc::derive::ConsensusState;
use ibc::primitives::proto::Any;

#[derive(ConsensusState, Debug, Clone, From, TryInto)]
pub enum ConsensusState {
    Tendermint(TmConsensusState),
}

impl From<ConsensusStateType> for ConsensusState {
    fn from(value: ConsensusStateType) -> Self {
        ConsensusState::Tendermint(value.into())
    }
}

impl TryFrom<ConsensusState> for ConsensusStateType {
    type Error = ClientError;

    fn try_from(value: ConsensusState) -> Result<Self, Self::Error> {
        match value {
            ConsensusState::Tendermint(tm_consensus_state) => {
                Ok(tm_consensus_state.inner().clone())
            }
        }
    }
}

impl From<ConsensusState> for Any {
    fn from(value: ConsensusState) -> Self {
        match value {
            ConsensusState::Tendermint(tm_consensus_state) => tm_consensus_state.into(),
        }
    }
}

impl TryFrom<Any> for ConsensusState {
    type Error = ClientError;

    fn try_from(value: Any) -> Result<Self, Self::Error> {
        match value.type_url.as_str() {
            TENDERMINT_CONSENSUS_STATE_TYPE_URL => {
                Ok(ConsensusState::Tendermint(value.try_into()?))
            }
            _ => Err(ClientError::Other {
                description: "Unknown consensus state type".into(),
            }),
        }
    }
}
