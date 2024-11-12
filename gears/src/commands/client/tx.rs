use std::path::PathBuf;

use core_types::tx::mode_info::SignMode;
use prost::Message;
use tendermint::rpc::client::{Client, HttpClient};
use tendermint::rpc::response::tx::broadcast::Response;
use tendermint::types::chain_id::ChainId;

use crate::application::handlers::client::{NodeFetcher, TxExecutionResult, TxHandler};
use crate::commands::client::query::execute_query;
use crate::crypto::any_key::AnyKey;
use crate::crypto::keys::GearsPublicKey;
use crate::crypto::ledger::LedgerProxyKey;
use crate::runtime::runtime;
use crate::types::auth::fee::Fee;
use crate::types::tx::raw::TxRaw;
use gas::Gas;

use super::keys::KeyringBackend;

#[derive(Debug, Clone)]
pub enum AccountProvider {
    Offline { sequence: u64, account_number: u64 },
    Online,
}

#[derive(Debug, Clone, former::Former)]
pub struct TxCommand<C> {
    pub ctx: ClientTxContext,
    pub inner: C,
}

/// Context for client during execution of tx which carry additional state.
///
/// I don't like the idea of context, but this allows
/// to share some state between different stages of tx request.
#[derive(Debug, Clone)]
pub struct ClientTxContext {
    pub node: url::Url,
    pub home: PathBuf,
    pub keyring: Keyring,
    pub memo: Option<String>,
    pub account: AccountProvider,
    pub chain_id: ChainId,
    pub timeout_height: Option<u32>,

    pub fee: Fee,
}

impl ClientTxContext {
    pub fn query<Response: TryFrom<Raw>, Raw: Message + Default + std::convert::From<Response>>(
        &self,
        path: String,
        query_bytes: Vec<u8>,
    ) -> anyhow::Result<Response>
    where
        <Response as TryFrom<Raw>>::Error: std::fmt::Display,
    {
        execute_query(path, query_bytes, self.node.as_str(), None)
    }

    /// Create new `self` with flag to immediately execute query instead of printing it or saving to file
    pub fn new_online(
        home: PathBuf,
        gas_limit: Gas,
        node: url::Url,
        chain_id: ChainId,
        from_key: &str,
    ) -> Self {
        Self {
            account: crate::commands::client::tx::AccountProvider::Online,
            home,
            keyring: Keyring::Local(LocalInfo {
                keyring_backend: KeyringBackend::Test,
                from_key: from_key.to_owned(),
            }),
            node,
            chain_id,
            memo: None,
            timeout_height: None,
            fee: Fee {
                amount: None,
                gas_limit,
                payer: None,
                granter: "".to_owned(),
            },
        }
    }
}

/// Source to fetch keys
#[derive(Debug, Clone)]
pub enum Keyring {
    Ledger,
    Local(LocalInfo),
}

/// Additional information for local keyring
#[derive(Debug, Clone)]
pub struct LocalInfo {
    pub keyring_backend: KeyringBackend,
    pub from_key: String,
}

/// Result of execution of tx
#[derive(Debug, Clone)]
pub enum RuntxResult {
    /// Result of broadcasting a txs
    Broadcast(Vec<Response>),
    /// Path to tx saved to file
    File(PathBuf),
    /// No result of tx. Probably it was printed to `stdout`
    None,
}

impl RuntxResult {
    pub fn broadcast(self) -> Option<Vec<Response>> {
        match self {
            Self::Broadcast(var) => Some(var),
            Self::File(_) => None,
            Self::None => None,
        }
    }

    pub fn file(self) -> Option<PathBuf> {
        match self {
            Self::Broadcast(_) => None,
            Self::File(var) => Some(var),
            Self::None => None,
        }
    }
}

impl From<TxExecutionResult> for RuntxResult {
    fn from(value: TxExecutionResult) -> Self {
        match value {
            TxExecutionResult::Broadcast(var) => Self::Broadcast(vec![var]),
            TxExecutionResult::File(var) => Self::File(var),
            TxExecutionResult::None => Self::None,
        }
    }
}

fn handle_key(client_tx_context: &ClientTxContext) -> anyhow::Result<AnyKey> {
    match client_tx_context.keyring {
        Keyring::Ledger => Ok(AnyKey::Ledger(LedgerProxyKey::new()?)),
        Keyring::Local(ref local) => {
            let keyring_home = client_tx_context
                .home
                .join(local.keyring_backend.get_sub_dir());
            let key = keyring::key_by_name(
                &local.from_key,
                local.keyring_backend.to_keyring_backend(&keyring_home),
            )?;

            Ok(AnyKey::Local(key))
        }
    }
}

/// Convenient way to broadcast a tx.
/// This method reads key from keyring which needed to sign a tx
/// and prepare it to broadcasting. After that it broadcasts messages by chunks
/// if more that single msg specified
pub fn run_tx<C, H: TxHandler<TxCommands = C>, F: NodeFetcher + Clone>(
    TxCommand { mut ctx, inner }: TxCommand<C>,
    handler: &H,
    fetcher: &F,
) -> anyhow::Result<RuntxResult> {
    let key = handle_key(&ctx)?;

    let messages = handler.prepare_tx(&mut ctx, inner, key.get_gears_public_key())?;

    if messages.chunk_size() > 0
    // TODO: uncomment and update logic when command will be extended by broadcast_mode
    /* && command.broadcast_mode == BroadcastMode::Block */
    {
        let chunk_size = messages.chunk_size();
        let msgs = messages.into_msgs();

        let mut res = vec![];
        for slice in msgs.chunks(chunk_size) {
            let tx_result = handler.handle_tx(
                handler.sign_msg(
                    slice
                        .to_vec()
                        .try_into()
                        .expect("chunking of the messages excludes empty vectors"),
                    &key,
                    SignMode::Direct,
                    &mut ctx,
                    fetcher,
                )?,
                &mut ctx,
            )?;

            if let TxExecutionResult::Broadcast(tx_result) = tx_result {
                res.push(tx_result);
            }
        }
        Ok(RuntxResult::Broadcast(res))
    } else {
        // TODO: can be reduced by changing variable `step`. Do we need it?
        handler
            .handle_tx(
                handler.sign_msg(messages, &key, SignMode::Direct, &mut ctx, fetcher)?,
                &mut ctx,
            )
            .map(Into::into)
    }
}

/// Helper method to run a tx with blocking.
///
/// **WARNING**: never use this method in async context due internal blocking using tokio runtime
pub fn broadcast_tx_commit(client: HttpClient, raw_tx: TxRaw) -> anyhow::Result<Response> {
    let res = runtime().block_on(
        client.broadcast_tx_commit(core_types::tx::raw::TxRaw::from(raw_tx).encode_to_vec()),
    )?;

    Ok(res)
}
