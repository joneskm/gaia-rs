use std::{cell::RefCell, sync::Arc};

use gears::{
    application::handlers::node::ABCIHandler,
    baseapp::{options::NodeOptions, NullQueryRequest, NullQueryResponse},
    gas::metering::GasMeter,
    params::ParamsSubspaceKey,
    store::StoreKey,
    types::{
        base::min_gas::MinGasPrices,
        tx::{raw::TxWithRaw, NullTxMsg},
    },
    x::{
        ante::{BaseAnteHandler, SignGasConsumer},
        keepers::{
            auth::AuthKeeper,
            bank::BankKeeper,
            staking::{KeeperHooks, StakingBankKeeper},
        },
        module::Module,
    },
};

use crate::genesis::GenutilGenesis;

#[derive(Debug, Clone)]
pub struct GenutilAbciHandler<
    SK: StoreKey,
    PSK: ParamsSubspaceKey,
    AK: AuthKeeper<SK, M>,
    BK: StakingBankKeeper<SK, M> + BankKeeper<SK, M>,
    KH: KeeperHooks<SK, AK, M>,
    M: Module,
    GC: SignGasConsumer,
> {
    staking: staking::Keeper<SK, PSK, AK, BK, KH, M>,
    ante_handler: BaseAnteHandler<BK, AK, SK, GC, M>,
}

impl<
        SK: StoreKey,
        PSK: ParamsSubspaceKey,
        AK: AuthKeeper<SK, M>,
        BK: StakingBankKeeper<SK, M>,
        KH: KeeperHooks<SK, AK, M>,
        M: Module,
        GC: SignGasConsumer,
    > GenutilAbciHandler<SK, PSK, AK, BK, KH, M, GC>
{
    pub fn new(
        staking: staking::Keeper<SK, PSK, AK, BK, KH, M>,
        ante_handler: BaseAnteHandler<BK, AK, SK, GC, M>,
    ) -> Self {
        Self {
            staking,
            ante_handler,
        }
    }
}

impl<
        SK: StoreKey,
        PSK: ParamsSubspaceKey,
        AK: AuthKeeper<SK, M> + Send + Sync + 'static,
        BK: StakingBankKeeper<SK, M>,
        KH: KeeperHooks<SK, AK, M>,
        M: Module,
        GC: SignGasConsumer,
    > ABCIHandler for GenutilAbciHandler<SK, PSK, AK, BK, KH, M, GC>
{
    type Message = NullTxMsg;

    type Genesis = GenutilGenesis;

    type StoreKey = SK;

    type QReq = NullQueryRequest;

    type QRes = NullQueryResponse;

    fn typed_query<DB: gears::store::database::Database>(
        &self,
        _ctx: &gears::context::query::QueryContext<DB, Self::StoreKey>,
        _query: Self::QReq,
    ) -> Self::QRes {
        unreachable!()
    }

    fn run_ante_checks<DB: gears::store::database::Database>(
        &self,
        _ctx: &mut gears::context::tx::TxContext<'_, DB, Self::StoreKey>,
        _tx: &gears::types::tx::raw::TxWithRaw<Self::Message>,
        _: bool,
    ) -> Result<(), gears::application::handlers::node::TxError> {
        Ok(())
    }

    fn msg<DB: gears::store::database::Database>(
        &self,
        _ctx: &mut gears::context::tx::TxContext<'_, DB, Self::StoreKey>,
        _msg: &Self::Message,
    ) -> Result<(), gears::application::handlers::node::TxError> {
        unreachable!()
    }

    fn init_genesis<DB: gears::store::database::Database>(
        &self,
        ctx: &mut gears::context::init::InitContext<'_, DB, Self::StoreKey>,
        genesis: Self::Genesis,
    ) -> Vec<gears::tendermint::types::proto::validator::ValidatorUpdate> {
        if genesis.gen_txs.is_empty() {
            vec![]
        } else {
            for mut tx in genesis.gen_txs {
                tx.set_signatures_data();
                let tx = TxWithRaw::from(tx);
                let ante_check_res = self.ante_handler.run(
                    ctx,
                    &tx,
                    false,
                    NodeOptions::new(MinGasPrices::default()),
                    Arc::new(RefCell::new(GasMeter::infinite())),
                );

                match ante_check_res {
                    Ok(_) => (),
                    Err(err) => panic!("Failed to run ante checks for tx: {err}"),
                }

                let msg = tx.tx.body.messages.first(); // We know that such tx should contain only one message

                let tx_result = self.staking.create_validator(
                    ctx,
                    ctx.consensus_params().validator.clone(),
                    msg,
                );

                match tx_result {
                    Ok(_) => (),
                    Err(err) => panic!("Failed to run message from tx: {err}"),
                }
            }

            match self.staking.apply_and_return_validator_set_updates(ctx) {
                Ok(res) => res,
                Err(err) => panic!("failed to apply validators err: {err}"),
            }
        }
    }

    fn query<DB: gears::store::database::Database + Send + Sync>(
        &self,
        _ctx: &gears::context::query::QueryContext<DB, Self::StoreKey>,
        _query: gears::tendermint::types::request::query::RequestQuery,
    ) -> Result<Vec<u8>, gears::baseapp::errors::QueryError> {
        unreachable!()
    }
}
