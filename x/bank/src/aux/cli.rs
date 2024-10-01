use std::{marker::PhantomData, path::PathBuf};

use clap::{ArgAction, Args, Subcommand, ValueHint};
use gears::application::ApplicationInfo;

use super::{metadata::CoinsMetaGenesisCmd, BankAuxCmd};

/// Add metadata about coins to genesis file
#[derive(Debug, Clone, Args)]
pub struct CoinsMetaGenesisCli<AI: ApplicationInfo> {
    #[arg(long, action = ArgAction::Set, value_hint = ValueHint::DirPath, default_value_os_t = AI::home_dir(), help = "directory for config and data")]
    pub home: PathBuf,
    /// Json with array of metadata or path to json file
    pub metadata: String,
    /// Dedup input metadata list
    #[arg(long, default_value_t = false)]
    pub dedup_input: bool,
    /// Return error if found duplicates with original genesis file
    #[arg(long, default_value_t = false)]
    pub fail_on_dup: bool,
    /// Overwrite metadata with same coin name
    #[arg(long, default_value_t = false)]
    pub overwrite_same: bool,

    #[arg(skip)]
    _marker: PhantomData<AI>,
}

impl<AI: ApplicationInfo> From<CoinsMetaGenesisCli<AI>> for CoinsMetaGenesisCmd {
    fn from(
        CoinsMetaGenesisCli {
            home,
            metadata,
            dedup_input,
            fail_on_dup,
            overwrite_same,
            _marker,
        }: CoinsMetaGenesisCli<AI>,
    ) -> Self {
        Self {
            home,
            metadata,
            dedup_input,
            fail_on_dup,
            overwrite_same,
        }
    }
}

#[derive(Debug, Clone, Subcommand)]
pub enum BankAuxCliCommands<AI: ApplicationInfo> {
    #[command(name = "add-coins-metadata")]
    Genesis(CoinsMetaGenesisCli<AI>),
}

#[derive(Debug, Clone, Args)]
pub struct BankAuxCli<AI: ApplicationInfo> {
    #[command(subcommand)]
    pub command: BankAuxCliCommands<AI>,
}

impl<AI: ApplicationInfo> TryFrom<BankAuxCli<AI>> for BankAuxCmd {
    type Error = anyhow::Error;

    fn try_from(BankAuxCli { command }: BankAuxCli<AI>) -> Result<Self, Self::Error> {
        match command {
            BankAuxCliCommands::Genesis(coins_meta_genesis_cli) => {
                Ok(BankAuxCmd::Genesis(coins_meta_genesis_cli.into()))
            }
        }
    }
}
