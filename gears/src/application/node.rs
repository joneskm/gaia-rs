use std::marker::PhantomData;

use database::{Database, DatabaseBuilder};

use super::{
    handlers::{node::ABCIHandler, AuxHandler},
    ApplicationInfo,
};
use crate::commands::node::{
    genesis::genesis_account_add,
    init::init,
    run::{run, RouterBuilder},
    AppCommands,
};
use crate::{
    config::{ApplicationConfig, Config},
    params::ParamsSubspaceKey,
};

/// A Gears application.
pub trait Node:
    AuxHandler
    + RouterBuilder<<Self::Handler as ABCIHandler>::QReq, <Self::Handler as ABCIHandler>::QRes>
{
    type ParamsSubspaceKey: ParamsSubspaceKey;
    type Handler: ABCIHandler;
    type ApplicationConfig: ApplicationConfig;
}

#[derive(Debug)]
pub struct NodeApplication<
    Core: Node,
    DB: Database,
    DBO: DatabaseBuilder<DB>,
    AHB: FnOnce(Config<Core::ApplicationConfig>) -> Core::Handler,
> {
    core: Core,
    abci_handler_builder: AHB,

    params_subspace_key: Core::ParamsSubspaceKey,
    db_builder: DBO,
    _marker: PhantomData<DB>,
}

impl<
        Core: Node,
        DB: Database,
        DBO: DatabaseBuilder<DB>,
        AHB: FnOnce(Config<Core::ApplicationConfig>) -> Core::Handler,
    > NodeApplication<Core, DB, DBO, AHB>
{
    pub fn new(
        core: Core,
        db_builder: DBO,
        abci_handler_builder: AHB,
        params_subspace_key: Core::ParamsSubspaceKey,
    ) -> Self {
        Self {
            core,
            abci_handler_builder,
            params_subspace_key,
            db_builder,
            _marker: PhantomData,
        }
    }

    /// Runs the command passed on the command line.
    pub fn execute<AI: ApplicationInfo>(
        self,
        command: AppCommands<Core::AuxCommands>,
    ) -> anyhow::Result<()> {
        match command {
            AppCommands::Init(cmd) => init::<_, Core::ApplicationConfig>(
                cmd,
                &<<Core as Node>::Handler as ABCIHandler>::Genesis::default(),
            )?,
            AppCommands::Run(cmd) => run::<DB, DBO, _, _, _, AI, _>(
                cmd,
                self.db_builder,
                self.params_subspace_key,
                self.abci_handler_builder,
                self.core,
            )?,
            AppCommands::GenesisAdd(cmd) => {
                genesis_account_add::<<<Core as Node>::Handler as ABCIHandler>::Genesis>(cmd)?
            }
            AppCommands::Aux(cmd) => {
                let cmd = self.core.prepare_aux(cmd)?;
                self.core.handle_aux(cmd)?;
            }
        };

        Ok(())
    }
}
