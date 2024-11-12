use std::error::Error;
use std::fs::{self, File};
use std::io::Write;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};

use extensions::socket_addr;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tendermint::rpc::url::Url;

use crate::defaults::{CLIENT_CONFIG_FILE_NAME, CONFIG_DIR, CONFIG_FILE_NAME, GENESIS_FILE_NAME};
use crate::types::base::min_gas::MinGasPrices;

pub const DEFAULT_GRPC_LISTEN_ADDR: SocketAddr = socket_addr!(127, 0, 0, 1, 8080);
pub const DEFAULT_REST_LISTEN_ADDR: SocketAddr =
    SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 1317);
pub const DEFAULT_ADDRESS: SocketAddr =
    SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 26658);
pub const DEFAULT_TENDERMINT_RPC_ADDRESS: &str = "http://localhost:26657";

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum ConfigDirectory {
    GenesisFile,
    ConfigFile,
    ClientConfigFile,
    ConfigDir,
}

impl ConfigDirectory {
    pub fn path_from_home(&self, home: &(impl AsRef<Path> + ?Sized)) -> PathBuf {
        match self {
            ConfigDirectory::GenesisFile => home.as_ref().join(CONFIG_DIR).join(GENESIS_FILE_NAME),
            ConfigDirectory::ConfigFile => home.as_ref().join(CONFIG_DIR).join(CONFIG_FILE_NAME),
            ConfigDirectory::ClientConfigFile => {
                home.as_ref().join(CONFIG_DIR).join(CLIENT_CONFIG_FILE_NAME)
            }
            ConfigDirectory::ConfigDir => home.as_ref().join(CONFIG_DIR),
        }
    }
}

pub trait ApplicationConfig: Serialize + DeserializeOwned + Default + Clone {}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
#[serde(default)]
pub struct Config<AC: Default + Clone> {
    pub tendermint_rpc_address: Url, // TODO: change to HttpClientUrl when Serialize and Deserialize are implemented
    pub rest_listen_addr: SocketAddr,
    pub grpc_listen_addr: SocketAddr,
    pub address: SocketAddr,
    pub min_gas_prices: Option<MinGasPrices>,
    pub app_config: AC,
}

impl<AC: ApplicationConfig> Config<AC> {
    pub fn from_file(filename: PathBuf) -> Result<Config<AC>, Box<dyn Error>> {
        let s = fs::read_to_string(filename)?;
        Ok(toml::from_str(&s)?)
    }

    pub fn write_default(mut file: File) -> Result<(), Box<dyn Error>> {
        let mut handlebars = handlebars::Handlebars::new();
        handlebars
            .register_template_string("config", CONFIG_TEMPLATE)
            .expect("hard coded config template is valid");

        let cfg: Config<AC> = Config {
            min_gas_prices: Some(MinGasPrices::default()),
            ..Default::default()
        };

        let config = handlebars
            .render("config", &cfg)
            .expect("Config will always work with the CONFIG_TEMPLATE");

        let app_cfg = toml::to_string(&cfg.app_config)?;

        file.write_all(config.as_bytes())?;
        writeln!(file)?;
        writeln!(file, "[app_config]")?;
        file.write_all(app_cfg.as_bytes()).map_err(|e| e.into())
    }

    /// Clone itself with default `[app_state]` e.g. `app_config` field
    pub fn clone_with_default(&self) -> Self {
        Self {
            tendermint_rpc_address: self.tendermint_rpc_address.to_owned(),
            rest_listen_addr: self.rest_listen_addr.to_owned(),
            grpc_listen_addr: self.grpc_listen_addr.to_owned(),
            address: self.address.to_owned(),
            min_gas_prices: self.min_gas_prices.to_owned(),
            app_config: AC::default(),
        }
    }
}

impl<AC: ApplicationConfig> Default for Config<AC> {
    fn default() -> Config<AC> {
        Self {
            tendermint_rpc_address: DEFAULT_TENDERMINT_RPC_ADDRESS
                .parse()
                .expect("const should be valid"),
            rest_listen_addr: DEFAULT_REST_LISTEN_ADDR,
            address: DEFAULT_ADDRESS,
            app_config: AC::default(),
            min_gas_prices: None,
            grpc_listen_addr: DEFAULT_GRPC_LISTEN_ADDR,
        }
    }
}

const CONFIG_TEMPLATE: &str = r#"# This is a TOML config file.
# For more information, see https://github.com/toml-lang/toml

#######################################################################
###                   Main Base Config Options                      ###
#######################################################################

# ABCI application TCP socket address
address = "{{address}}"

# REST service TCP socket address
rest_listen_addr = "{{rest_listen_addr}}"

# GRPC service TCP socket address
grpc_listen_addr = "{{grpc_listen_addr}}"

# Tendermint node RPC proxy address
tendermint_rpc_address = "{{tendermint_rpc_address}}"

min_gas_prices = "{{min_gas_prices}}"
"#;
