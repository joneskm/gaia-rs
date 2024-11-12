use std::{
    fmt::{Debug, Display},
    marker::PhantomData,
};

use gears::{
    application::handlers::node::{ABCIHandler, ModuleInfo},
    baseapp::{errors::QueryError, genesis::NullGenesis, QueryResponse},
    context::{query::QueryContext, QueryableContext},
    core::Protobuf,
    params::ParamsSubspaceKey,
    store::{database::Database, StoreKey},
    tendermint::types::request::query::RequestQuery,
    types::tx::NullTxMsg,
};
use tracing::info;

use crate::{
    handler::UpgradeHandler,
    keeper::{downgrade_verified, set_downgrade_verified, UpgradeKeeper},
    types::{
        query::{
            QueryAppliedPlanRequest, QueryAppliedPlanResponse, QueryCurrentPlanRequest,
            QueryCurrentPlanResponse, QueryModuleVersionsRequest, QueryModuleVersionsResponse,
            UpgradeQueryRequest, UpgradeQueryResponse,
        },
        ModuleVersion,
    },
    Module,
};

#[derive(Debug, Clone)]
pub struct UpgradeAbciHandler<SK: StoreKey, PSK: ParamsSubspaceKey, M: Module, UH, MI> {
    keeper: UpgradeKeeper<SK, M, UH>,
    _marker: PhantomData<(MI, SK, PSK, M)>,
}

impl<SK: StoreKey, PSK: ParamsSubspaceKey, M: Module, UH: UpgradeHandler, MI: ModuleInfo>
    ABCIHandler for UpgradeAbciHandler<SK, PSK, M, UH, MI>
where
    <M as TryFrom<Vec<u8>>>::Error: Display + Debug,
{
    type Message = NullTxMsg;

    type Genesis = NullGenesis;

    type StoreKey = SK;

    type QReq = UpgradeQueryRequest;

    type QRes = UpgradeQueryResponse;

    fn typed_query<DB: gears::store::database::Database>(
        &self,
        ctx: &gears::context::query::QueryContext<DB, Self::StoreKey>,
        query: Self::QReq,
    ) -> Self::QRes {
        match query {
            UpgradeQueryRequest::Plan(_) => Self::QRes::Plan(self.query_plan(ctx)),
            UpgradeQueryRequest::Applied(query) => {
                Self::QRes::Applied(self.query_applied(ctx, query))
            }
            UpgradeQueryRequest::ModuleVersions(query) => {
                Self::QRes::ModuleVersions(self.query_module_versions(ctx, query))
            }
        }
    }

    fn run_ante_checks<DB: gears::store::database::Database>(
        &self,
        _: &mut gears::context::tx::TxContext<'_, DB, Self::StoreKey>,
        _: &gears::types::tx::raw::TxWithRaw<Self::Message>,
        _: bool,
    ) -> Result<(), gears::application::handlers::node::TxError> {
        Ok(())
    }

    fn msg<DB: gears::store::database::Database>(
        &self,
        _: &mut gears::context::tx::TxContext<'_, DB, Self::StoreKey>,
        _: &Self::Message,
    ) -> Result<(), gears::application::handlers::node::TxError> {
        Ok(())
    }

    fn init_genesis<DB: gears::store::database::Database>(
        &self,
        _: &mut gears::context::init::InitContext<'_, DB, Self::StoreKey>,
        _: Self::Genesis,
    ) -> Vec<gears::tendermint::types::proto::validator::ValidatorUpdate> {
        Vec::new()
    }

    fn query<DB: gears::store::database::Database + Send + Sync>(
        &self,
        ctx: &gears::context::query::QueryContext<DB, Self::StoreKey>,
        RequestQuery {
            data,
            path,
            height: _,
            prove: _,
        }: RequestQuery,
    ) -> Result<Vec<u8>, gears::baseapp::errors::QueryError> {
        let query = match path.as_str() {
            QueryCurrentPlanRequest::QUERY_URL => {
                Self::QReq::Plan(QueryCurrentPlanRequest::decode(data)?)
            }
            QueryAppliedPlanRequest::QUERY_URL => {
                Self::QReq::Applied(QueryAppliedPlanRequest::decode(data)?)
            }
            QueryModuleVersionsRequest::QUERY_URL => {
                Self::QReq::ModuleVersions(QueryModuleVersionsRequest::decode(data)?)
            }
            _ => Err(QueryError::PathNotFound)?,
        };

        Ok(ABCIHandler::typed_query(self, ctx, query).into_bytes())
    }

    fn begin_block<'b, DB: gears::store::database::Database>(
        &self,
        ctx: &mut gears::context::block::BlockContext<'_, DB, Self::StoreKey>,
        request: gears::tendermint::request::RequestBeginBlock,
    ) {
        let plan = self.keeper.upgrade_plan(ctx);

        if !downgrade_verified() {
            set_downgrade_verified(true);
            let last_applied_plan = self.keeper.last_completed_upgrade(ctx);

            let is_none = plan.is_none();
            let should_execute = plan
                .as_ref()
                .map(|this| this.should_execute(ctx))
                .unwrap_or_default();

            // This check will make sure that we are using a valid binary.
            // It'll panic in these cases if there is no upgrade handler registered for the last applied upgrade.
            // 1. If there is no scheduled upgrade.
            // 2. If the plan is not ready.
            // 3. If the plan is ready and skip upgrade height is set for current height.
            if is_none || !should_execute || self.keeper.is_skip_height(ctx.height()) {
                match last_applied_plan {
                    Some(upg) if self.keeper.has_handler(&upg.name) => panic!(
                        "Wrong app version {}, upgrade handler is missing for {} upgrade plan",
                        request.header.version.app, upg.name,
                    ),
                    _ => (),
                }
            }
        }

        let plan = match plan {
            Some(plan) => plan,
            None => return,
        };

        // To make sure clear upgrade is executed at the same block
        if plan.should_execute(ctx) {
            // If skip upgrade has been set for current height, we clear the upgrade plan
            if self.keeper.is_skip_height(ctx.height()) {
                info!(
                    "UPGRADE `{}` SKIPPED at {}: {}",
                    plan.name.as_ref(),
                    plan.height,
                    plan.info
                );

                self.keeper.delete_upgrade_plan(ctx);
                return;
            }

            if !self.keeper.has_handler(&plan.name) {
                // TODO: store info https://github.com/cosmos/cosmos-sdk/blob/d3f09c222243bb3da3464969f0366330dcb977a8/x/upgrade/keeper/keeper.go#L375-L396

                // We don't have an upgrade handler for this upgrade name, meaning this software is out of date so shutdown
                let msg = format!(
                    "UPGRADE `{}` NEEDED at height: {}: {}",
                    plan.name.as_ref(),
                    plan.height,
                    plan.info
                );
                let log_msg = msg.clone();

                tracing::error!("{log_msg}");
                panic!("{msg}");
            }

            tracing::info!(
                "applying upgrade `{}` at height: {}",
                plan.name.as_ref(),
                plan.height
            );

            // todo: why they need gas https://github.com/cosmos/cosmos-sdk/blob/d3f09c222243bb3da3464969f0366330dcb977a8/x/upgrade/abci.go#L75
            match self.keeper.apply_upgrade(ctx, plan) {
                Ok(_) => return,
                Err(err) => panic!("{err}"),
            }
        }

        if self.keeper.has_handler(&plan.name) {
            let msg = format!(
                "BINARY UPDATED BEFORE TRIGGER! UPGRADE `{}` - in binary but not executed on chain",
                plan.name.as_ref()
            );
            let log_msg = msg.clone();
            tracing::error!("{log_msg}");
            panic!("{msg}");
        }
    }
}

