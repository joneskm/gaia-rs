use core_types::Protobuf;
use serde::{Deserialize, Serialize};
use tx_derive::AppMessage;

use crate::types::{
    address::AccAddress,
    base::{coin::UnsignedCoin, coins::UnsignedCoins, errors::CoinError},
};

mod inner {
    pub use core_types::base::Coin;
    pub use core_types::msg::MsgSend;
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, thiserror::Error)]
#[error("MsgSend parse error: {0}")]
pub struct MsgSendParseError(pub String);

/// MsgSend represents a message to send coins from one account to another.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, AppMessage)]
#[msg(
    url = "/cosmos.bank.v1beta1.MsgSend",
    amino_url = "cosmos-sdk/MsgSend",
    gears
)]
pub struct MsgSend {
    #[msg(signer)]
    pub from_address: AccAddress,
    pub to_address: AccAddress,
    pub amount: UnsignedCoins,
}

impl TryFrom<inner::MsgSend> for MsgSend {
    type Error = MsgSendParseError;

    fn try_from(raw: inner::MsgSend) -> Result<Self, Self::Error> {
        let from_address = AccAddress::from_bech32(&raw.from_address)
            .map_err(|e| MsgSendParseError(e.to_string()))?;

        let to_address = AccAddress::from_bech32(&raw.to_address)
            .map_err(|e| MsgSendParseError(e.to_string()))?;

        let coins = raw
            .amount
            .into_iter()
            .map(UnsignedCoin::try_from)
            .collect::<Result<Vec<_>, CoinError>>()
            .map_err(|e| MsgSendParseError(e.to_string()))?;

        Ok(MsgSend {
            from_address,
            to_address,
            amount: UnsignedCoins::new(coins).map_err(|e| MsgSendParseError(e.to_string()))?,
        })
    }
}

impl From<MsgSend> for inner::MsgSend {
    fn from(msg: MsgSend) -> inner::MsgSend {
        let coins: Vec<UnsignedCoin> = msg.amount.into();
        let coins = coins.into_iter().map(inner::Coin::from).collect();

        Self {
            from_address: msg.from_address.into(),
            to_address: msg.to_address.into(),
            amount: coins,
        }
    }
}

impl Protobuf<inner::MsgSend> for MsgSend {}
