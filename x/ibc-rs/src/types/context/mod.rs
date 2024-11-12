#[allow(dead_code)] //TODO: remove this when ready
#[allow(unused_variables)] // TODO: remove
#[allow(unused_imports)] //TODO: remove
use derive_more::{From, TryInto};
use gears::context::tx::TxContext;
use gears::params::ParamsSubspaceKey;
use gears::store::database::Database;
use gears::store::StoreKey;
use gears::tendermint::types::proto::event::Event;
use gears::tendermint::types::proto::event::EventAttribute;
use ibc::clients::tendermint::client_state::ClientState as TmClientState;
use ibc::clients::tendermint::consensus_state::ConsensusState as TmConsensusState;
use ibc::clients::tendermint::types::{
    ClientState as ClientStateType, ConsensusState as ConsensusStateType,
    TENDERMINT_CLIENT_STATE_TYPE_URL, TENDERMINT_CONSENSUS_STATE_TYPE_URL,
};
//use ibc::core::client::context::client_state::ClientStateValidation;
use gears::context::QueryableContext;
use gears::context::TransactionalContext;
use ibc::core::client::context::{
    ClientExecutionContext, ClientValidationContext, ExtClientValidationContext,
};
use ibc::core::client::types::error::ClientError;
use ibc::core::client::types::Height;
use ibc::core::handler::types::error::ContextError;
use ibc::core::handler::types::events::IbcEvent;
use ibc::core::host::types::identifiers::ClientId;
use ibc::core::host::{ExecutionContext, ValidationContext};
use ibc::core::primitives::proto::Protobuf;
use ibc::core::router::router::Router;
use ibc::derive::{ClientState, ConsensusState};
use ibc::primitives::proto::Any;
use ibc::primitives::Timestamp;
use ibc::primitives::ToVec;
use serde::Serialize;

use crate::ics02_client::types::client_state::ClientState;
use crate::ics02_client::types::consensus_state::ConsensusState;
use crate::{
    ics02_client::{message::MsgCreateClient, Keeper as ClientKeeper},
    ics03_connection::Keeper as ConnectionKeeper,
    ics04_channel::Keeper as ChannelKeeper,
    types::genesis::GenesisState,
};

use crate::ics02_client::ClientParamsKeeper;
//use super::keeper::KEY_NEXT_CLIENT_SEQUENCE;
//use super::params::ClientParamsKeeper;
use crate::ics02_client::KEY_NEXT_CLIENT_SEQUENCE;

pub const CLIENT_STATE_KEY: &str = "clientState";
pub const CLIENT_PARAMS_KEY: &str = "clientParams";
pub const NEXT_CLIENT_SEQUENCE: &str = "nextClientSequence";

pub const KEY_CLIENT_STORE_PREFIX: &str = "clients";
pub const KEY_CONSENSUS_STATE_PREFIX: &str = "consensusStates";
pub const KEY_PROCESSED_TIME: &str = "/processedTime";

// KeyProcessedHeight is appended to consensus state key to store the processed height
const KEY_PROCESSED_HEIGHT: &str = "/processedHeight";
const KEY_ITERATE_CONSENSUS_STATE_PREFIX: &[u8; 22] = b"iterateConsensusStates";

#[derive(Debug)]
pub struct Context<'a, 'b, DB, SK, PSK> {
    pub gears_ctx: &'a mut TxContext<'b, DB, SK>,
    pub client_keeper: &'a ClientKeeper<SK, PSK>,
    pub connection_keeper: &'a ConnectionKeeper<SK, PSK>,
    pub channel_keeper: &'a ChannelKeeper<SK>,
    pub store_key: SK, //TODO: remove this
}

impl<'a, 'b, DB, SK, PSK> Context<'a, 'b, DB, SK, PSK> {
    fn big_endian_height_bytes(height: Height) -> [u8; 16] {
        let revision_number = height.revision_number().to_be_bytes();
        let revision_height = height.revision_height().to_be_bytes();

        let mut join = [0; 16];
        join[..8].copy_from_slice(&revision_number);
        join[8..].copy_from_slice(&revision_height);
        join
    }

