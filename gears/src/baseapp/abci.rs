use super::{
    mode::{check::CheckTxMode, deliver::DeliverTxMode},
    BaseApp,
};
use crate::error::POISONED_LOCK;
use crate::params::ParamsSubspaceKey;
use crate::{application::handlers::node::ABCIHandler, context::init::InitContext};
use crate::{
    application::ApplicationInfo,
    context::simple::{SimpleBackend, SimpleContext},
};
use crate::{baseapp::RunTxInfo, context::block::BlockContext};
use bytes::Bytes;
use database::Database;
use extensions::lock::AcquireRwLock;
use gas::metering::Gas;
use tendermint::{
    application::ABCIApplication,
    types::{
        request::{
            begin_block::RequestBeginBlock,
            check_tx::RequestCheckTx,
            deliver_tx::RequestDeliverTx,
            echo::RequestEcho,
            end_block::RequestEndBlock,
            info::RequestInfo,
            init_chain::RequestInitChain,
            query::RequestQuery,
            snapshot::{RequestApplySnapshotChunk, RequestLoadSnapshotChunk, RequestOfferSnapshot},
        },
        response::{
            begin_block::ResponseBeginBlock,
            check_tx::ResponseCheckTx,
            deliver_tx::ResponseDeliverTx,
            echo::ResponseEcho,
            end_block::ResponseEndBlock,
            info::ResponseInfo,
            init_chain::ResponseInitChain,
            query::ResponseQuery,
            snapshot::{
                ResponseApplySnapshotChunk, ResponseListSnapshots, ResponseLoadSnapshotChunk,
                ResponseOfferSnapshot,
            },
            ResponseCommit, ResponseFlush,
        },
    },
};
use tracing::{debug, error, info};

