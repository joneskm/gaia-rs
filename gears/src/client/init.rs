use std::path::PathBuf;

use clap::{arg, value_parser, Arg, ArgAction, ArgMatches, Command};
use log::info;
use serde::Serialize;
use tendermint::informal::chain::Id;

use crate::{config::ApplicationConfig, utils::get_default_home_dir};

pub fn get_init_command(app_name: &str) -> Command {
    Command::new("init")
        .about("Initialize configuration files")
        .arg(Arg::new("moniker").required(true))
        .arg(
            arg!(--home)
                .help(format!(
                    "Directory for config and data [default: {}]",
                    get_default_home_dir(app_name).unwrap_or_default().display()
                ))
                .action(ArgAction::Set)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("chain-id")
                .long("chain-id")
                .help("Genesis file chain-id")
                .default_value("test-chain")
                .action(ArgAction::Set)
                .value_parser(value_parser!(Id)),
        )
}

#[derive(Debug, thiserror::Error)]
pub enum InitError {
    // TODO: reduce error count
    #[error("Could not create config directory {0}")]
    CreateConfigDirectory(#[source] std::io::Error),
    #[error("Could not create data directory {0}")]
    CreateDataDirectory(#[source] std::io::Error),
    #[error("Could not create config file {0}")]
    CreateConfigFile(#[source] std::io::Error),
    #[error("Error writing config file {0}")]
    WriteConfigFile(#[source] tendermint::error::Error),
    #[error("{0}")]
    WriteDefaultConfigFile(String),
    #[error("Could not create node key file {0}")]
    CreateNodeKeyFile(#[source] std::io::Error),
    #[error("Could not create private validator key file {0}")]
    PrivValidatorKey(#[source] std::io::Error),
    #[error("Error writing private validator state file {0}")]
    WritePrivValidatorKey(#[source] tendermint::error::Error),
    #[error("{0}")]
    Deserialize(#[from] serde_json::Error),
    #[error("Could not create genesis file {0}")]
    CreateGenesisFile(#[source] std::io::Error),
    #[error("Could not create config file {0}")]
    CreateConfigError(#[source] std::io::Error),
    #[error("Error writing config file {0}")]
    WriteConfigError(#[source] std::io::Error),
    #[error("Error writing key and genesis files {0}")]
    WriteKeysAndGenesis(#[source] tendermint::error::Error),
}

#[derive(Debug, Clone, derive_builder::Builder)]
// #[cfg_attr( // TODO: Ask Kevin which aprroach he prefer
//     all(feature = "cli"),
//     derive(::clap::Args)
// )]
pub struct InitOptions {
    pub home: PathBuf,
    pub moniker: String,
    pub chain_id: Id,
}

pub const DEFAULT_DIR_NAME: &str = env!("CARGO_PKG_NAME");

fn default_home() -> Option<PathBuf> {
    Some(dirs::home_dir()?.join(DEFAULT_DIR_NAME))
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("{0}")]
pub struct InitOptionsParseError(pub String);
impl TryFrom<&ArgMatches> for InitOptions {
    type Error = InitOptionsParseError;

    fn try_from(value: &ArgMatches) -> Result<Self, Self::Error> {
        let moniker = value
            .get_one::<String>("moniker")
            .ok_or(InitOptionsParseError(
                "moniker argument is required preventing `None`".to_owned(),
            ))?
            .clone();

        let home = value
            .get_one::<PathBuf>("home")
            .cloned()
            .or(default_home())
            .ok_or(InitOptionsParseError(
                "Home argument not provided and OS does not provide a default home directory"
                    .to_owned(),
            ))?;

        let chain_id = value
            .get_one::<Id>("chain-id")
            .ok_or(InitOptionsParseError(
                "has a default value so will never be None".to_owned(),
            ))?
            .clone();

        Ok(Self {
            home,
            moniker,
            chain_id,
        })
    }
}

pub fn init<G: Serialize, AC: ApplicationConfig>(
    opt: InitOptions,
    app_genesis_state: &G,
) -> Result<(), InitError> {
    let InitOptions {
        moniker,
        home,
        chain_id,
    } = opt;

    // Create config directory
    let config_dir = home.join("config");
    std::fs::create_dir_all(&config_dir).map_err(|e| InitError::CreateConfigDirectory(e))?;

    // Create data directory
    let data_dir = home.join("data");
    std::fs::create_dir_all(&data_dir).map_err(|e| InitError::CreateDataDirectory(e))?;

    // Write tendermint config file
    let tm_config_file_path = config_dir.join("config.toml");
    let tm_config_file = std::fs::File::create(&tm_config_file_path)
        .map_err(|e| InitError::CreateConfigDirectory(e))?;

    tendermint::write_tm_config(tm_config_file, &moniker)
        .map_err(|e| InitError::WriteConfigFile(e))?;

    info!(
        "Tendermint config written to {}",
        tm_config_file_path.display()
    );

    // Create node key file
    let node_key_file_path = config_dir.join("node_key.json");
    let node_key_file =
        std::fs::File::create(&node_key_file_path).map_err(|e| InitError::CreateNodeKeyFile(e))?;

    // Create private validator key file
    let priv_validator_key_file_path = config_dir.join("priv_validator_key.json");
    let priv_validator_key_file = std::fs::File::create(&priv_validator_key_file_path)
        .map_err(|e| InitError::PrivValidatorKey(e))?;

    let app_state = serde_json::to_value(app_genesis_state)?;

    // Create genesis file
    let mut genesis_file_path = home.clone();
    crate::utils::get_genesis_file_from_home_dir(&mut genesis_file_path);
    let genesis_file =
        std::fs::File::create(&genesis_file_path).map_err(|e| InitError::CreateGenesisFile(e))?;

    // Create config file
    let mut cfg_file_path = home.clone();
    crate::utils::get_config_file_from_home_dir(&mut cfg_file_path);
    let cfg_file =
        std::fs::File::create(&cfg_file_path).map_err(|e| InitError::CreateConfigFile(e))?;

    crate::config::Config::<AC>::write_default(cfg_file)
        .map_err(|e| InitError::WriteDefaultConfigFile(e.to_string()))?;

    info!("Config file written to {}", cfg_file_path.display());

    // Write key and genesis
    tendermint::write_keys_and_genesis(
        node_key_file,
        priv_validator_key_file,
        genesis_file,
        app_state,
        chain_id,
    )
    .map_err(|e| InitError::WriteKeysAndGenesis(e))?;

    info!(
        "Key files written to {} and {}",
        node_key_file_path.display(),
        priv_validator_key_file_path.display()
    );
    info!("Genesis file written to {}", genesis_file_path.display());

    // Write private validator state file
    let state_file_path = data_dir.join("priv_validator_state.json");
    let state_file =
        std::fs::File::create(&state_file_path).map_err(|e| InitError::PrivValidatorKey(e))?;

    tendermint::write_priv_validator_state(state_file)
        .map_err(|e| InitError::WritePrivValidatorKey(e))?;

    info!(
        "Private validator state written to {}",
        state_file_path.display()
    );

    Ok(())
}