    fn iteration_key(height: Height) -> [u8; 38] {
        let heights = Self::big_endian_height_bytes(height);

        let mut join = [0; 38];
        join[..22].copy_from_slice(KEY_ITERATE_CONSENSUS_STATE_PREFIX);
        join[22..].copy_from_slice(&heights);
        join
    }
}

impl<'a, 'b, DB: Database, SK: StoreKey, PSK: ParamsSubspaceKey> ClientValidationContext
    for Context<'a, 'b, DB, SK, PSK>
{
    type ClientStateRef = ClientState;
    type ConsensusStateRef = ConsensusState;

    fn client_state(
        &self,
        client_id: &ibc::core::host::types::identifiers::ClientId,
    ) -> Result<Self::ClientStateRef, ibc::core::handler::types::error::ContextError> {
        // TODO: impl this
        Err(ContextError::ClientError(
            ClientError::ClientStateNotFound {
                client_id: client_id.clone(),
            },
        ))
    }

    fn consensus_state(
        &self,
        client_cons_state_path: &ibc::core::host::types::path::ClientConsensusStatePath,
    ) -> Result<Self::ConsensusStateRef, ibc::core::handler::types::error::ContextError> {
        // TODO: check impl

        Err(ContextError::ClientError(
            ClientError::ConsensusStateNotFound {
                client_id: client_cons_state_path.client_id.clone(),
                height: Height::new(
                    client_cons_state_path.revision_number,
                    client_cons_state_path.revision_height,
                )
                .unwrap(), //TODO: unwrap
            },
        ))
    }

    fn client_update_meta(
        &self,
        client_id: &ibc::core::host::types::identifiers::ClientId,
        height: &ibc::core::client::types::Height,
    ) -> Result<
        (ibc::primitives::Timestamp, ibc::core::client::types::Height),
        ibc::core::handler::types::error::ContextError,
    > {
        todo!()
    }
}

impl<'a, 'b, DB: Database, SK: StoreKey, PSK: ParamsSubspaceKey> ValidationContext
    for Context<'a, 'b, DB, SK, PSK>
{
    type V = Self;

    type HostClientState = TmClientState;
    type HostConsensusState = TmConsensusState;

    fn get_client_validation_context(&self) -> &Self::V {
        self
    }

    fn host_height(
        &self,
    ) -> Result<ibc::core::client::types::Height, ibc::core::handler::types::error::ContextError>
    {
        todo!()
    }

    fn host_timestamp(
        &self,
    ) -> Result<ibc::primitives::Timestamp, ibc::core::handler::types::error::ContextError> {
        todo!()
    }

    fn host_consensus_state(
        &self,
        height: &ibc::core::client::types::Height,
    ) -> Result<Self::HostConsensusState, ibc::core::handler::types::error::ContextError> {
        todo!()
    }

    fn client_counter(&self) -> Result<u64, ibc::core::handler::types::error::ContextError> {
        // TODO: check impl
        let ibc_store = self.gears_ctx.kv_store(&self.store_key);
        let raw = ibc_store
            .get(KEY_NEXT_CLIENT_SEQUENCE)
            .map_err(|e| {
                ContextError::ClientError(ClientError::Other {
                    description: e.to_string(),
                })
            })?
            .unwrap()
            .try_into()
            .unwrap();

        Ok(u64::from_be_bytes(raw))
    }

    fn connection_end(
        &self,
        conn_id: &ibc::core::host::types::identifiers::ConnectionId,
    ) -> Result<
        ibc::core::connection::types::ConnectionEnd,
        ibc::core::handler::types::error::ContextError,
    > {
        todo!()
    }

    fn validate_self_client(
        &self,
        client_state_of_host_on_counterparty: Self::HostClientState,
    ) -> Result<(), ibc::core::handler::types::error::ContextError> {
        todo!()
    }

    fn commitment_prefix(&self) -> ibc::core::commitment_types::commitment::CommitmentPrefix {
        todo!()
    }

    fn connection_counter(&self) -> Result<u64, ibc::core::handler::types::error::ContextError> {
        todo!()
    }

    fn channel_end(
        &self,
        channel_end_path: &ibc::core::host::types::path::ChannelEndPath,
    ) -> Result<
        ibc::core::channel::types::channel::ChannelEnd,
        ibc::core::handler::types::error::ContextError,
    > {
        todo!()
    }

    fn get_next_sequence_send(
        &self,
        seq_send_path: &ibc::core::host::types::path::SeqSendPath,
    ) -> Result<
        ibc::core::host::types::identifiers::Sequence,
        ibc::core::handler::types::error::ContextError,
    > {
        todo!()
    }

    fn get_next_sequence_recv(
        &self,
        seq_recv_path: &ibc::core::host::types::path::SeqRecvPath,
    ) -> Result<
        ibc::core::host::types::identifiers::Sequence,
        ibc::core::handler::types::error::ContextError,
    > {
        todo!()
    }

    fn get_next_sequence_ack(
        &self,
        seq_ack_path: &ibc::core::host::types::path::SeqAckPath,
    ) -> Result<
        ibc::core::host::types::identifiers::Sequence,
        ibc::core::handler::types::error::ContextError,
    > {
        todo!()
    }

    fn get_packet_commitment(
        &self,
        commitment_path: &ibc::core::host::types::path::CommitmentPath,
    ) -> Result<
        ibc::core::channel::types::commitment::PacketCommitment,
        ibc::core::handler::types::error::ContextError,
    > {
        todo!()
    }

    fn get_packet_receipt(
        &self,
        receipt_path: &ibc::core::host::types::path::ReceiptPath,
    ) -> Result<
        ibc::core::channel::types::packet::Receipt,
        ibc::core::handler::types::error::ContextError,
    > {
        todo!()
    }

    fn get_packet_acknowledgement(
        &self,
        ack_path: &ibc::core::host::types::path::AckPath,
    ) -> Result<
        ibc::core::channel::types::commitment::AcknowledgementCommitment,
        ibc::core::handler::types::error::ContextError,
    > {
        todo!()
    }

    fn channel_counter(&self) -> Result<u64, ibc::core::handler::types::error::ContextError> {
        todo!()
    }

    fn max_expected_time_per_block(&self) -> std::time::Duration {
        todo!()
    }

    fn validate_message_signer(
        &self,
        _signer: &ibc::primitives::Signer,
    ) -> Result<(), ibc::core::handler::types::error::ContextError> {
        //TOOD: check impl
        // The signer is validated in our domain message types
        Ok(())
    }
}

