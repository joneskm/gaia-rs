pub mod metering;
pub mod store;

use cosmwasm_std::{Decimal256, Uint256};
use derive_more::derive::{Add, Deref, Display, From, Into, Mul, Sub};
use std::{num::ParseIntError, str::FromStr};
use ux::u63;

pub mod inner {
    pub use core_types::auth::Fee;
    pub use core_types::base::Coin;
}

/// Gas represents gas amounts. It's a wrapper around u63. Gas amounts are represented as i64 in tendermint
/// with only positive values representing valid gas amounts. Since u63::MAX == i64::MAX all valid tendermint
/// gas amounts can be represented as Gas and conversely all Gas amounts can be represented as i64.
/// This is inline with Cosmos SDK behaviour, there a u64 is used for gas amounts with an explicit check, see
/// https://github.com/cosmos/cosmos-sdk/blob/2582f0aab7b2cbf66ade066fe570a4622cf0b098/types/tx/types.go#L13
#[derive(
    Copy,
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    Display,
    Deref,
    Into,
    Add,
    Sub,
    From,
    Mul,
)]
pub struct Gas(u63);

impl Gas {
    pub const MAX: Self = Self(u63::MAX);

    pub const ZERO: Self = Self(u63::new(0));

    pub const fn new(val: u63) -> Self {
        Self(val)
    }

    // TODO: write a test for this
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        if (Self::MAX - self) >= rhs {
            Some(self + rhs)
        } else {
            None
        }
    }

    // TODO: write a test for this
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        let self_inner: u64 = self.0.into();
        let rhs_inner: u64 = rhs.0.into();

        let result = self_inner.checked_sub(rhs_inner)?;

        Some(Gas::new(u63::new(result)))
    }

    // TODO: write a test for this
    pub fn checked_mul(self, rhs: Self) -> Option<Self> {
        // Div and Mul are not implemented for u63 so we can't do this:
        // if self != Self::ZERO && rhs > Self::MAX / self {
        //     None
        // } else {
        //     Some(self * rhs)
        // }

        let a: u64 = self.0.into();
        let b: u64 = rhs.0.into();
        let max: u64 = u63::MAX.into();

        if a != 0 && b > max / a {
            None
        } else {
            Some(Self(u63::new(a * b))) //new is safe as we have already checked the limit
        }
    }
}

impl TryFrom<u64> for Gas {
    type Error = GasError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        if value > u63::MAX.into() {
            return Err(GasError::Limit(value));
        }

        Ok(Self(u63::new(value))) //new is safe as we have already checked the limit
    }
}

impl From<u8> for Gas {
    fn from(value: u8) -> Self {
        Self(value.into())
    }
}

impl From<u16> for Gas {
    fn from(value: u16) -> Self {
        Self(value.into())
    }
}

impl From<u32> for Gas {
    fn from(value: u32) -> Self {
        Self(value.into())
    }
}

impl From<Gas> for u64 {
    fn from(val: Gas) -> u64 {
        val.0.into()
    }
}

impl FromStr for Gas {
    type Err = GasError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let u_64 = u64::from_str(s)?;
        u_64.try_into().map_err(|_| GasError::Limit(u_64))
    }
}

impl From<Gas> for i64 {
    fn from(val: Gas) -> i64 {
        let u_64: u64 = val.0.into();
        u_64 as i64 // safe cast as this u_64 is always ≤ i64::MAX
    }
}

impl TryFrom<i64> for Gas {
    type Error = GasError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        if value < 0 {
            return Err(GasError::Negative(value));
        }

        Ok(u63::new(value as u64).into()) // cast is safe as we have already checked for negative values
    }
}

impl From<Gas> for Uint256 {
    fn from(val: Gas) -> Uint256 {
        let u_64: u64 = val.0.into();
        Uint256::from(u_64)
    }
}

impl From<Gas> for Decimal256 {
    fn from(val: Gas) -> Decimal256 {
        let u_64: u64 = val.0.into();
        Decimal256::from_atomics(u_64, 0)
            .expect("u64::MAX < Decimal256::MAX so this will never fail")
    }
}

#[derive(Debug, Clone, Eq, PartialEq, thiserror::Error)]
pub enum GasError {
    #[error("invalid gas amount {0} > max = {}", Gas::MAX)]
    Limit(u64),
    #[error("{0}")]
    Parse(#[from] ParseIntError),
    #[error("invalid gas amount {0} < 0")]
    Negative(i64),
}

#[cfg(test)]
mod tests {
    use extensions::testing::UnwrapTesting;

    use super::*;

    #[test]
    fn gas_try_from_into_u64() {
        let gas = Gas::try_from(100_u64).unwrap_test();
        assert_eq!(u64::from(gas), 100);
    }

    #[test]
    fn test_gas_try_from_error() {
        let mut raw_gas: u64 = u63::MAX.into();
        raw_gas += 1;
        let gas = Gas::try_from(raw_gas);
        assert!(gas.is_err());
    }

    #[test]
    fn test_gas_try_from_limit_ok() {
        let raw_gas: u64 = u63::MAX.into();
        let gas = Gas::try_from(raw_gas).unwrap_test();
        assert_eq!(gas, Gas::MAX);
    }

    #[test]
    fn test_gas_from_str() {
        let gas = Gas::from_str("100").unwrap_test();
        assert_eq!(gas, u63::new(100).into());
    }

    #[test]
    fn test_gas_from_str_err() {
        let gas = Gas::from_str("-100");
        assert!(gas.is_err());
    }

    #[test]
    fn test_gas_into_i64() {
        let gas: Gas = u63::new(100).into();
        assert_eq!(i64::from(gas), 100);
    }
}
