#![allow(missing_docs)]

use error::Error;
use std::{fs::File, io::Write, path::PathBuf, time::Duration};
use tendermint_config::{
    AbciMode, ConsensusConfig, CorsHeader, CorsMethod, DbBackend, FastsyncConfig,
    InstrumentationConfig, LogFormat, MempoolConfig, NodeKey, P2PConfig, PrivValidatorKey,
    RpcConfig, StatesyncConfig, StorageConfig, TendermintConfig, TransferRate, TxIndexConfig,
    TxIndexer,
};
use types::{chain_id::ChainId, proto::crypto::PublicKey};

pub mod abci;
pub mod application;
pub mod crypto;
pub mod error;
pub(crate) mod ext;
pub mod informal;
pub mod public;
pub mod rpc;
pub mod types;

//TODO: comma separated list fields; check all "serialize_comma_separated_list" in TendermintConfig
//TODO: expose write_tm_config_file args

pub fn get_validator_pub_key(priv_validator_key_file: File) -> Result<PublicKey, Error> {
    let priv_validator_key: PrivValidatorKey = serde_json::from_reader(priv_validator_key_file)?;
    let pub_key = priv_validator_key.pub_key.try_into()?;
    Ok(pub_key)
}

pub fn node_pub_key(node_key_file: File) -> Result<PublicKey, Error> {
    let priv_validator_key: NodeKey = serde_json::from_reader(node_key_file)?;
    let pub_key = priv_validator_key.public_key().try_into()?;
    Ok(pub_key)
}

pub fn write_keys_and_genesis(
    mut node_key_file: File,
    mut priv_validator_key_file: File,
    mut genesis_file: File,
    app_state: serde_json::Value, //TODO: make this a generic
    chain_id: ChainId,
) -> Result<(), Error> {
    // write node key
    let priv_key = crypto::new_private_key();
    let node_key = NodeKey { priv_key };
    node_key_file.write_all(
        serde_json::to_string_pretty(&node_key)
            .expect("NodeKey structure serialization will always succeed")
            .as_bytes(),
    )?;

    // write node private validator key
    let priv_key = crypto::new_private_key();
    let address: tendermint_informal::account::Id = priv_key.public_key().into();
    let priv_validator_key = PrivValidatorKey {
        address,
        pub_key: priv_key.public_key(),
        priv_key,
    };
    priv_validator_key_file.write_all(
        serde_json::to_string_pretty(&priv_validator_key)
            .expect("PrivValidatorKey structure serialization will always succeed")
            .as_bytes(),
    )?;

    // write genesis file
    // TODO: create a Genesis struct in this crate and define a default
    let genesis = tendermint_informal::Genesis {
        genesis_time: tendermint_informal::Time::now(),
        chain_id: chain_id.into(),
        initial_height: 1,
        consensus_params: tendermint_informal::consensus::Params {
            block: tendermint_informal::block::Size {
                max_bytes: 22020096,
                max_gas: -1,
                time_iota_ms: 1000,
            },
            evidence: tendermint_informal::evidence::Params {
                max_age_num_blocks: 100000,
                max_age_duration: tendermint_informal::evidence::Duration(Duration::new(172800, 0)),
                max_bytes: 1048576,
            },
            validator: tendermint_informal::consensus::params::ValidatorParams {
                pub_key_types: vec![tendermint_informal::public_key::Algorithm::Ed25519],
            },
            version: None,
        },
        validators: vec![],
        app_hash: vec![].try_into().expect("infallible"),
        app_state,
    };

    genesis_file
        .write_all(
            serde_json::to_string_pretty(&genesis)
                .expect("Genesis structure serialization will always succeed")
                .as_bytes(),
        )
        .map_err(|e| e.into())
}

pub fn write_priv_validator_state(mut priv_validator_state_key_file: File) -> Result<(), Error> {
    // This is what this code should do. However there's a bug in Tendermint-rs which means the round
    // gets written as a string. This causes Tendermint to fail. Instead we use a hard coded string
    // let state = State {
    //     height: 0u32.into(),
    //     round: 0u8.into(),
    //     step: 0,
    //     block_id: None,
    // };

    // priv_validator_state_key_file
    //     .write_all(
    //         serde_json::to_string_pretty(&state)
    //             .expect("State structure serialization will always succeed")
    //             .as_bytes(),
    //     )
    //     .map_err(|e| e.into())

    let state = r#"{
    "height": "0",
    "round": 0,
    "step": 0
}"#;

    priv_validator_state_key_file
        .write_all(state.as_bytes())
        .map_err(|e| e.into())
}

