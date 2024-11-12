use core_types::{errors::CoreError, Protobuf};
use cosmwasm_std::Uint256;
use extensions::pagination::PaginationKey;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, str::FromStr};

use crate::types::{base::errors::CoinError, denom::Denom, errors::DenomError};

use super::Coin;

pub mod inner {
    pub use core_types::base::Coin;
    pub use core_types::base::IntProto;
}

/// Coin defines a token with a denomination and an amount.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(try_from = "inner::Coin", into = "inner::Coin")]
pub struct UnsignedCoin {
    pub denom: Denom,
    pub amount: Uint256,
}

impl Coin for UnsignedCoin {
    type Amount = Uint256;

    fn denom(&self) -> &Denom {
        &self.denom
    }

    fn amount(&self) -> &Uint256 {
        &self.amount
    }
}

impl TryFrom<inner::Coin> for UnsignedCoin {
    type Error = CoinError;

    fn try_from(value: inner::Coin) -> Result<Self, Self::Error> {
        let denom = value
            .denom
            .try_into()
            .map_err(|e: DenomError| CoinError::Denom(e.to_string()))?;
        let amount =
            Uint256::from_str(&value.amount).map_err(|e| CoinError::Uint(e.to_string()))?;

        Ok(UnsignedCoin { denom, amount })
    }
}

impl From<UnsignedCoin> for inner::Coin {
    fn from(value: UnsignedCoin) -> inner::Coin {
        Self {
            denom: value.denom.to_string(),
            amount: value.amount.to_string(),
        }
    }
}

impl Protobuf<inner::Coin> for UnsignedCoin {}

impl FromStr for UnsignedCoin {
    type Err = CoinError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        // get the index at which amount ends and denom starts
        let i = input.find(|c: char| !c.is_numeric()).unwrap_or(input.len());

        let amount = input[..i]
            .parse::<Uint256>()
            .map_err(|e| CoinError::Uint(e.to_string()))?;

        let denom = input[i..]
            .parse::<Denom>()
            .map_err(|e| CoinError::Denom(e.to_string()))?;

        Ok(UnsignedCoin { denom, amount })
    }
}

impl TryFrom<Vec<u8>> for UnsignedCoin {
    type Error = CoreError;

    fn try_from(raw: Vec<u8>) -> Result<Self, Self::Error> {
        Self::decode_vec(&raw).map_err(|e| CoreError::DecodeProtobuf(e.to_string()))
    }
}

impl From<UnsignedCoin> for Vec<u8> {
    fn from(value: UnsignedCoin) -> Self {
        value.encode_vec()
    }
}

/// Uint256Proto is a proto wrapper around Uint256 to allow for proto serialization.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Uint256Proto {
    pub uint: Uint256,
}

impl TryFrom<inner::IntProto> for Uint256Proto {
    type Error = CoinError;

    fn try_from(value: inner::IntProto) -> Result<Self, Self::Error> {
        let uint = Uint256::from_str(&value.int).map_err(|e| CoinError::Uint(e.to_string()))?;
        Ok(Uint256Proto { uint })
    }
}

impl From<Uint256Proto> for inner::IntProto {
    fn from(value: Uint256Proto) -> inner::IntProto {
        Self {
            int: value.uint.to_string(),
        }
    }
}

impl Protobuf<inner::IntProto> for Uint256Proto {}

impl PaginationKey for UnsignedCoin {
    fn iterator_key(&self) -> Cow<'_, [u8]> {
        Cow::Borrowed(self.denom.as_ref())
    }
}

impl std::fmt::Display for UnsignedCoin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.amount, self.denom)
    }
}