impl<'a, 'b, DB: Database, SK: StoreKey, PSK: ParamsSubspaceKey> ExecutionContext
    for Context<'a, 'b, DB, SK, PSK>
{
    type E = Self;

    fn get_client_execution_context(&mut self) -> &mut Self::E {
        self
    }

    fn increase_client_counter(
        &mut self,
    ) -> Result<(), ibc::core::handler::types::error::ContextError> {
        //TODO: check impl

        let sequence = self.client_counter()? + 1;

        let mut ibc_store = self.gears_ctx.kv_store_mut(&self.store_key);
        ibc_store
            .set(KEY_NEXT_CLIENT_SEQUENCE.to_owned(), sequence.to_be_bytes())
            .map_err(|e| {
                ContextError::ClientError(ClientError::Other {
                    description: e.to_string(),
                })
            })?;

        Ok(())
    }

    fn store_connection(
        &mut self,
        connection_path: &ibc::core::host::types::path::ConnectionPath,
        connection_end: ibc::core::connection::types::ConnectionEnd,
    ) -> Result<(), ibc::core::handler::types::error::ContextError> {
        todo!()
    }

    fn store_connection_to_client(
        &mut self,
        client_connection_path: &ibc::core::host::types::path::ClientConnectionPath,
        conn_id: ibc::core::host::types::identifiers::ConnectionId,
    ) -> Result<(), ibc::core::handler::types::error::ContextError> {
        todo!()
    }

    fn increase_connection_counter(
        &mut self,
    ) -> Result<(), ibc::core::handler::types::error::ContextError> {
        todo!()
    }

    fn store_packet_commitment(
        &mut self,
        commitment_path: &ibc::core::host::types::path::CommitmentPath,
        commitment: ibc::core::channel::types::commitment::PacketCommitment,
    ) -> Result<(), ibc::core::handler::types::error::ContextError> {
        todo!()
    }

    fn delete_packet_commitment(
        &mut self,
        commitment_path: &ibc::core::host::types::path::CommitmentPath,
    ) -> Result<(), ibc::core::handler::types::error::ContextError> {
        todo!()
    }

    fn store_packet_receipt(
        &mut self,
        receipt_path: &ibc::core::host::types::path::ReceiptPath,
        receipt: ibc::core::channel::types::packet::Receipt,
    ) -> Result<(), ibc::core::handler::types::error::ContextError> {
        todo!()
    }

    fn store_packet_acknowledgement(
        &mut self,
        ack_path: &ibc::core::host::types::path::AckPath,
        ack_commitment: ibc::core::channel::types::commitment::AcknowledgementCommitment,
    ) -> Result<(), ibc::core::handler::types::error::ContextError> {
        todo!()
    }

    fn delete_packet_acknowledgement(
        &mut self,
        ack_path: &ibc::core::host::types::path::AckPath,
    ) -> Result<(), ibc::core::handler::types::error::ContextError> {
        todo!()
    }

    fn store_channel(
        &mut self,
        channel_end_path: &ibc::core::host::types::path::ChannelEndPath,
        channel_end: ibc::core::channel::types::channel::ChannelEnd,
    ) -> Result<(), ibc::core::handler::types::error::ContextError> {
        todo!()
    }

    fn store_next_sequence_send(
        &mut self,
        seq_send_path: &ibc::core::host::types::path::SeqSendPath,
        seq: ibc::core::host::types::identifiers::Sequence,
    ) -> Result<(), ibc::core::handler::types::error::ContextError> {
        todo!()
    }

    fn store_next_sequence_recv(
        &mut self,
        seq_recv_path: &ibc::core::host::types::path::SeqRecvPath,
        seq: ibc::core::host::types::identifiers::Sequence,
    ) -> Result<(), ibc::core::handler::types::error::ContextError> {
        todo!()
    }

    fn store_next_sequence_ack(
        &mut self,
        seq_ack_path: &ibc::core::host::types::path::SeqAckPath,
        seq: ibc::core::host::types::identifiers::Sequence,
    ) -> Result<(), ibc::core::handler::types::error::ContextError> {
        todo!()
    }

    fn increase_channel_counter(
        &mut self,
    ) -> Result<(), ibc::core::handler::types::error::ContextError> {
        todo!()
    }

    fn emit_ibc_event(
        &mut self,
        event: ibc::core::handler::types::events::IbcEvent,
    ) -> Result<(), ibc::core::handler::types::error::ContextError> {
        //TODO: implement

        match event {
            IbcEvent::CreateClient(c) => {
                self.gears_ctx.push_event(Event::new(
                    "create_client",
                    [
                        EventAttribute::new(
                            "client_id".into(),
                            c.client_id().as_bytes().to_vec().into(),
                            true,
                        ),
                        EventAttribute::new(
                            "client_type".into(),
                            c.client_type().as_str().to_owned().into(),
                            true,
                        ),
                        EventAttribute::new(
                            "consensus_height".into(),
                            c.consensus_height().to_string().into(),
                            true,
                        ),
                    ],
                ));
            }
            IbcEvent::UpdateClient(_) => todo!(),
            IbcEvent::UpgradeClient(_) => todo!(),
            IbcEvent::ClientMisbehaviour(_) => todo!(),
            IbcEvent::OpenInitConnection(_) => todo!(),
            IbcEvent::OpenTryConnection(_) => todo!(),
            IbcEvent::OpenAckConnection(_) => todo!(),
            IbcEvent::OpenConfirmConnection(_) => todo!(),
            IbcEvent::OpenInitChannel(_) => todo!(),
            IbcEvent::OpenTryChannel(_) => todo!(),
            IbcEvent::OpenAckChannel(_) => todo!(),
            IbcEvent::OpenConfirmChannel(_) => todo!(),
            IbcEvent::CloseInitChannel(_) => todo!(),
            IbcEvent::CloseConfirmChannel(_) => todo!(),
            IbcEvent::SendPacket(_) => todo!(),
            IbcEvent::ReceivePacket(_) => todo!(),
            IbcEvent::WriteAcknowledgement(_) => todo!(),
            IbcEvent::AcknowledgePacket(_) => todo!(),
            IbcEvent::TimeoutPacket(_) => todo!(),
            IbcEvent::ChannelClosed(_) => todo!(),
            IbcEvent::Module(_) => todo!(),
            IbcEvent::Message(m) => {
                //TODO: implement
            }
        };

        Ok(())
    }

    fn log_message(
        &mut self,
        message: String,
    ) -> Result<(), ibc::core::handler::types::error::ContextError> {
        //TODO: implement
        Ok(())
    }
}