pub fn write_tm_config(
    mut file: File,
    node_name: &str,
    // peers: Vec<Address>,
    // external_address: Option<Address>,
    // tm_rpc_bind: Option<SocketAddr>,
    // tm_p2p_bind: Option<SocketAddr>,
) -> Result<(), Error> {
    let mut handlebars = handlebars::Handlebars::new();
    handlebars
        .register_template_string("config", TM_CONFIG_TEMPLATE)
        .expect("hard coded config template is valid");

    let mut tm_config = get_default_tm_config();
    tm_config.moniker = node_name
        .parse()
        .expect("the Moniker::from_str method never fails");

    let tm_config = handlebars
        .render("config", &tm_config)
        .expect("TendermintConfig will always work with the TM_CONFIG_TEMPLATE");

    file.write_all(tm_config.as_bytes()).map_err(|e| e.into())
}

// TODO implement Default
fn get_default_tm_config() -> TendermintConfig {
    TendermintConfig {
        proxy_app: "tcp://127.0.0.1:26658"
            .parse()
            .expect("hard coded address is valid"),
        moniker: "anonymous".parse().expect("hard coded moniker is valid"),
        fast_sync: true,
        db_backend: DbBackend::GoLevelDb,
        db_dir: PathBuf::from("data"),
        log_level: "info".parse().expect("hard coded log level is valid"),
        log_format: LogFormat::Plain,
        genesis_file: PathBuf::from("config/genesis.json"),
        priv_validator_key_file: Some(PathBuf::from("config/priv_validator_key.json")),
        priv_validator_state_file: PathBuf::from("data/priv_validator_state.json"),
        priv_validator_laddr: None,
        node_key_file: PathBuf::from("config/node_key.json"),
        abci: AbciMode::Socket,
        filter_peers: false,
        rpc: RpcConfig {
            laddr: "tcp://127.0.0.1:26657"
                .parse()
                .expect("hard coded address is valid"),
            cors_allowed_origins: vec![],
            cors_allowed_methods: get_default_cors_allowed_methods(),
            cors_allowed_headers: get_default_cors_allowed_headers(),
            grpc_laddr: None,
            grpc_max_open_connections: 900,
            unsafe_commands: false,
            max_open_connections: 900,
            max_subscription_clients: 100,
            max_subscriptions_per_client: 5,
            timeout_broadcast_tx_commit: "10000ms".parse().expect("hard coded timeout is valid"),
            max_body_bytes: 1000000,
            max_header_bytes: 1048576,
            tls_cert_file: None,
            tls_key_file: None,
            pprof_laddr: None,
        },
        p2p: P2PConfig {
            laddr: "tcp://0.0.0.0:26656"
                .parse()
                .expect("hard coded address is valid"),
            external_address: None,
            seeds: vec![],
            persistent_peers: vec![],
            upnp: false,
            addr_book_file: PathBuf::from("config/addrbook.json"),
            addr_book_strict: true,
            max_num_inbound_peers: 40,
            max_num_outbound_peers: 10,
            unconditional_peer_ids: vec![],
            persistent_peers_max_dial_period: "0ms".parse().expect("hard coded timeout is valid"),
            flush_throttle_timeout: "100ms".parse().expect("hard coded timeout is valid"),
            max_packet_msg_payload_size: 1024,
            send_rate: new_transfer_rate(5120000),
            recv_rate: new_transfer_rate(5120000),
            pex: true,
            seed_mode: false,
            private_peer_ids: vec![],
            allow_duplicate_ip: false,
            handshake_timeout: "20000ms".parse().expect("hard coded timeout is valid"),
            dial_timeout: "3000ms".parse().expect("hard coded timeout is valid"),
        },
        mempool: MempoolConfig {
            recheck: true,
            broadcast: true,
            wal_dir: None,
            size: 5000,
            max_txs_bytes: 1073741824,
            cache_size: 10000,
            keep_invalid_txs_in_cache: false,
            max_tx_bytes: 1048576,
            max_batch_bytes: 0,
        },
        consensus: ConsensusConfig {
            wal_file: PathBuf::from("data/cs.wal/wal"),
            timeout_propose: "3000ms".parse().expect("hard coded timeout is valid"),
            timeout_propose_delta: "500ms".parse().expect("hard coded timeout is valid"),
            timeout_prevote: "1000ms".parse().expect("hard coded timeout is valid"),
            timeout_prevote_delta: "500ms".parse().expect("hard coded timeout is valid"),
            timeout_precommit: "1000ms".parse().expect("hard coded timeout is valid"),
            timeout_precommit_delta: "500ms".parse().expect("hard coded timeout is valid"),
            timeout_commit: "5000ms".parse().expect("hard coded timeout is valid"),
            double_sign_check_height: 0,
            skip_timeout_commit: false,
            create_empty_blocks: true,
            create_empty_blocks_interval: "0ms".parse().expect("hard coded timeout is valid"),
            peer_gossip_sleep_duration: "100ms".parse().expect("hard coded timeout is valid"),
            peer_query_maj23_sleep_duration: "2000ms".parse().expect("hard coded timeout is valid"),
        },
        storage: StorageConfig {
            discard_abci_responses: false,
        },
        tx_index: TxIndexConfig {
            indexer: TxIndexer::Kv,
        },
        instrumentation: InstrumentationConfig {
            prometheus: false,
            prometheus_listen_addr: ":26660".into(),
            max_open_connections: 3,
            namespace: "tendermint".into(),
        },
        statesync: StatesyncConfig {
            enable: false,
            rpc_servers: vec![],
            trust_height: 0,
            trust_hash: "".into(),
            trust_period: "168h0m0s".parse().expect("hard coded timeout is valid"),
            discovery_time: "15000ms".parse().expect("hard coded timeout is valid"),
            temp_dir: "".into(),
        },
        fastsync: FastsyncConfig {
            version: "v0".into(),
        },
    }
}

