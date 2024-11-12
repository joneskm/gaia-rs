use std::{marker::PhantomData, path::PathBuf};

use address::AccAddress;
use clap::{ArgAction, Args, Subcommand, ValueEnum, ValueHint};
use strum::Display;
use tendermint::types::chain_id::ChainId;

use crate::{
    application::ApplicationInfo,
    cli::config::client_config,
    commands::client::{
        keys::KeyringBackend,
        tx::{AccountProvider, ClientTxContext, Keyring as TxKeyring, LocalInfo, TxCommand},
    },
    types::{auth::fee::Fee, base::coins::UnsignedCoins},
};

use gas::Gas;

/// Transaction subcommands
#[derive(Debug, Clone, ::clap::Args)]
pub struct CliTxCommand<T: ApplicationInfo, C: Args> {
    #[arg(long, global = true, action = ArgAction::Set, value_hint = ValueHint::DirPath, default_value_os_t = T::home_dir(), help = "directory for config and data")]
    home: PathBuf,
    /// <host>:<port> to Tendermint RPC interface for this chain
    #[arg(long, global = true, action = ArgAction::Set, value_hint = ValueHint::Url, env = "GEARS_NODE", default_value_t = client_config(&T::home_dir()).node())]
    pub node: url::Url,
    /// the network chain-id
    #[arg(long =  "chain-id", global = true, action = ArgAction::Set, default_value_t = client_config(&T::home_dir()).chain_id())]
    pub chain_id: ChainId,

    #[command(flatten)]
    pub fee: FeeCli,

    #[arg(long, short, default_value_t = Keyring::Local)]
    pub keyring: Keyring,

    #[command(flatten)]
    #[group(id = "local", conflicts_with = Keyring::Ledger, global = true)]
    pub local: Option<Local<T>>,

    #[command(flatten)]
    #[group(id = "Broadcast mode", global = true)]
    pub mode: Mode,

    /// Note to add a description to the transaction
    #[arg(long, global = true, action = ArgAction::Set, required = false )]
    pub note: Option<String>,

    /// Set a block timeout height to prevent the tx from being committed past a certain height
    #[arg(long, global = true, action = ArgAction::Set, required = false )]
    pub timeout_height: Option<u32>,

    #[command(flatten)]
    pub command: C,

    #[arg(skip)]
    _marker: PhantomData<T>,
}

#[derive(Debug, Clone, ::clap::Args)]
pub struct FeeCli {
    // TODO: Cosmos has "auto" feature to calculate gas price if needed
    /// gas limit to set per-transaction
    #[arg(long, short, global = true, action = ArgAction::Set, default_value_t = 200_000)]
    pub gas_limit: u64,
    /// Fees to pay along with transaction; eg: 10uatom
    #[arg(long, global = true, action = ArgAction::Set)]
    pub fees: Option<UnsignedCoins>,
    /// Fee payer pays fees for the transaction instead of deducting from the signer
    #[arg(long = "fee-payer", global = true, action = ArgAction::Set, required = false )]
    pub payer: Option<AccAddress>,
    /// Fee granter grants fees for the transaction
    #[arg(long = "fee-granter", global = true, action = ArgAction::Set, required = false )]
    pub granter: Option<String>,
}

impl TryFrom<FeeCli> for Fee {
    type Error = anyhow::Error;

    fn try_from(
        FeeCli {
            gas_limit,
            fees,
            payer,
            granter,
        }: FeeCli,
    ) -> Result<Self, Self::Error> {
        let gas_limit = Gas::try_from(gas_limit)?;

        if granter.as_ref().is_some_and(|this| this.is_empty()) {
            Err(anyhow::anyhow!("`fee-granter` can't be empty"))?
        }

        Ok(Self {
            amount: fees,
            gas_limit,
            payer,
            granter: match granter {
                Some(var) => var,
                None => "".to_owned(),
            },
        })
    }
}

#[derive(Debug, Clone, ::clap::Args)]
pub struct Mode {
    /// makes sure that the client will not reach out to full node.
    /// As a result, the account and sequence number queries will not be performed and
    /// it is required to set such parameters manually. Note, invalid values will cause
    /// the transaction to fail.
    #[arg(long, default_value_t = false, help_heading = "Broadcast mode")]
    pub offline: bool,
    /// The sequence number of the signing account (offline mode only)
    #[arg(long, required = false, help_heading = "Broadcast mode")]
    pub sequence: Option<u64>,
    /// The account number of the signing account (offline mode only)
    #[arg(long, required = false, help_heading = "Broadcast mode")]
    pub account_number: Option<u64>,
}

#[derive(ValueEnum, Debug, Clone, Display)]
pub enum Keyring {
    /// Use a Ledger device to sign the transaction
    #[strum(to_string = "ledger")]
    Ledger,
    /// Use a local keyring to source the signing key
    #[strum(to_string = "local")]
    Local,
}

#[derive(Debug, Clone, ::clap::Args)]
pub struct Local<T: ApplicationInfo> {
    /// from key
    #[arg(long, short = 'f', global = true, required = false)]
    #[arg(help_heading = "Local signing options")]
    from_key: String,

    /// select keyring's backend
    #[arg(long = "keyring-backend", short = 'b',  global = true, action = ArgAction::Set, default_value_t = client_config(&T::home_dir()).keyring_backend() )]
    #[arg(help_heading = "Local signing options")]
    keyring_backend: KeyringBackend,

    #[arg(skip)]
    _marker: PhantomData<T>,
}

#[derive(Debug, Clone, ::clap::Args)]
pub struct Ledger<C: Subcommand> {
    #[command(subcommand)]
    command: C,
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("Missing options for: {0}")]
pub struct MissingCliOptions(pub String);

impl<T, C, AC> TryFrom<CliTxCommand<T, C>> for TxCommand<AC>
where
    T: ApplicationInfo,
    C: Args,
    AC: TryFrom<C, Error = anyhow::Error>,
{
    type Error = anyhow::Error;

    fn try_from(value: CliTxCommand<T, C>) -> Result<Self, Self::Error> {
        let CliTxCommand {
            home,
            node,
            chain_id,
            _marker,
            keyring,
            local,
            mode,
            note,
            timeout_height,
            fee,
            command,
        } = value;

        let keyring = match keyring {
            Keyring::Ledger => TxKeyring::Ledger,
            Keyring::Local => {
                let Local {
                    from_key,
                    keyring_backend,
                    ..
                } = local.ok_or(MissingCliOptions(
                    "local signing options: from-key".to_owned(),
                ))?;

                TxKeyring::Local(LocalInfo {
                    keyring_backend,
                    from_key,
                })
            }
        };

        let account = match mode {
            Mode {
                offline: true,
                sequence,
                account_number,
            } => AccountProvider::Offline {
                sequence: sequence.unwrap_or_default(),
                account_number: account_number.unwrap_or_default(),
            },
            Mode {
                offline: false,
                sequence: Some(sequence),
                account_number,
            } => AccountProvider::Offline {
                sequence,
                account_number: account_number.unwrap_or_default(),
            },
            Mode {
                offline: false,
                sequence,
                account_number: Some(account_number),
            } => AccountProvider::Offline {
                sequence: sequence.unwrap_or_default(),
                account_number,
            },
            _ => AccountProvider::Online,
        };

        Ok(Self {
            inner: command.try_into()?,
            ctx: ClientTxContext {
                home,
                node,
                chain_id,
                keyring,
                account,
                memo: note,
                timeout_height,
                fee: fee.try_into()?,
            },
        })
    }
}
