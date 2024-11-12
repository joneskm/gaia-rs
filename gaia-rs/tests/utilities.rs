//! This modules should be added to test modules with `#[path = "./utilities.rs"]` as it contains gaia specific code and dedicated crate is bothersome.
#![allow(dead_code)]

use std::{
    net::{Ipv4Addr, SocketAddr},
    ops::Deref,
    str::FromStr,
};

use gaia_rs::{
    client::{GaiaQueryCommands, GaiaTxCommands, WrappedGaiaQueryCommands, WrappedGaiaTxCommands},
    query::GaiaQueryResponse,
    GaiaCoreClient, QueryNodeFetcher,
};
use gears::{
    commands::{
        client::{
            query::QueryCommand,
            tx::{ClientTxContext, RuntxResult, TxCommand},
        },
        node::run::{LogLevel, RunCommand},
    },
    extensions::testing::UnwrapTesting,
    types::address::AccAddress,
    utils::tendermint::{random_port, TendermintSubprocess},
};
use vec1::Vec1;

/// List of all accounts saved into genesis file - alice is validator
/// key name | account address | mnemonic
const ACCOUNTS_LIST  : [(&str, &str, &str); 11] =  [
    ("alice",  "cosmos1syavy2npfyt9tcncdtsdzf7kny9lh777pahuux", "race draft rival universe maid cheese steel logic crowd fork comic easy truth drift tomorrow eye buddy head time cash swing swift midnight borrow"),
    ("alice0", "cosmos18mjeafdpsgdgu0tmt39zggx20h7994hpd6wkpx", "same season present whip glass cargo fiber volume exit bracket gentle wish umbrella honey grab grace rigid mix credit route morning doll enemy wire"),
    ("alice1", "cosmos1nxyyhwcfzzptad0trg6suh6j650npjlxxc73ay", "cute oppose want human pact ecology pig climb spider miracle local gentle title odor swamp liar other wage cheap wage barrel salute addict brick"),
    ("alice2", "cosmos1ks9zdcycywxfcj880m3rlyctsfhfsh87fc7cm3", "ten despair spin toward soap brown obvious border episode arrow ice alley wet swim monitor allow smile check bind raw coast base wing antenna"),
    ("alice3", "cosmos1luuusx6cura35dl4ztezd9x2pjea3js0xvvmc8", "lunch mosquito rice flag green laundry round solve kitten empower gold subway security warrior humor accident maze small party ship velvet note balcony suggest"),
    ("alice4", "cosmos1mf4pu0zmzlng5rn8m0nmju9auxujns6jswcxwr", "debris six path vintage whisper parrot novel toward select fresh bachelor turn loyal walk elite order festival model birth strike evolve diary lady suggest"),
    ("alice5", "cosmos15rteevxs5vsxfaj5qukh0hhuecgkel8s45g4dt", "impose super rich deliver roast robust caught toe gift vessel alter danger final found goose phone remind custom pigeon harvest blind monitor sight noble"),
    ("alice6", "cosmos1ly29f22fpeqsafa03tpnf7huyewv9vl2dvamxp", "myself caution entire tumble movie exist drama tray carbon cheap brand tobacco beach web puppy breeze shaft imitate rail expect mixed expose space project"),
    ("alice7", "cosmos1f78mzqldvqmpxxf5xf0354hu3cx7n3xrptyql9", "setup census bronze spoil swarm corn note misery lemon eyebrow fit canvas until cereal link fit endorse rookie ski flower stereo oxygen lumber follow"),
    ("alice8", "cosmos1kn7ccmk9k2q024l66efw39mat4ys2cvxc2kh67", "voyage wealth dust general alter rare puppy exist when taste fit inflict since combine offer awesome artist cereal fiction glue same general seminar sorry"),
    ("alice9", "cosmos19k5n7f35e4tskjcm2peujta0e3rvszmmzlhjej", "moon meat day town sugar matrix coffee lamp metal output fever document crush forum noise pear question cycle surprise wasp enough achieve initial shop"),
];

