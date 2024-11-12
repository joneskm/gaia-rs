use crate::errors::ValidatorHistoricalRewardsReferenceCountError;
use gears::{
    core::{errors::CoreError, Protobuf},
    types::{
        address::ValAddress,
        base::{
            coin::{DecimalCoin, DecimalCoinRaw},
            coins::DecimalCoins,
            errors::CoinsError,
        },
        decimal256::Decimal256,
        errors::StdError,
    },
};
use prost::Message;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// ValidatorOutstandingRewardsRecord is used for import/export via genesis json.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ValidatorOutstandingRewardsRecord {
    /// validator_address is the address of the validator.
    pub validator_address: ValAddress,
    /// outstanding_rewards represents the oustanding rewards of a validator.
    pub outstanding_rewards: ValidatorOutstandingRewards,
}

#[derive(Clone, PartialEq, Serialize, Message)]
pub struct ValidatorOutstandingRewardsRaw {
    #[prost(message, repeated)]
    pub rewards: Vec<DecimalCoinRaw>,
}

impl From<ValidatorOutstandingRewards> for ValidatorOutstandingRewardsRaw {
    fn from(ValidatorOutstandingRewards { rewards }: ValidatorOutstandingRewards) -> Self {
        Self {
            rewards: rewards.into_inner().into_iter().map(Into::into).collect(),
        }
    }
}

/// ValidatorOutstandingRewards represents outstanding (un-withdrawn) rewards
/// for a validator inexpensive to track, allows simple sanity checks.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(try_from = "Vec<DecimalCoin>")]
pub struct ValidatorOutstandingRewards {
    pub rewards: DecimalCoins,
}

impl TryFrom<Vec<DecimalCoin>> for ValidatorOutstandingRewards {
    type Error = CoinsError;

    fn try_from(value: Vec<DecimalCoin>) -> Result<Self, Self::Error> {
        Ok(Self {
            rewards: DecimalCoins::new(value)?,
        })
    }
}

impl TryFrom<ValidatorOutstandingRewardsRaw> for ValidatorOutstandingRewards {
    type Error = CoreError;
    fn try_from(
        ValidatorOutstandingRewardsRaw { rewards }: ValidatorOutstandingRewardsRaw,
    ) -> Result<Self, Self::Error> {
        let mut coins = vec![];
        for coin in rewards {
            coins.push(coin.try_into()?);
        }
        let rewards = DecimalCoins::new(coins).map_err(|e| CoreError::Coin(e.to_string()))?;
        Ok(Self { rewards })
    }
}

impl Protobuf<ValidatorOutstandingRewardsRaw> for ValidatorOutstandingRewards {}

/// ValidatorAccumulatedCommissionRecord is used for import / export via genesis json.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ValidatorAccumulatedCommissionRecord {
    /// validator_address is the address of the validator.
    pub validator_address: ValAddress,
    /// accumulated is the accumulated commission of a validator.
    pub accumulated: ValidatorAccumulatedCommission,
}

#[derive(Clone, PartialEq, Serialize, Message)]
pub struct ValidatorAccumulatedCommissionRaw {
    #[prost(message, repeated)]
    pub commission: Vec<DecimalCoinRaw>,
}

