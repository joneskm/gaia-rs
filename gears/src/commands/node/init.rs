use std::path::PathBuf;

use serde::Serialize;
use tendermint::types::chain_id::ChainId;

use crate::config::{ApplicationConfig, ConfigDirectory};

#[derive(Debug, Clone, former::Former)]
pub struct InitCommand {
    pub home: PathBuf,
    pub moniker: String,
    pub chain_id: ChainId,
}

/// Init application configuration like `tendermint` config, genesis file
pub fn init<G: Serialize, AC: ApplicationConfig>(
    cmd: InitCommand,
    app_genesis_state: &G,
) -> Result<(), InitError> {
    let InitCommand {
        moniker,
        home,
        chain_id,
    } = cmd;

    // Create config directory
    let config_dir = home.join("config");
    std::fs::create_dir_all(&config_dir).map_err(InitError::CreateConfigDirectory)?;

    // Create data directory
    let data_dir = home.join("data");
    std::fs::create_dir_all(&data_dir).map_err(InitError::CreateDataDirectory)?;

    // Write tendermint config file
    let tm_config_file_path = config_dir.join("config.toml");
    let tm_config_file =
        std::fs::File::create(&tm_config_file_path).map_err(InitError::CreateConfigDirectory)?;

    tendermint::write_tm_config(tm_config_file, &moniker).map_err(InitError::WriteConfigFile)?;

    #[cfg(not(feature = "utils"))]
    println!("Tendermint config written to {tm_config_file_path:?}");

    // Create node key file
    let node_key_file_path = config_dir.join("node_key.json");
    let node_key_file =
        std::fs::File::create(&node_key_file_path).map_err(InitError::CreateNodeKeyFile)?;

    // Create private validator key file
    let priv_validator_key_file_path = config_dir.join("priv_validator_key.json");
    let priv_validator_key_file = std::fs::File::create(&priv_validator_key_file_path)
        .map_err(InitError::PrivValidatorKey)?;

    let app_state = serde_json::to_value(app_genesis_state)?;

    // Create genesis file
    let genesis_file_path = ConfigDirectory::GenesisFile.path_from_home(&home);
    let genesis_file =
        std::fs::File::create(&genesis_file_path).map_err(InitError::CreateGenesisFile)?;

    // Create config file
    let cfg_file_path = ConfigDirectory::ConfigFile.path_from_home(&home);
    let cfg_file = std::fs::File::create(&cfg_file_path).map_err(InitError::CreateConfigFile)?;

    crate::config::Config::<AC>::write_default(cfg_file)
        .map_err(|e| InitError::WriteDefaultConfigFile(e.to_string()))?;

    #[cfg(not(feature = "utils"))]
    println!("Config file written to {}", cfg_file_path.display());

    // Write key and genesis
    tendermint::write_keys_and_genesis(
        node_key_file,
        priv_validator_key_file,
        genesis_file,
        app_state,
        chain_id,
    )
    .map_err(InitError::WriteKeysAndGenesis)?;

    #[cfg(not(feature = "utils"))]
    println!(
        "Key files written to {} and {}",
        node_key_file_path.display(),
        priv_validator_key_file_path.display()
    );
    #[cfg(not(feature = "utils"))]
    println!("Genesis file written to {}", genesis_file_path.display());

    // Write private validator state file
    let state_file_path = data_dir.join("priv_validator_state.json");
    let state_file =
        std::fs::File::create(&state_file_path).map_err(InitError::PrivValidatorKey)?;

    tendermint::write_priv_validator_state(state_file).map_err(InitError::WritePrivValidatorKey)?;

    #[cfg(not(feature = "utils"))]
    println!(
        "Private validator state written to {}",
        state_file_path.display()
    );

    Ok(())
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
