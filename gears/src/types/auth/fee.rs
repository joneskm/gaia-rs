use std::str::FromStr;

use core_types::address::AccAddress;
use core_types::errors::AddressError;
use cosmwasm_std::Uint256;
use serde::Deserialize;
use serde::Serialize;
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use tendermint::types::proto::Protobuf;

use crate::types::base::coin::Coin;
use crate::types::base::errors::CoinsError;
use crate::types::base::errors::SendCoinsError;
use crate::types::base::send::SendCoins;

pub mod inner {
    pub use core_types::auth::fee::Fee;
    pub use core_types::base::coin::Coin;
}

pub const MAX_GAS_WANTED: u64 = 9223372036854775807; // = (1 << 63) -1 as specified in the cosmos SDK

/// Fee includes the amount of coins paid in fees and the maximum
/// gas to be used by the transaction. The ratio yields an effective "gasprice",
/// which must be above some miminum to be accepted into the mempool.
#[serde_as]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fee {
    /// amount is the amount of coins to be paid as a fee
    pub amount: Option<SendCoins>,
    /// gas_limit is the maximum gas that can be used in transaction processing
    /// before an out of gas error occurs
    #[serde_as(as = "DisplayFromStr")]
    pub gas_limit: u64,
    /// if unset, the first signer is responsible for paying the fees. If set, the specified account must pay the fees.
    /// the payer must be a tx signer (and thus have signed this field in AuthInfo).
    /// setting this field does *not* change the ordering of required signers for the transaction.
    pub payer: Option<AccAddress>,
    /// if set, the fee payer (either the first signer or the value of the payer field) requests that a fee grant be used
    /// to pay fees instead of the fee payer's own balance. If an appropriate fee grant does not exist or the chain does
    /// not support fee grants, this will fail
    pub granter: String,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum FeeError {
    #[error("invalid gas supplied {0} > {MAX_GAS_WANTED}")]
    GasLimit(u64),
    #[error("{0}")]
    Coins(#[from] CoinsError),
    #[error("{0}")]
    Address(#[from] AddressError),
    #[error("{0}")]
    SendCoins(#[from] SendCoinsError),
}

impl TryFrom<inner::Fee> for Fee {
    type Error = FeeError;

    fn try_from(raw: inner::Fee) -> Result<Self, Self::Error> {
        if raw.gas_limit > MAX_GAS_WANTED {
            return Err(FeeError::GasLimit(raw.gas_limit));
        }

        // There's a special case in the cosmos-sdk which allows the list of coins to be "invalid" provided
        // they're all zero - we'll check for this case and represent such a list of coins as a None fee amount.
        let mut all_zero = true;
        for coin in &raw.amount {
            let amount =
                Uint256::from_str(&coin.amount).map_err(|e| CoinsError::Uint(e.to_string()))?;

            if !amount.is_zero() {
                all_zero = false;
                break;
            }
        }

        let payer = match raw.payer.as_str() {
            "" => None,
            address => {
                let addr = AccAddress::from_bech32(address)?;
                Some(addr)
            }
        };

        if all_zero {
            return Ok(Fee {
                amount: None,
                gas_limit: raw.gas_limit,
                payer,
                granter: raw.granter,
            });
        }

        let coins: Result<Vec<Coin>, CoinsError> =
            raw.amount.into_iter().map(Coin::try_from).collect();

        Ok(Fee {
            amount: Some(SendCoins::new(coins?)?),
            gas_limit: raw.gas_limit,
            payer,
            granter: raw.granter,
        })
    }
}

impl From<Fee> for inner::Fee {
    fn from(fee: Fee) -> inner::Fee {
        let payer = match fee.payer {
            Some(addr) => addr.to_string(),
            None => "".into(),
        };
        match fee.amount {
            Some(amount) => {
                let coins: Vec<Coin> = amount.into();
                let coins = coins.into_iter().map(inner::Coin::from).collect();

                Self {
                    amount: coins,
                    gas_limit: fee.gas_limit,
                    payer,
                    granter: fee.granter,
                }
            }
            None => Self {
                amount: vec![],
                gas_limit: fee.gas_limit,
                payer,
                granter: fee.granter,
            },
        }
    }
}

impl Protobuf<inner::Fee> for Fee {}

