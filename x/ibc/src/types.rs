use database::Database;
use gears::types::context::init_context::InitContext;
use ibc::clients::tendermint::client_state::ClientState as WrappedTendermintClientState;
use ibc::clients::tendermint::context::CommonContext;
use ibc::core::client::context::{ClientExecutionContext, ClientValidationContext};
use ibc::core::client::types::Height;
use ibc::core::host::types::path::{ClientConsensusStatePath, ClientStatePath};
use ibc::primitives::Timestamp;
use proto_messages::cosmos::ibc::types::tendermint::RawConsensusState;
use proto_messages::cosmos::ibc::types::{ClientError, ContextError, RawClientId};
use store::StoreKey;

// TODO: try to find this const in external crates
pub const ATTRIBUTE_KEY_MODULE: &str = "module";

#[derive(
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    Clone,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
)]
pub struct ClientId(pub String);

impl From<&str> for ClientId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

#[derive(
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    Clone,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
)]
pub struct Signer(pub String);

impl From<&str> for Signer {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

pub struct InitContextShim<'a, 'b, DB: Send + Sync, SK: Sync + Send>(
    pub &'a mut InitContext<'b, DB, SK>,
); // TODO: What about using `Cow` so we could have option for owned and reference? Note: I don't think Cow support mutable borrowing

impl<'a, 'b, DB: Database + Send + Sync, SK: StoreKey + Send + Sync>
    From<&'a mut InitContext<'b, DB, SK>> for InitContextShim<'a, 'b, DB, SK>
{
    fn from(value: &'a mut InitContext<'b, DB, SK>) -> Self {
        Self(value)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Infallible")]
pub struct InfallibleError;

impl From<InfallibleError> for ClientError {
    fn from(value: InfallibleError) -> Self {
        ClientError::Other {
            description: value.to_string(),
        }
    }
}

pub struct ConsensusState(pub RawConsensusState);

impl TryFrom<ConsensusState> for RawConsensusState {
    type Error = InfallibleError;

    fn try_from(value: ConsensusState) -> Result<Self, Self::Error> {
        Ok(value.0)
    }
}

impl<'a, 'b, DB: Database + Send + Sync, SK: StoreKey + Send + Sync> CommonContext
    for InitContextShim<'a, 'b, DB, SK>
{
    type ConversionError = InfallibleError;

    type AnyConsensusState = ConsensusState;

    fn host_timestamp(&self) -> Result<Timestamp, ContextError> {
        todo!()
    }

    fn host_height(&self) -> Result<Height, ContextError> {
        todo!()
    }

    fn consensus_state(
        &self,
        _client_cons_state_path: &ClientConsensusStatePath,
    ) -> Result<Self::AnyConsensusState, ContextError> {
        todo!()
    }

    fn consensus_state_heights(
        &self,
        _client_id: &RawClientId,
    ) -> Result<Vec<Height>, ContextError> {
        todo!()
    }
}

impl<'a, 'b, DB: Database + Send + Sync, SK: StoreKey + Send + Sync>
    ibc::core::host::ValidationContext for InitContextShim<'a, 'b, DB, SK>
{
    type V = Self;

    type E = Self;

    type AnyConsensusState = RawConsensusState;

    type AnyClientState = WrappedTendermintClientState;

    fn get_client_validation_context(&self) -> &Self::V {
        todo!()
    }

    fn client_state(&self, _client_id: &RawClientId) -> Result<Self::AnyClientState, ContextError> {
        todo!()
    }

    fn decode_client_state(
        &self,
        _client_state: ibc::primitives::proto::Any,
    ) -> Result<Self::AnyClientState, ContextError> {
        todo!()
    }

    fn consensus_state(
        &self,
        _client_cons_state_path: &ClientConsensusStatePath,
    ) -> Result<Self::AnyConsensusState, ContextError> {
        todo!()
    }

    fn host_height(&self) -> Result<Height, ContextError> {
        todo!()
    }

    fn host_timestamp(&self) -> Result<Timestamp, ContextError> {
        todo!()
    }

    fn host_consensus_state(
        &self,
        _height: &Height,
    ) -> Result<Self::AnyConsensusState, ContextError> {
        todo!()
    }

    fn client_counter(&self) -> Result<u64, ContextError> {
        todo!()
    }

    fn connection_end(
        &self,
        _conn_id: &ibc::core::host::types::identifiers::ConnectionId,
    ) -> Result<ibc::core::connection::types::ConnectionEnd, ContextError> {
        todo!()
    }

    fn validate_self_client(
        &self,
        _client_state_of_host_on_counterparty: ibc::primitives::proto::Any,
    ) -> Result<(), ContextError> {
        todo!()
    }

    fn commitment_prefix(&self) -> proto_messages::cosmos::ibc::types::CommitmentPrefix {
        todo!()
    }

    fn connection_counter(&self) -> Result<u64, ContextError> {
        todo!()
    }

    fn channel_end(
        &self,
        _channel_end_path: &ibc::core::host::types::path::ChannelEndPath,
    ) -> Result<ibc::core::channel::types::channel::ChannelEnd, ContextError> {
        todo!()
    }

    fn get_next_sequence_send(
        &self,
        _seq_send_path: &ibc::core::host::types::path::SeqSendPath,
    ) -> Result<ibc::core::host::types::identifiers::Sequence, ContextError> {
        todo!()
    }

    fn get_next_sequence_recv(
        &self,
        _seq_recv_path: &ibc::core::host::types::path::SeqRecvPath,
    ) -> Result<ibc::core::host::types::identifiers::Sequence, ContextError> {
        todo!()
    }

    fn get_next_sequence_ack(
        &self,
        _seq_ack_path: &ibc::core::host::types::path::SeqAckPath,
    ) -> Result<ibc::core::host::types::identifiers::Sequence, ContextError> {
        todo!()
    }

    fn get_packet_commitment(
        &self,
        _commitment_path: &ibc::core::host::types::path::CommitmentPath,
    ) -> Result<ibc::core::channel::types::commitment::PacketCommitment, ContextError> {
        todo!()
    }

    fn get_packet_receipt(
        &self,
        _receipt_path: &ibc::core::host::types::path::ReceiptPath,
    ) -> Result<ibc::core::channel::types::packet::Receipt, ContextError> {
        todo!()
    }

    fn get_packet_acknowledgement(
        &self,
        _ack_path: &ibc::core::host::types::path::AckPath,
    ) -> Result<ibc::core::channel::types::commitment::AcknowledgementCommitment, ContextError>
    {
        todo!()
    }

    fn channel_counter(&self) -> Result<u64, ContextError> {
        todo!()
    }

    fn max_expected_time_per_block(&self) -> std::time::Duration {
        todo!()
    }

    fn validate_message_signer(
        &self,
        _signer: &ibc::primitives::Signer,
    ) -> Result<(), ContextError> {
        todo!()
    }
}

impl<'a, 'b, DB: Database + Send + Sync, SK: StoreKey + Send + Sync>
    ibc::core::host::ExecutionContext for InitContextShim<'a, 'b, DB, SK>
{
    fn get_client_execution_context(&mut self) -> &mut Self::E {
        todo!()
    }

    fn increase_client_counter(&mut self) -> Result<(), ContextError> {
        todo!()
    }

    fn store_connection(
        &mut self,
        _connection_path: &ibc::core::host::types::path::ConnectionPath,
        _connection_end: ibc::core::connection::types::ConnectionEnd,
    ) -> Result<(), ContextError> {
        todo!()
    }

    fn store_connection_to_client(
        &mut self,
        _client_connection_path: &ibc::core::host::types::path::ClientConnectionPath,
        _conn_id: ibc::core::host::types::identifiers::ConnectionId,
    ) -> Result<(), ContextError> {
        todo!()
    }

    fn increase_connection_counter(&mut self) -> Result<(), ContextError> {
        todo!()
    }

    fn store_packet_commitment(
        &mut self,
        _commitment_path: &ibc::core::host::types::path::CommitmentPath,
        _commitment: ibc::core::channel::types::commitment::PacketCommitment,
    ) -> Result<(), ContextError> {
        todo!()
    }

    fn delete_packet_commitment(
        &mut self,
        _commitment_path: &ibc::core::host::types::path::CommitmentPath,
    ) -> Result<(), ContextError> {
        todo!()
    }

    fn store_packet_receipt(
        &mut self,
        _receipt_path: &ibc::core::host::types::path::ReceiptPath,
        _receipt: ibc::core::channel::types::packet::Receipt,
    ) -> Result<(), ContextError> {
        todo!()
    }

    fn store_packet_acknowledgement(
        &mut self,
        _ack_path: &ibc::core::host::types::path::AckPath,
        _ack_commitment: ibc::core::channel::types::commitment::AcknowledgementCommitment,
    ) -> Result<(), ContextError> {
        todo!()
    }

    fn delete_packet_acknowledgement(
        &mut self,
        _ack_path: &ibc::core::host::types::path::AckPath,
    ) -> Result<(), ContextError> {
        todo!()
    }

    fn store_channel(
        &mut self,
        _channel_end_path: &ibc::core::host::types::path::ChannelEndPath,
        _channel_end: ibc::core::channel::types::channel::ChannelEnd,
    ) -> Result<(), ContextError> {
        todo!()
    }

    fn store_next_sequence_send(
        &mut self,
        _seq_send_path: &ibc::core::host::types::path::SeqSendPath,
        _seq: ibc::core::host::types::identifiers::Sequence,
    ) -> Result<(), ContextError> {
        todo!()
    }

    fn store_next_sequence_recv(
        &mut self,
        _seq_recv_path: &ibc::core::host::types::path::SeqRecvPath,
        _seq: ibc::core::host::types::identifiers::Sequence,
    ) -> Result<(), ContextError> {
        todo!()
    }

    fn store_next_sequence_ack(
        &mut self,
        _seq_ack_path: &ibc::core::host::types::path::SeqAckPath,
        _seq: ibc::core::host::types::identifiers::Sequence,
    ) -> Result<(), ContextError> {
        todo!()
    }

    fn increase_channel_counter(&mut self) -> Result<(), ContextError> {
        todo!()
    }

    fn emit_ibc_event(
        &mut self,
        _event: proto_messages::cosmos::ibc::types::IbcEvent,
    ) -> Result<(), ContextError> {
        todo!()
    }

    fn log_message(&mut self, _message: String) -> Result<(), ContextError> {
        todo!()
    }
}

impl<'a, 'b, DB: Database + Send + Sync, SK: StoreKey + Send + Sync> ClientExecutionContext
    for InitContextShim<'a, 'b, DB, SK>
{
    type V = Self;

    type AnyClientState = WrappedTendermintClientState;

    type AnyConsensusState = RawConsensusState;

    fn store_client_state(
        &mut self,
        _client_state_path: ClientStatePath,
        _client_state: Self::AnyClientState,
    ) -> Result<(), ContextError> {
        todo!()
    }

    fn store_consensus_state(
        &mut self,
        _consensus_state_path: ClientConsensusStatePath,
        _consensus_state: Self::AnyConsensusState,
    ) -> Result<(), ContextError> {
        todo!()
    }

    fn delete_consensus_state(
        &mut self,
        _consensus_state_path: ClientConsensusStatePath,
    ) -> Result<(), ContextError> {
        todo!()
    }

    fn store_update_time(
        &mut self,
        _client_id: RawClientId,
        _height: Height,
        _host_timestamp: Timestamp,
    ) -> Result<(), ContextError> {
        todo!()
    }

    fn store_update_height(
        &mut self,
        _client_id: RawClientId,
        _height: Height,
        _host_height: Height,
    ) -> Result<(), ContextError> {
        todo!()
    }

    fn delete_update_time(
        &mut self,
        _client_id: RawClientId,
        _height: Height,
    ) -> Result<(), ContextError> {
        todo!()
    }

    fn delete_update_height(
        &mut self,
        _client_id: RawClientId,
        _height: Height,
    ) -> Result<(), ContextError> {
        todo!()
    }
}

impl<DB: Database + Send + Sync, SK: StoreKey + Send + Sync> ClientValidationContext
    for InitContextShim<'_, '_, DB, SK>
{
    fn client_update_time(
        &self,
        _client_id: &RawClientId,
        _height: &Height,
    ) -> Result<Timestamp, ContextError> {
        todo!()
    }

    fn client_update_height(
        &self,
        _client_id: &RawClientId,
        _height: &Height,
    ) -> Result<Height, ContextError> {
        todo!()
    }
}

impl<'a, 'b, DB: Database + Send + Sync, SK: StoreKey + Send + Sync>
    ibc::clients::tendermint::context::ValidationContext for InitContextShim<'a, 'b, DB, SK>
{
    fn next_consensus_state(
        &self,
        _client_id: &RawClientId,
        _height: &Height,
    ) -> Result<Option<Self::AnyConsensusState>, ContextError> {
        todo!()
    }

    fn prev_consensus_state(
        &self,
        _client_id: &RawClientId,
        _height: &Height,
    ) -> Result<Option<Self::AnyConsensusState>, ContextError> {
        todo!()
    }
}