#[derive(Debug)]
pub struct GaiaNode {
    tendermint: TendermintSubprocess,
    gaia_handler: std::thread::JoinHandle<()>,
    pub proxy_addr: SocketAddr,
    pub grpc_addr: SocketAddr,
    pub rest_addr: SocketAddr,
    pub rpc_addr: url::Url,
}

impl GaiaNode {
    pub fn validator_account() -> AccAddress {
        let (_, addr, _) = ACCOUNTS_LIST[0];
        AccAddress::from_bech32(addr).unwrap_test()
    }

    pub fn validator_key() -> &'static str {
        let (key, _, _) = ACCOUNTS_LIST[0];
        key
    }

    pub fn accounts() -> Vec1<(&'static str, AccAddress)> {
        let mut result = Vec::with_capacity(11);

        for (key, addr, _) in ACCOUNTS_LIST {
            result.push((key, AccAddress::from_bech32(addr).unwrap_test()));
        }

        result.try_into().expect("not empty")
    }

    pub fn run() -> anyhow::Result<Self> {
        const TENDERMINT_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/assets");
        const LOG_LEVEL: LogLevel = LogLevel::Off;

        let tendermint = TendermintSubprocess::run(TENDERMINT_PATH, LOG_LEVEL)?;
        let tmp_path = tendermint.home();

        let proxy_addr = SocketAddr::new(
            std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            tendermint.proxy_port,
        );

        let rest_addr = SocketAddr::new(
            std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            random_port(),
        );

        let grpc_addr = SocketAddr::new(
            std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            random_port(),
        );

        let rpc_addr = url::Url::from_str(&format!("http://localhost:{}", tendermint.rpc_port))?;

        let rpc_addr_moved = rpc_addr.clone();
        let server_thread = std::thread::spawn(move || {
            let node = gears::application::node::NodeApplication::<
                gaia_rs::GaiaCore,
                gears::store::database::sled::SledDb,
                _,
                _,
            >::new(
                gaia_rs::GaiaCore,
                gears::store::database::DBBuilder,
                gaia_rs::abci_handler::GaiaABCIHandler::new,
                gaia_rs::store_keys::GaiaParamsStoreKey::BaseApp,
            );

            let cmd = RunCommand {
                home: tmp_path,
                address: Some(proxy_addr),
                rest_listen_addr: Some(rest_addr),
                grpc_listen_addr: Some(grpc_addr),
                read_buf_size: 1048576,
                log_level: LOG_LEVEL,
                min_gas_prices: Default::default(),
                tendermint_rpc_addr: Some(rpc_addr_moved.try_into().expect("invalid rpc addr")),
            };

            let _ = node
                .execute::<gaia_rs::GaiaApplication>(gears::commands::node::AppCommands::Run(cmd));
        });

        std::thread::sleep(std::time::Duration::from_secs(10));

        Ok(Self {
            tendermint,
            gaia_handler: server_thread,
            proxy_addr,
            grpc_addr,
            rest_addr,
            rpc_addr,
        })
    }

    pub fn query(&self, cmd: GaiaQueryCommands) -> anyhow::Result<GaiaQueryResponse> {
        let cmd = QueryCommand {
            node: self.rpc_addr.clone(),
            height: None,
            inner: WrappedGaiaQueryCommands(cmd),
        };

        let result = gears::commands::client::query::run_query(cmd, &gaia_rs::GaiaCoreClient)?;

        Ok(result)
    }

    pub fn tx(&self, cmd: GaiaTxCommands, key: &str) -> anyhow::Result<RuntxResult> {
        let responses = gears::commands::client::tx::run_tx(
            TxCommand {
                ctx: ClientTxContext::new_online(
                    self.tendermint.home(),
                    200_000_u32.into(),
                    self.rpc_addr.clone().try_into().expect("invalid addr"),
                    self.tendermint.chain_id.clone(),
                    key,
                ),
                inner: WrappedGaiaTxCommands(cmd),
            },
            &GaiaCoreClient,
            &QueryNodeFetcher,
        )?;

        Ok(responses)
    }
}

impl Deref for GaiaNode {
    type Target = TendermintSubprocess;

    fn deref(&self) -> &Self::Target {
        &self.tendermint
    }
}