impl<'a, 'b, DB: Database, SK: StoreKey, PSK: ParamsSubspaceKey> ClientExecutionContext
    for Context<'a, 'b, DB, SK, PSK>
{
    type ClientStateMut = ClientState;

    fn store_client_state(
        &mut self,
        client_state_path: ibc::core::host::types::path::ClientStatePath,
        client_state: Self::ClientStateRef,
    ) -> Result<(), ibc::core::handler::types::error::ContextError> {
        self.client_keeper
            .client_state_set(self.gears_ctx, client_state_path, client_state)
            .map_err(|e| {
                ContextError::ClientError(ClientError::Other {
                    description: e.to_string(),
                })
            })?;
        Ok(())
    }

    fn store_consensus_state(
        &mut self,
        consensus_state_path: ibc::core::host::types::path::ClientConsensusStatePath,
        consensus_state: Self::ConsensusStateRef,
    ) -> Result<(), ibc::core::handler::types::error::ContextError> {
        //TODO: check implementation

        let any: Any = consensus_state.into();
        let encoded_bytes = any.to_vec();

        // println!("encoded bytes:\n {:?}", encoded_bytes.clone());

        // let prefix = format!(
        //     "{KEY_CLIENT_STORE_PREFIX}/{}/",
        //     consensus_state_path.client_id
        // )
        // .into_bytes();

        // println!(
        //     "revision height: {:?}",
        //     consensus_state_path.revision_height
        // );

        // println!("prefix: {:?}", prefix.clone());

        // let key = format!(
        //     "{KEY_CONSENSUS_STATE_PREFIX}/{}-{}",
        //     consensus_state_path.revision_number, consensus_state_path.revision_height
        // )
        // .into_bytes();
        // println!("key: {:?}", key);

        self.gears_ctx
            .kv_store_mut(&self.store_key)
            .prefix_store_mut(
                format!(
                    "{KEY_CLIENT_STORE_PREFIX}/{}/",
                    consensus_state_path.client_id
                )
                .into_bytes(),
            )
            .set(
                format!(
                    "{KEY_CONSENSUS_STATE_PREFIX}/{}-{}",
                    consensus_state_path.revision_number, consensus_state_path.revision_height
                )
                .into_bytes(),
                encoded_bytes,
            )
            .map_err(|e| {
                ContextError::ClientError(ClientError::Other {
                    description: e.to_string(),
                })
            })?;

        Ok(())
    }

    fn delete_consensus_state(
        &mut self,
        consensus_state_path: ibc::core::host::types::path::ClientConsensusStatePath,
    ) -> Result<(), ibc::core::handler::types::error::ContextError> {
        todo!()
    }

    fn store_update_meta(
        &mut self,
        client_id: ibc::core::host::types::identifiers::ClientId,
        height: ibc::core::client::types::Height,
        host_timestamp: ibc::primitives::Timestamp,
        host_height: ibc::core::client::types::Height,
    ) -> Result<(), ibc::core::handler::types::error::ContextError> {
        //TODO: check implementation

        // set processed time
        let processed_time: u64 = 1714492003000000000; // TODO: hard coded for deterministic testing
        let store = self.gears_ctx.kv_store_mut(&self.store_key);
        let key = format!(
            "{KEY_CONSENSUS_STATE_PREFIX}/{}-{}{KEY_PROCESSED_TIME}",
            height.revision_number(),
            height.revision_height()
        )
        .into_bytes();
        let value = processed_time.to_be_bytes();
        let prefix = format!("{KEY_CLIENT_STORE_PREFIX}/{}/", client_id).into_bytes();
        store
            .prefix_store_mut(prefix)
            .set(key, value)
            .map_err(|e| {
                ContextError::ClientError(ClientError::Other {
                    description: e.to_string(),
                })
            })?;

        // set processed height
        let store = self.gears_ctx.kv_store_mut(&self.store_key);
        let key = format!(
            "{KEY_CONSENSUS_STATE_PREFIX}/{}-{}{KEY_PROCESSED_HEIGHT}",
            height.revision_number(),
            height.revision_height()
        )
        .into_bytes();
        let value = format!(
            "{}-{}",
            host_height.revision_number(),
            host_height.revision_height()
        )
        .into_bytes();
        let prefix = format!("{KEY_CLIENT_STORE_PREFIX}/{}/", client_id).into_bytes();
        store
            .prefix_store_mut(prefix)
            .set(key, value)
            .map_err(|e| {
                ContextError::ClientError(ClientError::Other {
                    description: e.to_string(),
                })
            })?;

        // set iteration key
        let store = self.gears_ctx.kv_store_mut(&self.store_key);
        let key = Self::iteration_key(height);
        let value = format!(
            "{KEY_CONSENSUS_STATE_PREFIX}/{}-{}",
            height.revision_number(),
            height.revision_height()
        )
        .into_bytes();
        let prefix = format!("{KEY_CLIENT_STORE_PREFIX}/{}/", client_id).into_bytes();

        store
            .prefix_store_mut(prefix)
            .set(key, value)
            .map_err(|e| {
                ContextError::ClientError(ClientError::Other {
                    description: e.to_string(),
                })
            })?;

        Ok(())
    }

    fn delete_update_meta(
        &mut self,
        client_id: ibc::core::host::types::identifiers::ClientId,
        height: ibc::core::client::types::Height,
    ) -> Result<(), ibc::core::handler::types::error::ContextError> {
        todo!()
    }
}