impl From<ValidatorAccumulatedCommission> for ValidatorAccumulatedCommissionRaw {
    fn from(ValidatorAccumulatedCommission { commission }: ValidatorAccumulatedCommission) -> Self {
        Self {
            commission: commission
                .into_inner()
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}

/// ValidatorAccumulatedCommission represents accumulated commission
/// for a validator kept as a running counter, can be withdrawn at any time.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(try_from = "Vec<DecimalCoin>")]
pub struct ValidatorAccumulatedCommission {
    pub commission: DecimalCoins,
}

impl TryFrom<Vec<DecimalCoin>> for ValidatorAccumulatedCommission {
    type Error = CoinsError;

    fn try_from(value: Vec<DecimalCoin>) -> Result<Self, Self::Error> {
        Ok(Self {
            commission: DecimalCoins::new(value)?,
        })
    }
}

impl TryFrom<ValidatorAccumulatedCommissionRaw> for ValidatorAccumulatedCommission {
    type Error = CoreError;
    fn try_from(
        ValidatorAccumulatedCommissionRaw { commission }: ValidatorAccumulatedCommissionRaw,
    ) -> Result<Self, Self::Error> {
        let mut coins = vec![];
        for coin in commission {
            coins.push(coin.try_into()?);
        }
        let commission = DecimalCoins::new(coins).map_err(|e| CoreError::Coin(e.to_string()))?;
        Ok(Self { commission })
    }
}

impl Protobuf<ValidatorAccumulatedCommissionRaw> for ValidatorAccumulatedCommission {}

/// ValidatorHistoricalRewardsRecord is used for import / export via genesis json.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ValidatorHistoricalRewardsRecord {
    /// validator_address is the address of the validator.
    pub validator_address: ValAddress,
    /// period defines the period the historical rewards apply to.
    pub period: u64,
    /// rewards defines the historical rewards of a validator.
    pub rewards: ValidatorHistoricalRewards,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ValidatorHistoricalRewardsReferenceCount {
    // inherits sdk type
    counter: u32,
    upper_bound: u32,
}

impl ValidatorHistoricalRewardsReferenceCount {
    const DEFAULT_UPPER_LIMIT: u32 = 2;

    pub fn new(counter: u32) -> Result<Self, ValidatorHistoricalRewardsReferenceCountError> {
        if counter > Self::DEFAULT_UPPER_LIMIT {
            return Err(
                ValidatorHistoricalRewardsReferenceCountError::CounterValueOutOfBounds(
                    counter,
                    Self::DEFAULT_UPPER_LIMIT,
                ),
            );
        }
        Ok(Self {
            counter,
            upper_bound: Self::DEFAULT_UPPER_LIMIT,
        })
    }

    pub fn counter(&self) -> u32 {
        self.counter
    }

    pub fn is_zero(&self) -> bool {
        self.counter == 0
    }

    pub fn increment(&mut self) -> Result<u32, ValidatorHistoricalRewardsReferenceCountError> {
        if self.counter < self.upper_bound {
            self.counter += 1;
            Ok(self.counter)
        } else {
            Err(ValidatorHistoricalRewardsReferenceCountError::IncrementBound(self.upper_bound))
        }
    }

    pub fn decrement(&mut self) -> Result<u32, ValidatorHistoricalRewardsReferenceCountError> {
        if self.counter > 0 {
            self.counter -= 1;
            Ok(self.counter)
        } else {
            Err(ValidatorHistoricalRewardsReferenceCountError::DecrementBound)
        }
    }
}

#[derive(Clone, PartialEq, Message)]
pub struct ValidatorHistoricalRewardsRaw {
    #[prost(message, repeated)]
    pub cumulative_reward_ratio: Vec<DecimalCoinRaw>,
    #[prost(uint32)]
    pub reference_count: u32,
}

impl From<ValidatorHistoricalRewards> for ValidatorHistoricalRewardsRaw {
    fn from(
        ValidatorHistoricalRewards {
            cumulative_reward_ratio,
            reference_count,
        }: ValidatorHistoricalRewards,
    ) -> Self {
        Self {
            cumulative_reward_ratio: cumulative_reward_ratio
                .into_inner()
                .into_iter()
                .map(Into::into)
                .collect(),
            reference_count: reference_count.counter(),
        }
    }
}

/// ValidatorHistoricalRewards represents historical rewards for a validator.
/// Height is implicit within the store key.
/// Cumulative reward ratio is the sum from the zeroeth period
/// until this period of rewards / tokens, per the spec.
/// The reference count indicates the number of objects
/// which might need to reference this historical entry at any point.
/// ReferenceCount =
///    number of outstanding delegations which ended the associated period (and
///    might need to read that record)
///  + number of slashes which ended the associated period (and might need to read that record)
///  + one per validator for the zeroeth period, set on initialization
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
// TODO: add serde(try_from) to check coins during genesis
pub struct ValidatorHistoricalRewards {
    pub cumulative_reward_ratio: DecimalCoins,
    pub reference_count: ValidatorHistoricalRewardsReferenceCount,
}

impl TryFrom<ValidatorHistoricalRewardsRaw> for ValidatorHistoricalRewards {
    type Error = CoreError;
    fn try_from(
        ValidatorHistoricalRewardsRaw {
            cumulative_reward_ratio,
            reference_count,
        }: ValidatorHistoricalRewardsRaw,
    ) -> Result<Self, Self::Error> {
        let mut coins = vec![];
        for coin in cumulative_reward_ratio {
            coins.push(coin.try_into()?);
        }
        let cumulative_reward_ratio =
            DecimalCoins::new(coins).map_err(|e| CoreError::Coin(e.to_string()))?;
        Ok(Self {
            cumulative_reward_ratio,
            reference_count: ValidatorHistoricalRewardsReferenceCount::new(reference_count)
                .map_err(|e| CoreError::Custom(e.to_string()))?,
        })
    }
}

impl Protobuf<ValidatorHistoricalRewardsRaw> for ValidatorHistoricalRewards {}

/// ValidatorCurrentRewardsRecord is used for import / export via genesis json.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ValidatorCurrentRewardsRecord {
    /// validator_address is the address of the validator.
    pub validator_address: ValAddress,
    /// rewards defines the current rewards of a validator.
    pub rewards: ValidatorCurrentRewards,
}

#[derive(Clone, PartialEq, Message)]
pub struct ValidatorCurrentRewardsRaw {
    #[prost(message, repeated)]
    pub rewards: Vec<DecimalCoinRaw>,
    #[prost(uint64)]
    pub period: u64,
}

impl From<ValidatorCurrentRewards> for ValidatorCurrentRewardsRaw {
    fn from(ValidatorCurrentRewards { rewards, period }: ValidatorCurrentRewards) -> Self {
        Self {
            rewards: rewards.into_inner().into_iter().map(Into::into).collect(),
            period,
        }
    }
}

/// ValidatorCurrentRewards represents current rewards and current
/// period for a validator kept as a running counter and incremented
/// each block as long as the validator's tokens remain constant.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
// TODO: add serde(try_from) to check coins during genesis
pub struct ValidatorCurrentRewards {
    pub rewards: DecimalCoins,
    pub period: u64,
}

impl TryFrom<ValidatorCurrentRewardsRaw> for ValidatorCurrentRewards {
    type Error = CoreError;
    fn try_from(
        ValidatorCurrentRewardsRaw { rewards, period }: ValidatorCurrentRewardsRaw,
    ) -> Result<Self, Self::Error> {
        let mut coins = vec![];
        for coin in rewards {
            coins.push(coin.try_into()?);
        }
        let rewards = DecimalCoins::new(coins).map_err(|e| CoreError::Coin(e.to_string()))?;
        Ok(Self { rewards, period })
    }
}

impl Protobuf<ValidatorCurrentRewardsRaw> for ValidatorCurrentRewards {}

/// ValidatorSlashEventRecord is used for import / export via genesis json.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ValidatorSlashEventRecord {
    /// validator_address is the address of the validator.
    pub validator_address: ValAddress,
    /// height defines the block height at which the slash event occured.
    pub height: u64,
    /// period is the period of the slash event.
    pub period: u64,
    /// validator_slash_event describes the slash event.
    pub validator_slash_event: ValidatorSlashEvent,
}

