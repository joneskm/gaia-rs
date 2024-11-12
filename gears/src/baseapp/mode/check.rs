use database::Database;
use gas::metering::{
    basic_meter::BasicGasMeter, infinite_meter::InfiniteGasMeter, kind::BlockKind, Gas, GasMeter,
};
use kv_store::bank::multi::TransactionMultiBank;
use tendermint::types::proto::event::Event;

use super::ExecutionMode;
use crate::{
    application::handlers::node::ABCIHandler,
    baseapp::errors::RunTxError,
    context::{tx::TxContext, TransactionalContext},
    types::tx::raw::TxWithRaw,
};

/// Specific to `check_tx` ABCI method.
///
/// Mode to validate a transaction before letting
/// them into local mempool. This mode doesn't execute a
/// transaction only runs ante checks on them.
/// Performs gas metering too
#[derive(Debug)]
pub struct CheckTxMode<DB, AH: ABCIHandler> {
    pub(crate) block_gas_meter: GasMeter<BlockKind>,
    pub(crate) multi_store: TransactionMultiBank<DB, AH::StoreKey>,
}

impl<DB, AH: ABCIHandler> CheckTxMode<DB, AH> {
    pub fn new(max_gas: Gas, multi_store: TransactionMultiBank<DB, AH::StoreKey>) -> Self {
        Self {
            block_gas_meter: GasMeter::new(match max_gas {
                Gas::Infinite => Box::<InfiniteGasMeter>::default(),
                Gas::Finite(max_gas) => Box::new(BasicGasMeter::new(max_gas)),
            }),
            multi_store,
        }
    }
}

impl<DB: Database, AH: ABCIHandler> ExecutionMode<DB, AH> for CheckTxMode<DB, AH> {
    fn run_msg<'m>(
        ctx: &mut TxContext<'_, DB, AH::StoreKey>,
        _handler: &AH,
        _msgs: impl Iterator<Item = &'m AH::Message>,
    ) -> Result<Vec<Event>, RunTxError> {
        Ok(ctx.events_drain())
    }

    fn run_ante_checks(
        ctx: &mut TxContext<'_, DB, AH::StoreKey>,
        handler: &AH,
        tx_with_raw: &TxWithRaw<AH::Message>,
    ) -> Result<(), RunTxError> {
        handler
            .run_ante_checks(ctx, tx_with_raw, true)
            .inspect_err(|_| ctx.multi_store_mut().clear_cache())
            .map_err(RunTxError::from)
    }

    fn runnable(_: &mut TxContext<'_, DB, AH::StoreKey>) -> Result<(), RunTxError> {
        Ok(())
    }
}