impl<'a, 'b, DB: Database, SK: StoreKey, PSK: ParamsSubspaceKey> ExtClientValidationContext
    for Context<'a, 'b, DB, SK, PSK>
{
    fn host_timestamp(
        &self,
    ) -> Result<ibc::primitives::Timestamp, ibc::core::handler::types::error::ContextError> {
        // TODO: check impl - should probably be using the timestamp from the context (block timestamp)
        Ok(Timestamp::now())
    }

    fn host_height(
        &self,
    ) -> Result<ibc::core::client::types::Height, ibc::core::handler::types::error::ContextError>
    {
        // TODO: check impl
        Ok(Height::new(0, self.gears_ctx.height().into()).unwrap()) //TODO: unwrap
    }

    fn consensus_state_heights(
        &self,
        client_id: &ibc::core::host::types::identifiers::ClientId,
    ) -> Result<Vec<ibc::core::client::types::Height>, ibc::core::handler::types::error::ContextError>
    {
        todo!()
    }

    fn next_consensus_state(
        &self,
        client_id: &ibc::core::host::types::identifiers::ClientId,
        height: &ibc::core::client::types::Height,
    ) -> Result<Option<Self::ConsensusStateRef>, ibc::core::handler::types::error::ContextError>
    {
        todo!()
    }

    fn prev_consensus_state(
        &self,
        client_id: &ibc::core::host::types::identifiers::ClientId,
        height: &ibc::core::client::types::Height,
    ) -> Result<Option<Self::ConsensusStateRef>, ibc::core::handler::types::error::ContextError>
    {
        todo!()
    }
}

#[derive(Debug)]
pub struct ClientRouter;

impl Router for ClientRouter {
    fn get_route(
        &self,
        module_id: &ibc::core::router::types::module::ModuleId,
    ) -> Option<&dyn ibc::core::router::module::Module> {
        todo!()
    }

    fn get_route_mut(
        &mut self,
        module_id: &ibc::core::router::types::module::ModuleId,
    ) -> Option<&mut dyn ibc::core::router::module::Module> {
        todo!()
    }

    fn lookup_module(
        &self,
        port_id: &ibc::core::host::types::identifiers::PortId,
    ) -> Option<ibc::core::router::types::module::ModuleId> {
        todo!()
    }
}
