//! This modules should be added to test modules with `#[path = "./utilities.rs"]` as it contains gaia specific code and dedicated crate is bothersome.
#![allow(dead_code)]

use std::{path::PathBuf, time::Duration};

use gaia_rs::{
    abci_handler::GaiaABCIHandler,
    config::AppConfig,
    genesis::GenesisState,
    store_keys::{GaiaParamsStoreKey, GaiaStoreKey},
    GaiaApplication, GaiaCore,
};
use gears::{
    application::node::NodeApplication,
    baseapp::genesis::Genesis,
    commands::{
        client::keys::{keys, AddKeyCommand, KeyCommand, KeyringBackend},
        node::{
            run::{LogLevel, RunCommand},
            AppCommands,
        },
    },
    config::{DEFAULT_ADDRESS, DEFAULT_REST_LISTEN_ADDR},
};
use gears::{
    core::address::AccAddress,
    types::base::send::SendCoins,
    utils::{TempDir, TmpChild},
};

pub const TENDERMINT_PATH: &str = "./tests/assets";
pub const NODE_URL_STR: &str = "http://localhost:26657/";

pub fn node_url() -> url::Url {
    NODE_URL_STR.try_into().expect("Default should be valid")
}

/// Helper method to start gaia node and tendermint in tmp folder
pub fn run_gaia_and_tendermint() -> anyhow::Result<(TmpChild, std::thread::JoinHandle<()>)> {
    let tmp_dir = TempDir::new()?;
    let tmp_path = tmp_dir.to_path_buf();

    let tendermint = TmpChild::run_tendermint::<_, AppConfig>(
        tmp_dir,
        TENDERMINT_PATH,
        &MockGenesis::default(),
    )?;

    std::thread::sleep(Duration::from_secs(10));

    let server_thread = std::thread::spawn(move || {
        let node = NodeApplication::<'_, GaiaCore, GaiaApplication>::new(
            GaiaCore,
            &GaiaABCIHandler::new,
            GaiaStoreKey::Params,
            GaiaParamsStoreKey::BaseApp,
        );

        let cmd = RunCommand {
            home: tmp_path,
            address: DEFAULT_ADDRESS,
            rest_listen_addr: DEFAULT_REST_LISTEN_ADDR,
            read_buf_size: 1048576,
            log_level: LogLevel::Off,
        };

        let _ = node.execute(AppCommands::Run(cmd));
    });

    std::thread::sleep(Duration::from_secs(10));

    Ok((tendermint, server_thread))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct MockGenesis(pub GenesisState);

impl Genesis for MockGenesis {
    fn add_genesis_account(
        &mut self,
        address: AccAddress,
        coins: SendCoins,
    ) -> Result<(), gears::error::AppError> {
        self.0.add_genesis_account(address, coins)
    }
}

pub const KEY_NAME: &str = "alice";

pub fn key_add(home: impl Into<PathBuf>) -> anyhow::Result<()> {
    let cmd = AddKeyCommand {
        name: KEY_NAME.to_owned(),
        recover: Default::default(),
        home: home.into(),
        keyring_backend: KeyringBackend::Test,
    };

    keys(KeyCommand::Add(cmd))?;

    Ok(())
}
