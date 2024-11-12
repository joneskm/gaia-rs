use std::path::PathBuf;

use crate::{
    baseapp::Query,
    commands::client::tx::{broadcast_tx_commit, AccountProvider, ClientTxContext},
    crypto::{
        info::{create_signed_transaction_direct, create_signed_transaction_textual, SigningInfo},
        keys::{GearsPublicKey, ReadAccAddress, SigningKey},
        public::PublicKey,
    },
    runtime::runtime,
    signing::{handler::MetadataGetter, renderer::value_renderer::ValueRenderer},
    types::{
        account::{Account, BaseAccount},
        address::AccAddress,
        denom::Denom,
        tx::{body::TxBody, metadata::Metadata, Messages, Tx, TxMessage},
    },
};

use anyhow::anyhow;
use core_types::tx::mode_info::SignMode;
use serde::Serialize;

use tendermint::{
    rpc::{
        client::{Client, HttpClient},
        response::tx::broadcast::Response,
    },
    types::proto::block::Height,
};

#[derive(Debug, Clone, Default)]
pub enum TxExecutionResult {
    Broadcast(Response),
    File(PathBuf),
    #[default]
    None,
}

impl TxExecutionResult {
    pub fn broadcast(self) -> Option<Response> {
        match self {
            TxExecutionResult::Broadcast(var) => Some(var),
            TxExecutionResult::File(_) => None,
            TxExecutionResult::None => None,
        }
    }

    pub fn file(self) -> Option<PathBuf> {
        match self {
            TxExecutionResult::Broadcast(_) => None,
            TxExecutionResult::File(var) => Some(var),
            TxExecutionResult::None => None,
        }
    }
}

impl From<Response> for TxExecutionResult {
    fn from(value: Response) -> Self {
        Self::Broadcast(value)
    }
}

pub trait TxHandler {
    type Message: TxMessage + ValueRenderer;
    type TxCommands;

    fn prepare_tx(
        &self,
        client_tx_context: &mut ClientTxContext,
        command: Self::TxCommands,
        pubkey: PublicKey,
    ) -> anyhow::Result<Messages<Self::Message>>;

    fn account<F: NodeFetcher>(
        &self,
        address: AccAddress,
        client_tx_context: &mut ClientTxContext,
        fetcher: &F,
    ) -> anyhow::Result<Option<Account>> {
        match client_tx_context.account {
            AccountProvider::Offline {
                sequence,
                account_number,
            } => Ok(Some(Account::Base(BaseAccount {
                address,
                pub_key: None,
                account_number,
                sequence,
            }))),
            AccountProvider::Online => {
                fetcher.latest_account(address, client_tx_context.node.as_str())
            }
        }
    }

    fn sign_msg<K: SigningKey + ReadAccAddress + GearsPublicKey, F: NodeFetcher + Clone>(
        &self,
        msgs: Messages<Self::Message>,
        key: &K,
        mode: SignMode,
        ctx: &mut ClientTxContext,
        fetcher: &F,
    ) -> anyhow::Result<Tx<Self::Message>> {
        let address = key.get_address();

        let account = self
            .account(address.to_owned(), ctx, fetcher)?
            .ok_or_else(|| anyhow!("account not found: {}", address))?;

        let signing_infos = vec![SigningInfo {
            key,
            sequence: account.get_sequence(),
            account_number: account.get_account_number(),
        }];

        let tx_body = TxBody {
            messages: msgs.into_msgs(),
            memo: ctx.memo.clone().unwrap_or_default(),
            timeout_height: ctx.timeout_height.unwrap_or_default(),
            extension_options: vec![], // TODO: remove hard coded
            non_critical_extension_options: vec![], // TODO: remove hard coded
        };

        let tip = None; //TODO: remove hard coded

        match mode {
            SignMode::Direct => create_signed_transaction_direct(
                signing_infos,
                ctx.chain_id.clone(),
                ctx.fee.clone(),
                tip,
                tx_body,
            )
            .map_err(|e| anyhow!(e.to_string())),
            SignMode::Textual => create_signed_transaction_textual(
                signing_infos,
                ctx.chain_id.clone(),
                ctx.fee.clone(),
                tip,
                ctx.node.clone(),
                tx_body,
                fetcher,
            )
            .map_err(|e| anyhow!(e.to_string())),
            _ => Err(anyhow!("unsupported sign mode")),
        }
    }

    fn handle_tx(
        &self,
        raw_tx: Tx<Self::Message>,
        client_tx_context: &mut ClientTxContext,
    ) -> anyhow::Result<TxExecutionResult> {
        match client_tx_context.account {
            AccountProvider::Offline {
                sequence: _,
                account_number: _,
            } => {
                println!("{}", serde_json::to_string_pretty(&raw_tx)?);

                Ok(TxExecutionResult::None)
            }
            AccountProvider::Online => {
                let client = HttpClient::new(tendermint::rpc::url::Url::try_from(
                    client_tx_context.node.clone(),
                )?)?;
                broadcast_tx_commit(client, Into::into(&raw_tx)).map(Into::into)
            }
        }
    }
}

/// Handles query request, serialization and displaying it as `String`
pub trait QueryHandler {
    /// Additional context to use. \
    /// In most cases you would expect this to be some sort of cli command
    type QueryCommands;
    /// Query request which contains all information needed for request
    type QueryRequest: Query;
    /// Serialized response from query request
    type QueryResponse: Serialize;

    /// Prepare query to execute based on input command.
    /// Return `Self::Query` which should be used in `Self::execute_query` to retrieve raw bytes of query
    fn prepare_query_request(
        &self,
        command: &Self::QueryCommands,
    ) -> anyhow::Result<Self::QueryRequest>;

    /// Executes request to node
    /// Returns raw bytes of `Self::QueryResponse`
    fn execute_query_request(
        &self,
        query: Self::QueryRequest,
        node: url::Url,
        height: Option<Height>,
    ) -> anyhow::Result<Vec<u8>> {
        let client = HttpClient::new(node.as_str())?;

        let res = runtime().block_on(client.abci_query(
            Some(query.query_url().to_owned()),
            query.into_bytes(),
            height,
            false,
        ))?;

        if res.code.is_err() {
            return Err(anyhow::anyhow!("node returned an error: {}", res.log));
        }

        Ok(res.value)
    }

    /// Handle serialization of query bytes into concrete type. \
    /// # Motivation
    /// This method allows to use custom serialization logic without introducing any new trait bound
    /// and allows to use it with enum which stores all responses from you module
    fn handle_raw_response(
        &self,
        query_bytes: Vec<u8>,
        command: &Self::QueryCommands,
    ) -> anyhow::Result<Self::QueryResponse>;
}

pub trait NodeFetcher {
    /// Query node to get latest account state
    fn latest_account(
        &self,
        address: AccAddress,
        node: impl AsRef<str>,
    ) -> anyhow::Result<Option<Account>>;

    /// Query node to get denom metadata
    fn denom_metadata(
        &self,
        base: Denom,
        node: impl AsRef<str>,
    ) -> anyhow::Result<Option<Metadata>>;
}

#[derive(Debug)]
pub struct MetadataViaRPC<F: NodeFetcher> {
    pub node: url::Url,
    pub fetcher: F,
}

impl<F: NodeFetcher> MetadataGetter for MetadataViaRPC<F> {
    type Error = anyhow::Error;

    fn metadata(
        &self,
        denom: &Denom,
    ) -> Result<Option<crate::types::tx::metadata::Metadata>, Self::Error> {
        let res = self
            .fetcher
            .denom_metadata(denom.to_owned(), self.node.as_str())?;
        Ok(res)
    }
}