#[derive(Clone, PartialEq, Serialize, Message)]
pub struct ValidatorSlashEventRaw {
    #[prost(uint64)]
    pub validator_period: u64,
    #[prost(string)]
    pub fraction: String,
}

impl From<ValidatorSlashEvent> for ValidatorSlashEventRaw {
    fn from(
        ValidatorSlashEvent {
            validator_period,
            fraction,
        }: ValidatorSlashEvent,
    ) -> Self {
        Self {
            validator_period,
            fraction: fraction.to_string(),
        }
    }
}

/// ValidatorSlashEvent represents a validator slash event.
/// Height is implicit within the store key.
/// This is needed to calculate appropriate amount of staking tokens
/// for delegations which are withdrawn after a slash has occurred.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ValidatorSlashEvent {
    pub validator_period: u64,
    pub fraction: Decimal256,
}

impl TryFrom<ValidatorSlashEventRaw> for ValidatorSlashEvent {
    type Error = StdError;
    fn try_from(
        ValidatorSlashEventRaw {
            validator_period,
            fraction,
        }: ValidatorSlashEventRaw,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            validator_period,
            fraction: Decimal256::from_str(&fraction)?,
        })
    }
}

impl Protobuf<ValidatorSlashEventRaw> for ValidatorSlashEvent {}
