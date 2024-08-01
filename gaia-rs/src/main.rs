#![warn(rust_2018_idioms)]

use clap::Parser;
use gaia_rs::abci_handler::GaiaABCIHandler;
use gaia_rs::client::{GaiaQueryCommands, GaiaTxCommands};
use gaia_rs::store_keys::GaiaParamsStoreKey;
use gaia_rs::{GaiaApplication, GaiaCore, GaiaCoreClient};
use gears::application::client::ClientApplication;
use gears::application::node::NodeApplication;
use gears::cli::aux::CliNilAuxCommand;
use gears::cli::CliApplicationArgs;
use gears::store::database::rocks::RocksDBBuilder;

type Args = CliApplicationArgs<
    GaiaApplication,
    CliNilAuxCommand,
    CliNilAuxCommand,
    GaiaTxCommands,
    GaiaQueryCommands,
>;

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    args.execute_or_help(
        |command| ClientApplication::new(GaiaCoreClient).execute(command.try_into()?),
        |command| {
            NodeApplication::<GaiaCore, _, _, _>::new(
                GaiaCore,
                RocksDBBuilder,
                GaiaABCIHandler::new,
                GaiaParamsStoreKey::BaseApp,
            )
            .execute::<GaiaApplication>(command.try_into()?)
        },
    )
}
