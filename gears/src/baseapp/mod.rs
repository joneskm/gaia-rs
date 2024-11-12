pub mod options;
use std::{
    fmt::Debug,
    marker::PhantomData,
    num::NonZero,
    sync::{Arc, RwLock},
};

use crate::{
    application::{handlers::node::ABCIHandler, ApplicationInfo},
    context::{query::QueryContext, simple::SimpleContext, tx::TxContext},
    error::POISONED_LOCK,
    params::ParamsSubspaceKey,
    types::tx::raw::TxWithRaw,
};
use bytes::Bytes;
use database::Database;
use errors::QueryError;
use gas::metering::{descriptor::BLOCK_GAS_DESCRIPTOR, kind::BlockKind, FiniteGas, Gas, GasMeter};
use kv_store::{
    bank::multi::{ApplicationMultiBank, TransactionMultiBank},
    query::QueryMultiStore,
};
use mode::build_tx_gas_meter;
use tendermint::types::{
    chain_id::ChainId,
    proto::{event::Event, header::Header},
    request::query::RequestQuery,
};

use self::{
    errors::RunTxError, mode::ExecutionMode, options::NodeOptions, state::ApplicationState,
};

mod abci;
pub mod errors;
pub mod genesis;
pub mod mode;
mod params;
mod query;
pub mod state;
pub use params::{
    BaseAppParamsKeeper, BlockParams, ConsensusParams, EvidenceParams, ValidatorParams,
};

pub use query::*;

/// Core ABCI application which stores all data needed to execute application
#[derive(Debug, Clone)]
pub struct BaseApp<DB: Database, PSK: ParamsSubspaceKey, H: ABCIHandler, AI: ApplicationInfo> {
    state: Arc<RwLock<ApplicationState<DB, H>>>,
    multi_store: Arc<RwLock<ApplicationMultiBank<DB, H::StoreKey>>>,
    abci_handler: H,
    block_header: Arc<RwLock<Header>>, // passed by Tendermint in call to begin_block
    baseapp_params_keeper: BaseAppParamsKeeper<PSK>,
    options: NodeOptions,
    _info_marker: PhantomData<AI>,
}

impl<DB: Database, PSK: ParamsSubspaceKey, H: ABCIHandler, AI: ApplicationInfo>
    BaseApp<DB, PSK, H, AI>
{
    /// Create new `self`. Gas prices would be set from
    /// params store which returns `Default` if not values is set.
    ///
    /// See [BlockParams] for details on which values is used
    pub fn new(db: DB, params_subspace_key: PSK, abci_handler: H, options: NodeOptions) -> Self {
        let multi_store = ApplicationMultiBank::new(Arc::new(db));
        let mut multi_store = match multi_store {
            Ok(ms) => ms,
            Err(err) => panic!("Failed to init MultiStore with err: {err}"),
        };

        let baseapp_params_keeper = BaseAppParamsKeeper {
            params_subspace_key,
        };

        let height = multi_store.head_version();
        let ctx = SimpleContext::new((&mut multi_store).into(), height, ChainId::default());

        let max_gas = baseapp_params_keeper
            .block_params(&ctx)
            .map(|e| e.max_gas)
            .unwrap_or_default();

        Self {
            abci_handler,
            block_header: Arc::new(RwLock::new(Default::default())),
            baseapp_params_keeper,
            state: Arc::new(RwLock::new(ApplicationState::new(
                Gas::from(max_gas),
                &multi_store,
            ))),
            multi_store: Arc::new(RwLock::new(multi_store)),
            options,
            _info_marker: PhantomData,
        }
    }

    fn get_block_header(&self) -> Header {
        self.block_header.read().expect(POISONED_LOCK).clone()
    }

    fn set_block_header(&self, header: Header) {
        let mut current_header = self.block_header.write().expect(POISONED_LOCK);
        *current_header = header;
    }

    fn run_query(&self, request: &RequestQuery) -> Result<Bytes, QueryError> {
        //TODO: request height u32
        let version = NonZero::new(
            request
                .height
                .try_into()
                .map_err(|_| QueryError::InvalidHeight)?,
        );

        let store = self.multi_store.read().expect(POISONED_LOCK);
        let ctx = QueryContext::new(
            QueryMultiStore::new(&*store, version)?,
            version.map(|this| this.get()).unwrap_or_default(),
        )?;

        self.abci_handler
            .query(&ctx, request.clone())
            .map(Into::into)
    }

    /// Execute transaction for specific mode
    fn run_tx<MD: ExecutionMode<DB, H>>(
        &self,
        raw: Bytes,
        multi_store: &mut TransactionMultiBank<DB, H::StoreKey>,
        gas_meter: &mut GasMeter<BlockKind>,
    ) -> Result<RunTxInfo, RunTxError> {
        let tx_with_raw: TxWithRaw<H::Message> =
            TxWithRaw::from_bytes(raw.clone()).map_err(|e: core_types::errors::CoreError| {
                RunTxError::InvalidTransaction(e.to_string())
            })?;

        let header = self.get_block_header();
        let height = header.height;

        let consensus_params = {
            self.baseapp_params_keeper
                .consensus_params(&SimpleContext::new(
                    multi_store.into(),
                    height,
                    header.chain_id.clone(),
                ))
        };

        let mut ctx = TxContext::new(
            multi_store,
            height,
            header,
            consensus_params,
            build_tx_gas_meter(height, Some(&tx_with_raw.tx.auth_info.fee)),
            gas_meter,
            self.options.clone(),
        );

        MD::runnable(&mut ctx)?;
        MD::run_ante_checks(&mut ctx, &self.abci_handler, &tx_with_raw)?;

        ctx.multi_store_mut().upgrade_cache();

        let gas_wanted = ctx.gas_meter.borrow().limit();
        let gas_used = ctx.gas_meter.borrow().consumed_or_limit();

        let events = MD::run_msg(
            &mut ctx,
            &self.abci_handler,
            tx_with_raw.tx.get_msgs().iter(),
        )?;

        ctx.block_gas_meter
            .consume_gas(gas_used, BLOCK_GAS_DESCRIPTOR)?;

        ctx.multi_store_mut().upgrade_cache();

        Ok(RunTxInfo {
            events,
            gas_wanted,
            gas_used,
        })
    }
}

impl<DB: Database, PSK: ParamsSubspaceKey, H: ABCIHandler, AI: ApplicationInfo> ApplicationInfo
    for BaseApp<DB, PSK, H, AI>
{
    const APP_NAME: &'static str = AI::APP_NAME;
    const APP_VERSION: &'static str = AI::APP_VERSION;
}

#[derive(Debug, Clone)]
pub struct RunTxInfo {
    pub events: Vec<Event>,
    pub gas_wanted: Gas,
    pub gas_used: FiniteGas,
}
