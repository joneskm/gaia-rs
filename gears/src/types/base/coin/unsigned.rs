use cosmwasm_std::Uint256;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tendermint::types::proto::Protobuf;

use crate::types::{base::errors::CoinsError, denom::Denom, errors::Error};

mod inner {
    pub use core_types::base::coin::Coin;
    pub use core_types::base::coin::IntProto;
}

/// Coin defines a token with a denomination and an amount.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(try_from = "inner::Coin", into = "inner::Coin")]
pub struct Coin {
    pub denom: Denom,
    pub amount: Uint256,
}

impl TryFrom<inner::Coin> for Coin {
    type Error = CoinsError;

    fn try_from(value: inner::Coin) -> Result<Self, Self::Error> {
        let denom = value
            .denom
            .try_into()
            .map_err(|e: Error| CoinsError::Denom(e.to_string()))?;
        let amount =
            Uint256::from_str(&value.amount).map_err(|e| CoinsError::Uint(e.to_string()))?;

        Ok(Coin { denom, amount })
    }
}

impl From<Coin> for inner::Coin {
    fn from(value: Coin) -> inner::Coin {
        Self {
            denom: value.denom.to_string(),
            amount: value.amount.to_string(),
        }
    }
}

impl Protobuf<inner::Coin> for Coin {}

impl FromStr for Coin {
    type Err = CoinsError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        // get the index at which amount ends and denom starts
        let i = input.find(|c: char| !c.is_numeric()).unwrap_or(input.len());

        let amount = input[..i]
            .parse::<Uint256>()
            .map_err(|e| CoinsError::Uint(e.to_string()))?;

        let denom = input[i..]
            .parse::<Denom>()
            .map_err(|e| CoinsError::Denom(e.to_string()))?;

        Ok(Coin { denom, amount })
    }
}

/// Uint256Proto is a proto wrapper around Uint256 to allow for proto serialization.
#[derive(Clone, Serialize, Deserialize)]
pub struct Uint256Proto {
    pub uint: Uint256,
}

impl TryFrom<inner::IntProto> for Uint256Proto {
    type Error = CoinsError;

    fn try_from(value: inner::IntProto) -> Result<Self, Self::Error> {
        let uint = Uint256::from_str(&value.int).map_err(|e| CoinsError::Uint(e.to_string()))?;
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

impl From<(Denom, Uint256)> for Coin {
    fn from((denom, amount): (Denom, Uint256)) -> Self {
        Self { denom, amount }
    }
}

impl From<Coin> for (Denom, Uint256) {
    fn from(Coin { denom, amount }: Coin) -> Self {
        (denom, amount)
    }
}
