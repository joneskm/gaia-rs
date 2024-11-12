use crate::application::handlers::node::ABCIHandler;
use crate::application::ApplicationInfo;
use crate::baseapp::options::NodeOptions;
use crate::baseapp::{BaseApp, NodeQueryHandler};
use crate::config::{ApplicationConfig, Config, ConfigDirectory};
use crate::grpc::run_grpc_server;
use crate::params::ParamsSubspaceKey;
use crate::rest::{run_rest_server, RestState};
use crate::types::base::min_gas::MinGasPrices;
use axum::Router;
use database::{Database, DatabaseBuilder};
use std::net::SocketAddr;
use std::path::PathBuf;
use tendermint::abci::ServerBuilder;
use tendermint::application::ABCI;
use tower_layer::Identity;
use tracing::metadata::LevelFilter;
use tracing::{error, info};

#[derive(Debug, Clone)]
pub struct RunCommand {
    pub home: PathBuf,
    pub address: Option<SocketAddr>,
    pub grpc_listen_addr: Option<SocketAddr>,
    pub rest_listen_addr: Option<SocketAddr>,
    pub tendermint_rpc_addr: Option<tendermint::rpc::url::Url>,
    pub read_buf_size: usize,
    pub log_level: LogLevel,
    pub min_gas_prices: Option<MinGasPrices>,
}

#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error("{0}")]
    HomeDirectory(String),
    #[error("{0}")]
    Database(String),
    #[error("{0}")]
    TendermintServer(#[from] tendermint::abci::errors::Error),
    #[error("{0}")]
    Custom(String),
    #[error("{0}")]
    TendermintRPC(#[from] tendermint::rpc::error::Error),
}

#[derive(Debug, Clone, Default, strum::Display)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
pub enum LogLevel {
    #[strum(to_string = "debug")]
    Debug,
    #[default]
    #[strum(to_string = "info")]
    Info,
    #[strum(to_string = "warn")]
    Warn,
    #[strum(to_string = "error")]
    Error,
    #[strum(to_string = "off")]
    Off,
}

impl From<LogLevel> for LevelFilter {
    fn from(value: LogLevel) -> Self {
        match value {
            LogLevel::Debug => Self::DEBUG,
            LogLevel::Info => Self::INFO,
            LogLevel::Warn => Self::WARN,
            LogLevel::Error => Self::ERROR,
            LogLevel::Off => Self::OFF,
        }
    }
}

pub trait RouterBuilder<QReq, QRes> {
    fn build_router<App: NodeQueryHandler<QReq, QRes>>(&self)
        -> Router<RestState<QReq, QRes, App>>;

    fn build_grpc_router<App: NodeQueryHandler<QReq, QRes>>(
        &self,
        app: App,
    ) -> tonic::transport::server::Router<Identity>;
}

/// Start your node
///
/// *Note*: tendermint should be started manually
pub fn run<
    DB: Database,
    DBO: DatabaseBuilder<DB>,
    PSK: ParamsSubspaceKey,
    H: ABCIHandler,
    AC: ApplicationConfig,
    AI: ApplicationInfo,
    RB: RouterBuilder<H::QReq, H::QRes>,
>(
    cmd: RunCommand,
    db_builder: DBO,
    params_subspace_key: PSK,
    abci_handler_builder: impl FnOnce(Config<AC>) -> H,
    router_builder: RB,
) -> Result<(), RunError> {
    let RunCommand {
        home,
        address,
        rest_listen_addr,
        grpc_listen_addr,
        read_buf_size,
        log_level,
        min_gas_prices,
        tendermint_rpc_addr: tendermint_addr,
    } = cmd;

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .try_init()
        .map_err(|e| RunError::Custom(format!("Failed to set logger: {}", e)))?;

    info!("Using directory {} for config and data", home.display());

    let db_dir = home.join("data");
    let db = db_builder
        .build(db_dir.join("application.db"))
        .map_err(|e| RunError::Database(format!("{e:?}")))?;

    let cfg_file_path = ConfigDirectory::ConfigFile.path_from_home(&home);

    let config: Config<AC> = Config::from_file(cfg_file_path)
        .map_err(|e| RunError::Custom(format!("Error reading config file: {:?}", e)))?;

    let abci_handler = abci_handler_builder(config.clone());

    let options = NodeOptions::new(min_gas_prices.or(config.min_gas_prices).ok_or(
        RunError::HomeDirectory(
            "Failed to get `min_gas_prices` set it via cli or in config file".to_owned(),
        ),
    )?);

    let app: BaseApp<DB, PSK, H, AI> = BaseApp::new(db, params_subspace_key, abci_handler, options);

    run_rest_server::<H::Message, H::QReq, H::QRes, _>(
        app.clone(),
        rest_listen_addr.unwrap_or(config.rest_listen_addr),
        router_builder.build_router::<BaseApp<DB, PSK, H, AI>>(),
        tendermint_addr
            .unwrap_or(config.tendermint_rpc_address)
            .try_into()?,
    );

    run_grpc_server(
        router_builder.build_grpc_router::<BaseApp<DB, PSK, H, AI>>(app.clone()),
        grpc_listen_addr.unwrap_or(config.grpc_listen_addr),
    );

    let addr = address.unwrap_or(config.address);

    let server = ServerBuilder::new(read_buf_size).bind(addr, ABCI::from(app))?;

    info!("Starting proxy server at: {}", addr.to_string());

    server.listen().map_err(|e| e.into())
}