impl<DB: Database, PSK: ParamsSubspaceKey, H: ABCIHandler, AI: ApplicationInfo>
    ABCIApplication<H::Genesis> for BaseApp<DB, PSK, H, AI>
{
    fn init_chain(
        &self,
        RequestInitChain {
            time,
            chain_id,
            consensus_params,
            validators: _, // TODO: should it be ignored?
            app_genesis,
            ..
        }: RequestInitChain<H::Genesis>,
    ) -> ResponseInitChain {
        let mut multi_store = self.multi_store.write().expect(POISONED_LOCK);
        let mut state = self.state.write().expect(POISONED_LOCK);

        //TODO: handle request height > 1 as is done in SDK
        // On a new chain, we consider the init chain block height as 0, even though
        // req.InitialHeight is 1 by default.
        // see https://github.com/cosmos/cosmos-sdk/blob/2582f0aab7b2cbf66ade066fe570a4622cf0b098/baseapp/abci.go#L28-L29
        let initial_height = 0;

        let mut ctx = InitContext::new(
            &mut multi_store,
            initial_height,
            time,
            chain_id,
            consensus_params.clone().into(),
        );

        self.baseapp_params_keeper
            .set_consensus_params(&mut ctx, consensus_params.clone().into());

        let val_updates = self
            .abci_handler
            .init_genesis(&mut ctx, app_genesis.clone()); //TODO: should also return consensus params

        // TODO: there's sanity checking of val_updates here in the Cosmos SDK

        state.append_block_cache(&mut multi_store);

        ResponseInitChain {
            consensus_params: Some(consensus_params),
            validators: val_updates,
            app_hash: "hash_goes_here".into(), //TODO: set app hash - note this will be the hash of block 1
        }
    }

    fn info(&self, _request: RequestInfo) -> ResponseInfo {
        let state = self.state.read().expect(POISONED_LOCK);

        ResponseInfo {
            data: AI::APP_NAME.to_owned(),
            version: AI::APP_VERSION.to_owned(),
            app_version: 1,
            last_block_height: state.last_height,
            last_block_app_hash: state.head_hash.to_vec().into(),
        }
    }

    fn query(&self, request: RequestQuery) -> ResponseQuery {
        match self.run_query(&request) {
            Ok(res) => ResponseQuery {
                code: 0,
                log: "exists".to_string(),
                info: "".to_string(),
                index: 0,
                key: request.data,
                value: res,
                proof_ops: None,
                height: request.height as u32,
                codespace: "".to_string(),
            },
            Err(e) => ResponseQuery {
                code: 1,
                log: e.to_string(),
                info: "".to_string(),
                index: 0,
                key: request.data,
                value: Default::default(),
                proof_ops: None,
                height: 0,
                codespace: "".to_string(),
            },
        }
    }

    fn check_tx(&self, RequestCheckTx { tx, r#type }: RequestCheckTx) -> ResponseCheckTx {
        let mut state = self.state.acquire_write();

        let CheckTxMode {
            block_gas_meter,
            multi_store,
        } = &mut state.check_mode;

        let result = match r#type {
            0 | 1 => self.run_tx::<CheckTxMode<_, _>>(tx.clone(), multi_store, block_gas_meter),
            _ => panic!("unknown Request CheckTx type: {}", r#type),
        };

        match result {
            Ok(RunTxInfo {
                events,
                gas_wanted,
                gas_used,
            }) => {
                debug!("{:?}", events);
                ResponseCheckTx {
                    code: 0,
                    data: Default::default(),
                    log: "".to_string(),
                    info: "".to_string(),
                    gas_wanted: gas_wanted.into(),
                    gas_used: gas_used.into(),
                    events,
                    codespace: "".to_string(),
                    mempool_error: "".to_string(),
                    priority: 0,
                    sender: "".to_string(),
                }
            }
            Err(e) => {
                error!("check err: {e}");
                ResponseCheckTx {
                    code: e.code(),
                    data: Default::default(),
                    log: e.to_string(),
                    info: "".to_string(),
                    gas_wanted: 1,
                    gas_used: 0,
                    events: vec![],
                    codespace: e.codespace().to_string(),
                    mempool_error: "".to_string(),
                    priority: 0,
                    sender: "".to_string(),
                }
            }
        }
    }

    fn deliver_tx(&self, RequestDeliverTx { tx }: RequestDeliverTx) -> ResponseDeliverTx {
        let mut state = self.state.write().expect(POISONED_LOCK);

        let DeliverTxMode {
            block_gas_meter,
            multi_store,
        } = &mut state.deliver_mode;

        let result = self.run_tx::<DeliverTxMode<_, _>>(tx.clone(), multi_store, block_gas_meter);

        match result {
            Ok(RunTxInfo {
                events,
                gas_wanted,
                gas_used,
            }) => ResponseDeliverTx {
                code: 0,
                data: Default::default(),
                log: "".to_string(),
                info: "".to_string(),
                gas_wanted: gas_wanted.into(),
                gas_used: gas_used.into(),
                events: events.into_iter().collect(),
                codespace: "".to_string(),
            },
            Err(e) => {
                info!("Failed to process tx: {}", e);
                ResponseDeliverTx {
                    code: e.code(),
                    data: Bytes::new(),
                    log: e.to_string(),
                    info: "".to_string(),
                    gas_wanted: 0,
                    gas_used: 0,
                    events: vec![],
                    codespace: e.codespace().to_string(),
                }
            }
        }
    }

    fn commit(&self) -> ResponseCommit {
        let mut multi_store = self.multi_store.write().expect(POISONED_LOCK);
        let mut state = self.state.write().expect(POISONED_LOCK);

        let height = self.get_block_header().height;

        let hash = state.commit(&mut multi_store);

        info!(
            "Committed state, block height: {} app hash: {}",
            height,
            hex::encode(hash)
        );

        ResponseCommit {
            data: hash.to_vec().into(),
            retain_height: 0, // this is the height above which tendermint will retain all blocks // TODO: make this configurable as in Cosmos
        }
    }

    fn echo(&self, request: RequestEcho) -> ResponseEcho {
        ResponseEcho {
            message: request.message,
        }
    }

    fn begin_block(&self, request: RequestBeginBlock) -> ResponseBeginBlock {
        //TODO: Cosmos SDK validates the request height here

        self.set_block_header(request.header.clone());

        let mut state = self.state.write().expect(POISONED_LOCK);
        let mut multi_store = self.multi_store.write().expect(POISONED_LOCK);

        let ctx = SimpleContext::new(
            SimpleBackend::Application(&mut multi_store),
            request.header.height,
            request.header.chain_id.clone(),
        );

        let max_gas = self
            .baseapp_params_keeper
            .block_params(&ctx)
            .map(|e| e.max_gas)
            .unwrap_or_default(); // This is how cosmos handles it https://github.com/cosmos/cosmos-sdk/blob/d3f09c222243bb3da3464969f0366330dcb977a8/baseapp/baseapp.go#L497

        let consensus_params = self.baseapp_params_keeper.consensus_params(&ctx);

        state.replace_meter(Gas::from(max_gas));

        let mut ctx = BlockContext::new(
            &mut multi_store,
            request.header.height,
            request.header.clone(),
            consensus_params,
        );

        self.abci_handler.begin_block(&mut ctx, request);

        let events = ctx.events;

        state.append_block_cache(&mut multi_store);

        ResponseBeginBlock {
            events: events.into_iter().collect(),
        }
    }

    fn end_block(&self, request: RequestEndBlock) -> ResponseEndBlock {
        let mut state = self.state.write().expect(POISONED_LOCK);
        let mut multi_store = self.multi_store.write().expect(POISONED_LOCK);

        state.take_block_cache(&mut multi_store);

        let header = self.get_block_header();

        let consensus_params = {
            let ctx = SimpleContext::new(
                SimpleBackend::Application(&mut multi_store),
                header.height,
                header.chain_id.clone(),
            );

            self.baseapp_params_keeper.consensus_params(&ctx)
        };

        let mut ctx = BlockContext::new(
            &mut multi_store,
            header.height,
            header.clone(),
            consensus_params,
        );

        let validator_updates = self.abci_handler.end_block(&mut ctx, request);

        let events = ctx.events;

        state.append_block_cache(&mut multi_store);

        ResponseEndBlock {
            events: events.into_iter().collect(),
            validator_updates,
            consensus_param_updates: None,
            // TODO: there is only one call to BaseAppParamsKeeper::set_consensus_params,
            // which is made during init. This means that these params cannot change.
            // However a get method should be implemented in future.
        }
    }

    /// Signals that messages queued on the client should be flushed to the server.
    fn flush(&self) -> ResponseFlush {
        ResponseFlush {}
    }

    /// Used during state sync to discover available snapshots on peers.
    fn list_snapshots(&self) -> ResponseListSnapshots {
        Default::default()
    }

    /// Called when bootstrapping the node using state sync.
    fn offer_snapshot(&self, _request: RequestOfferSnapshot) -> ResponseOfferSnapshot {
        Default::default()
    }

    /// Used during state sync to retrieve chunks of snapshots from peers.
    fn load_snapshot_chunk(&self, _request: RequestLoadSnapshotChunk) -> ResponseLoadSnapshotChunk {
        Default::default()
    }

    /// Apply the given snapshot chunk to the application's state.
    fn apply_snapshot_chunk(
        &self,
        _request: RequestApplySnapshotChunk,
    ) -> ResponseApplySnapshotChunk {
        Default::default()
    }
}