impl<SK: StoreKey, PSK: ParamsSubspaceKey, M: Module, UH: UpgradeHandler, MI: ModuleInfo>
    UpgradeAbciHandler<SK, PSK, M, UH, MI>
where
    <M as TryFrom<Vec<u8>>>::Error: Display + Debug,
{
    pub fn query_plan<DB: Database>(&self, ctx: &QueryContext<DB, SK>) -> QueryCurrentPlanResponse {
        QueryCurrentPlanResponse {
            plan: self.keeper.upgrade_plan(ctx),
        }
    }

    pub fn query_applied<DB: Database>(
        &self,
        ctx: &QueryContext<DB, SK>,
        QueryAppliedPlanRequest { name }: QueryAppliedPlanRequest,
    ) -> QueryAppliedPlanResponse {
        QueryAppliedPlanResponse {
            height: self.keeper.done_height(ctx, name).unwrap_or_default(),
        }
    }

    pub fn query_module_versions<DB: Database>(
        &self,
        ctx: &QueryContext<DB, SK>,
        QueryModuleVersionsRequest { module_name }: QueryModuleVersionsRequest,
    ) -> QueryModuleVersionsResponse {
        let mut list = match module_name.is_empty() {
            true => self
                .keeper
                .modules_version(ctx)
                .into_iter()
                .map(|(key, version)| ModuleVersion {
                    name: key.name().to_owned(),
                    version,
                })
                .collect::<Vec<_>>(),
            false => {
                match self
                    .keeper
                    .modules_version(ctx)
                    .into_iter()
                    .find(|this| this.0.name() == module_name)
                {
                    Some((key, version)) => [ModuleVersion {
                        name: key.name().to_owned(),
                        version,
                    }]
                    .to_vec(),
                    None => Vec::new(),
                }
            }
        };

        list.sort();

        QueryModuleVersionsResponse {
            module_versions: list,
        }
    }
}