/// This method is needed since there doesn't seem to be a way to construct
/// a CorsMethod directly
fn get_default_cors_allowed_methods() -> Vec<CorsMethod> {
    serde_json::from_str(r#"["HEAD", "GET", "POST"]"#).expect("hard coded cors methods are valid")
}

/// This method is needed since there doesn't seem to be a way to construct
/// a CorsHeader directly
fn get_default_cors_allowed_headers() -> Vec<CorsHeader> {
    serde_json::from_str(
        r#"["Origin", "Accept", "Content-Type", "X-Requested-With", "X-Server-Time"]"#,
    )
    .expect("hard coded cors headers are valid")
}

/// This method is needed since there doesn't seem to be a way to construct
/// a TransferRate directly
fn new_transfer_rate(rate: u64) -> TransferRate {
    serde_json::from_str(&rate.to_string()).expect("will always succeed")
}

const TM_CONFIG_TEMPLATE: &str = r#"# This is a TOML config file.
# For more information, see https://github.com/toml-lang/toml

# NOTE: Any path below can be absolute (e.g. "/var/myawesomeapp/data") or
# relative to the home directory (e.g. "data"). The home directory is
# "$HOME/.tendermint" by default, but could be changed via $TMHOME env variable
# or --home cmd flag.

#######################################################################
###                   Main Base Config Options                      ###
#######################################################################

# TCP or UNIX socket address of the ABCI application,
# or the name of an ABCI application compiled in with the Tendermint binary
proxy_app = "{{ proxy_app }}"

# A custom human readable name for this node
moniker = "{{ moniker }}"

# If this node is many blocks behind the tip of the chain, FastSync
# allows them to catchup quickly by downloading blocks in parallel
# and verifying their commits
fast_sync = {{ fast_sync }}

# Database backend: goleveldb | cleveldb | boltdb | rocksdb | badgerdb
# * goleveldb (github.com/syndtr/goleveldb - most popular implementation)
#   - pure go
#   - stable
# * cleveldb (uses levigo wrapper)
#   - fast
#   - requires gcc
#   - use cleveldb build tag (go build -tags cleveldb)
# * boltdb (uses etcd's fork of bolt - github.com/etcd-io/bbolt)
#   - EXPERIMENTAL
#   - may be faster is some use-cases (random reads - indexer)
#   - use boltdb build tag (go build -tags boltdb)
# * rocksdb (uses github.com/tecbot/gorocksdb)
#   - EXPERIMENTAL
#   - requires gcc
#   - use rocksdb build tag (go build -tags rocksdb)
# * badgerdb (uses github.com/dgraph-io/badger)
#   - EXPERIMENTAL
#   - use badgerdb build tag (go build -tags badgerdb)
db_backend = "{{ db_backend }}"

# Database directory
db_dir = "{{ db_dir  }}"

# Output level for logging, including package level options
log_level = "{{ log_level  }}"

# Output format: 'plain' (colored text) or 'json'
log_format = "{{ log_format }}"

##### additional base config options #####

# Path to the JSON file containing the initial validator set and other meta data
genesis_file = "{{ genesis_file }}"

# Path to the JSON file containing the private key to use as a validator in the consensus protocol
priv_validator_key_file = "{{ priv_validator_key_file }}"

# Path to the JSON file containing the last sign state of a validator
priv_validator_state_file = "{{ priv_validator_state_file }}"

# TCP or UNIX socket address for Tendermint to listen on for
# connections from an external PrivValidator process
priv_validator_laddr = "{{ priv_validator_laddr }}"

# Path to the JSON file containing the private key to use for node authentication in the p2p protocol
node_key_file = "{{ node_key_file }}"

# Mechanism to connect to the ABCI application: socket | grpc
abci = "{{ abci }}"

# If true, query the ABCI app on connecting to a new peer
# so the app can decide if we should keep the connection or not
filter_peers = {{ filter_peers }}


#######################################################################
###                 Advanced Configuration Options                  ###
#######################################################################

#######################################################
###       RPC Server Configuration Options          ###
#######################################################
[rpc]

# TCP or UNIX socket address for the RPC server to listen on
laddr = "{{ rpc.laddr }}"

# A list of origins a cross-domain request can be executed from
# Default value '[]' disables cors support
# Use '["*"]' to allow any origin
cors_allowed_origins = [{{#each rpc.cors_allowed_origins}}{{#if @index}},{{/if}}"{{this}}"{{/each}}]

# A list of methods the client is allowed to use with cross-domain requests
cors_allowed_methods = [{{#each rpc.cors_allowed_methods}}{{#if @index}},{{/if}}"{{this}}"{{/each}}]

# A list of non simple headers the client is allowed to use with cross-domain requests
cors_allowed_headers = [{{#each rpc.cors_allowed_headers}}{{#if @index}},{{/if}}"{{this}}"{{/each}}]

# TCP or UNIX socket address for the gRPC server to listen on
# NOTE: This server only supports /broadcast_tx_commit
grpc_laddr = "{{ rpc.grpc_laddr }}"

# Maximum number of simultaneous connections.
# Does not include RPC (HTTP&WebSocket) connections. See max_open_connections
# If you want to accept a larger number than the default, make sure
# you increase your OS limits.
# 0 - unlimited.
# Should be < {ulimit -Sn} - {MaxNumInboundPeers} - {MaxNumOutboundPeers} - {N of wal, db and other open files}
# 1024 - 40 - 10 - 50 = 924 = ~900
grpc_max_open_connections = {{ rpc.grpc_max_open_connections }}

# Activate unsafe RPC commands like /dial_seeds and /unsafe_flush_mempool
unsafe = {{ rpc.unsafe }}

# Maximum number of simultaneous connections (including WebSocket).
# Does not include gRPC connections. See grpc_max_open_connections
# If you want to accept a larger number than the default, make sure
# you increase your OS limits.
# 0 - unlimited.
# Should be < {ulimit -Sn} - {MaxNumInboundPeers} - {MaxNumOutboundPeers} - {N of wal, db and other open files}
# 1024 - 40 - 10 - 50 = 924 = ~900
max_open_connections = {{ rpc.max_open_connections }}

# Maximum number of unique clientIDs that can /subscribe
# If you're using /broadcast_tx_commit, set to the estimated maximum number
# of broadcast_tx_commit calls per block.
max_subscription_clients = {{ rpc.max_subscription_clients }}

# Maximum number of unique queries a given client can /subscribe to
# If you're using GRPC (or Local RPC client) and /broadcast_tx_commit, set to
# the estimated # maximum number of broadcast_tx_commit calls per block.
max_subscriptions_per_client = {{ rpc.max_subscriptions_per_client }}

# Experimental parameter to specify the maximum number of events a node will
# buffer, per subscription, before returning an error and closing the
# subscription. Must be set to at least 100, but higher values will accommodate
# higher event throughput rates (and will use more memory).
experimental_subscription_buffer_size = 200

# Experimental parameter to specify the maximum number of RPC responses that
# can be buffered per WebSocket client. If clients cannot read from the
# WebSocket endpoint fast enough, they will be disconnected, so increasing this
# parameter may reduce the chances of them being disconnected (but will cause
# the node to use more memory).
#
# Must be at least the same as "experimental_subscription_buffer_size",
# otherwise connections could be dropped unnecessarily. This value should
# ideally be somewhat higher than "experimental_subscription_buffer_size" to
# accommodate non-subscription-related RPC responses.
experimental_websocket_write_buffer_size = 200

# If a WebSocket client cannot read fast enough, at present we may
# silently drop events instead of generating an error or disconnecting the
# client.
#
# Enabling this experimental parameter will cause the WebSocket connection to
# be closed instead if it cannot read fast enough, allowing for greater
# predictability in subscription behaviour.
experimental_close_on_slow_client = false

# How long to wait for a tx to be committed during /broadcast_tx_commit.
# WARNING: Using a value larger than 10s will result in increasing the
# global HTTP write timeout, which applies to all connections and endpoints.
# See https://github.com/tendermint/tendermint/issues/3435
timeout_broadcast_tx_commit = "{{ rpc.timeout_broadcast_tx_commit }}"

# Maximum size of request body, in bytes
max_body_bytes = {{ rpc.max_body_bytes  }}

# Maximum size of request header, in bytes
max_header_bytes = {{ rpc.max_header_bytes }}

# The path to a file containing certificate that is used to create the HTTPS server.
# Might be either absolute path or path related to Tendermint's config directory.
# If the certificate is signed by a certificate authority,
# the certFile should be the concatenation of the server's certificate, any intermediates,
# and the CA's certificate.
# NOTE: both tls_cert_file and tls_key_file must be present for Tendermint to create HTTPS server.
# Otherwise, HTTP server is run.
tls_cert_file = "{{ rpc.tls_cert_file }}"

# The path to a file containing matching private key that is used to create the HTTPS server.
# Might be either absolute path or path related to Tendermint's config directory.
# NOTE: both tls-cert-file and tls-key-file must be present for Tendermint to create HTTPS server.
# Otherwise, HTTP server is run.
tls_key_file = "{{ rpc.tls_key_file }}"

# pprof listen address (https://golang.org/pkg/net/http/pprof)
pprof_laddr = "{{ rpc.pprof_laddr }}"

#######################################################
###           P2P Configuration Options             ###
#######################################################
[p2p]

# Address to listen for incoming connections
laddr = "{{ p2p.laddr }}"

# Address to advertise to peers for them to dial
# If empty, will use the same port as the laddr,
# and will introspect on the listener or use UPnP
# to figure out the address. ip and port are required
# example: 159.89.10.97:26656
external_address = "{{ p2p.external_address }}"

# Comma separated list of seed nodes to connect to
seeds = "{{ p2p.seeds }}"

# Comma separated list of nodes to keep persistent connections to
persistent_peers = "{{ p2p.persistent_peers }}"

# UPNP port forwarding
upnp = {{ p2p.upnp }}

# Path to address book
addr_book_file = "{{ p2p.addr_book_file }}"

# Set true for strict address routability rules
# Set false for private or local networks
addr_book_strict = {{ p2p.addr_book_strict }}

# Maximum number of inbound peers
max_num_inbound_peers = {{ p2p.max_num_inbound_peers }}

# Maximum number of outbound peers to connect to, excluding persistent peers
max_num_outbound_peers = {{ p2p.max_num_outbound_peers }}

# List of node IDs, to which a connection will be (re)established ignoring any existing limits
unconditional_peer_ids = "{{ p2p.unconditional_peer_ids }}"

# Maximum pause when redialing a persistent peer (if zero, exponential backoff is used)
persistent_peers_max_dial_period = "{{ p2p.persistent_peers_max_dial_period }}"

# Time to wait before flushing messages out on the connection
flush_throttle_timeout = "{{ p2p.flush_throttle_timeout }}"

# Maximum size of a message packet payload, in bytes
max_packet_msg_payload_size = {{ p2p.max_packet_msg_payload_size }}

# Rate at which packets can be sent, in bytes/second
send_rate = {{ p2p.send_rate }}

# Rate at which packets can be received, in bytes/second
recv_rate = {{ p2p.recv_rate }}

# Set true to enable the peer-exchange reactor
pex = {{ p2p.pex }}

# Seed mode, in which node constantly crawls the network and looks for
# peers. If another node asks it for addresses, it responds and disconnects.
#
# Does not work if the peer-exchange reactor is disabled.
seed_mode = {{ p2p.seed_mode }}

# Comma separated list of peer IDs to keep private (will not be gossiped to other peers)
private_peer_ids = "{{ p2p.private_peer_ids }}"

# Toggle to disable guard against peers connecting from the same ip.
allow_duplicate_ip = {{ p2p.allow_duplicate_ip }}

# Peer connection configuration.
handshake_timeout = "{{ p2p.handshake_timeout }}"
dial_timeout = "{{ p2p.dial_timeout }}"

#######################################################
###          Mempool Configuration Option          ###
#######################################################
[mempool]

# Mempool version to use:
#   1) "v0" - (default) FIFO mempool.
#   2) "v1" - prioritized mempool.
version = "v0"

recheck = {{ mempool.recheck }}
broadcast = {{ mempool.broadcast }}
wal_dir = "{{ mempool.wal_dir }}"

# Maximum number of transactions in the mempool
size = {{ mempool.size }}

# Limit the total size of all txs in the mempool.
# This only accounts for raw transactions (e.g. given 1MB transactions and
# max_txs_bytes=5MB, mempool will only accept 5 transactions).
max_txs_bytes = {{ mempool.max_txs_bytes }}

# Size of the cache (used to filter transactions we saw earlier) in transactions
cache_size = {{ mempool.cache_size }}

# Do not remove invalid transactions from the cache (default: false)
# Set to true if it's not possible for any invalid transaction to become valid
# again in the future.
keep-invalid-txs-in-cache = {{ mempool.keep-invalid-txs-in-cache }}

# Maximum size of a single transaction.
# NOTE: the max size of a tx transmitted over the network is {max_tx_bytes}.
max_tx_bytes = {{ mempool.max_tx_bytes }}

# Maximum size of a batch of transactions to send to a peer
# Including space needed by encoding (one varint per transaction).
# XXX: Unused due to https://github.com/tendermint/tendermint/issues/5796
max_batch_bytes = {{ mempool.max_batch_bytes }}

# ttl-duration, if non-zero, defines the maximum amount of time a transaction
# can exist for in the mempool.
#
# Note, if ttl-num-blocks is also defined, a transaction will be removed if it
# has existed in the mempool at least ttl-num-blocks number of blocks or if it's
# insertion time into the mempool is beyond ttl-duration.
ttl-duration = "0"

# ttl-num-blocks, if non-zero, defines the maximum number of blocks a transaction
# can exist for in the mempool.
#
# Note, if ttl-duration is also defined, a transaction will be removed if it
# has existed in the mempool at least ttl-num-blocks number of blocks or if
# it's insertion time into the mempool is beyond ttl-duration.
ttl-num-blocks = 0

#######################################################
###         State Sync Configuration Options        ###
#######################################################
[statesync]
# State sync rapidly bootstraps a new node by discovering, fetching, and restoring a state machine
# snapshot from peers instead of fetching and replaying historical blocks. Requires some peers in
# the network to take and serve state machine snapshots. State sync is not attempted if the node
# has any local state (LastBlockHeight > 0). The node will have a truncated block history,
# starting from the height of the snapshot.
enable = {{ statesync.enable }}

# RPC servers (comma-separated) for light client verification of the synced state machine and
# retrieval of state data for node bootstrapping. Also needs a trusted height and corresponding
# header hash obtained from a trusted source, and a period during which validators can be trusted.
#
# For Cosmos SDK-based chains, trust_period should usually be about 2/3 of the unbonding time (~2
# weeks) during which they can be financially punished (slashed) for misbehavior.
rpc_servers = "{{ statesync.rpc_servers }}"
trust_height = {{ statesync.trust_height }}
trust_hash = "{{ statesync.trust_hash }}"
trust_period = "{{ statesync.trust_period }}"

# Time to spend discovering snapshots before initiating a restore.
discovery_time = "{{ statesync.discovery_time }}"

# Temporary directory for state sync snapshot chunks, defaults to the OS tempdir (typically /tmp).
# Will create a new, randomly named directory within, and remove it when done.
temp_dir = "{{ statesync.temp_dir }}"

# The timeout duration before re-requesting a chunk, possibly from a different
# peer (default: 1 minute).
chunk_request_timeout = "10s"

# The number of concurrent chunk fetchers to run (default: 1).
chunk_fetchers = "4"

#######################################################
###       Fast Sync Configuration Connections       ###
#######################################################
[fastsync]

# Fast Sync version to use:
#   1) "v0" (default) - the legacy fast sync implementation
#   2) "v1" - refactor of v0 version for better testability
#   2) "v2" - complete redesign of v0, optimized for testability & readability
version = "{{ fastsync.version }}"

#######################################################
###         Consensus Configuration Options         ###
#######################################################
[consensus]

wal_file = "{{ consensus.wal_file }}"

# How long we wait for a proposal block before prevoting nil
timeout_propose = "{{ consensus.timeout_propose }}"
# How much timeout_propose increases with each round
timeout_propose_delta = "{{ consensus.timeout_propose_delta }}"
# How long we wait after receiving +2/3 prevotes for “anything” (ie. not a single block or nil)
timeout_prevote = "{{ consensus.timeout_prevote }}"
# How much the timeout_prevote increases with each round
timeout_prevote_delta = "{{ consensus.timeout_prevote_delta }}"
# How long we wait after receiving +2/3 precommits for “anything” (ie. not a single block or nil)
timeout_precommit = "{{ consensus.timeout_precommit }}"
# How much the timeout_precommit increases with each round
timeout_precommit_delta = "{{ consensus.timeout_precommit_delta }}"
# How long we wait after committing a block, before starting on the new
# height (this gives us a chance to receive some more precommits, even
# though we already have +2/3).
timeout_commit = "{{ consensus.timeout_commit }}"

# How many blocks to look back to check existence of the node's consensus votes before joining consensus
# When non-zero, the node will panic upon restart
# if the same consensus key was used to sign {double_sign_check_height} last blocks.
# So, validators should stop the state machine, wait for some blocks, and then restart the state machine to avoid panic.
double_sign_check_height = {{ consensus.double_sign_check_height }}

# Make progress as soon as we have all the precommits (as if TimeoutCommit = 0)
skip_timeout_commit = {{ consensus.skip_timeout_commit }}

# EmptyBlocks mode and possible interval between empty blocks
create_empty_blocks = {{ consensus.create_empty_blocks }}
create_empty_blocks_interval = "{{ consensus.create_empty_blocks_interval }}"

# Reactor sleep duration parameters
peer_gossip_sleep_duration = "{{ consensus.peer_gossip_sleep_duration }}"
peer_query_maj23_sleep_duration = "{{ consensus.peer_query_maj23_sleep_duration }}"

#######################################################
###         Storage Configuration Options           ###
#######################################################
[storage]
# Set to true to discard ABCI responses from the state store, which can save a
# considerable amount of disk space. Set to false to ensure ABCI responses are
# persisted. ABCI responses are required for /block_results RPC queries, and to
# reindex events in the command-line tool.
discard_abci_responses = {{ storage.discard_abci_responses }}

#######################################################
###   Transaction Indexer Configuration Options     ###
#######################################################
[tx_index]

# What indexer to use for transactions
#
# The application will set which txs to index. In some cases a node operator will be able
# to decide which txs to index based on configuration set in the application.
#
# Options:
#   1) "null"
#   2) "kv" (default) - the simplest possible indexer, backed by key-value storage (defaults to levelDB; see DBBackend).
# 		- When "kv" is chosen "tx.height" and "tx.hash" will always be indexed.
#   3) "psql" - the indexer services backed by PostgreSQL.
# When "kv" or "psql" is chosen "tx.height" and "tx.hash" will always be indexed.
indexer = "{{ tx_index.indexer }}"

# The PostgreSQL connection configuration, the connection format:
#   postgresql://<user>:<password>@<host>:<port>/<db>?<opts>
psql-conn = ""

#######################################################
###       Instrumentation Configuration Options     ###
#######################################################
[instrumentation]

# When true, Prometheus metrics are served under /metrics on
# PrometheusListenAddr.
# Check out the documentation for the list of available metrics.
prometheus = {{ instrumentation.prometheus }}

# Address to listen for Prometheus collector(s) connections
prometheus_listen_addr = "{{ instrumentation.prometheus_listen_addr }}"

# Maximum number of simultaneous connections.
# If you want to accept a larger number than the default, make sure
# you increase your OS limits.
# 0 - unlimited.
max_open_connections = {{ instrumentation.max_open_connections }}

# Instrumentation namespace
namespace = "{{ instrumentation.namespace }}"

"#;
