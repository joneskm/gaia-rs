use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

use gears::tendermint::types::time::duration::serde_with::{
    deserialize_duration_from_nanos_string, serialize_duration_to_nanos_string,
};
use gears::{
    application::keepers::params::ParamsKeeper,
    core::{errors::CoreError, Protobuf},
    error::ProtobufError,
    params::{ParamsDeserialize, ParamsSerialize, ParamsSubspaceKey},
    tendermint::types::time::duration::Duration,
    types::{
        base::{
            coin::UnsignedCoin,
            coins::{Coins, UnsignedCoins},
        },
        decimal256::Decimal256,
    },
};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::errors::{EXISTS, SERDE_JSON_CONVERSION};

const KEY_DEPOSIT_PARAMS: &str = "depositparams";
const KEY_VOTING_PARAMS: &str = "votingparams";
const KEY_TALLY_PARAMS: &str = "tallyparams";

const DEFAULT_PERIOD: Duration = Duration::new_from_secs(172800); // 2 days

mod environment;

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DepositParams {
    pub min_deposit: UnsignedCoins,
    #[serde(serialize_with = "serialize_duration_to_nanos_string")]
    #[serde(deserialize_with = "deserialize_duration_from_nanos_string")]
    pub max_deposit_period: Duration, // ?
}

impl Default for DepositParams {
    fn default() -> Self {
        Self {
            min_deposit: UnsignedCoins::new(vec![UnsignedCoin::from_str(
                environment::DEFAULT_MIN_DEPOSIT,
            )
            .expect("default is valid")])
            .expect("default is valid"),
            max_deposit_period: DEFAULT_PERIOD,
        }
    }
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VotingParams {
    #[serde(serialize_with = "serialize_duration_to_nanos_string")]
    #[serde(deserialize_with = "deserialize_duration_from_nanos_string")]
    pub voting_period: Duration,
}

impl Default for VotingParams {
    fn default() -> Self {
        Self {
            voting_period: DEFAULT_PERIOD,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TallyParams {
    pub quorum: Decimal256,
    pub threshold: Decimal256,
    pub veto_threshold: Decimal256,
}

impl Default for TallyParams {
    fn default() -> Self {
        Self {
            // TODO: change back to original defaults once Decimal256 formatting is fixed (currently
            // 0.334000000000000000 is being formatted as 0.334)
            //quorum: Decimal256::from_atomics(334_u16, 3).expect("Default should be valid"),
            quorum: Decimal256::from_atomics(334000000000000001_u64, 18)
                .expect("Default should be valid"),
            //threshold: Decimal256::from_atomics(5_u8, 1).expect("Default should be valid"),
            threshold: Decimal256::from_atomics(500000000000000001_u64, 18)
                .expect("Default should be valid"),
            //veto_threshold: Decimal256::from_atomics(334_u16, 3).expect("Default should be valid"),
            veto_threshold: Decimal256::from_atomics(334000000000000001_u64, 18)
                .expect("Default should be valid"),
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct GovParams {
    pub deposit: DepositParams,
    pub voting: VotingParams,
    pub tally: TallyParams,
}

impl ParamsSerialize for GovParams {
    fn keys() -> HashSet<&'static str> {
        [KEY_DEPOSIT_PARAMS, KEY_VOTING_PARAMS, KEY_TALLY_PARAMS]
            .into_iter()
            .collect()
    }

    fn to_raw(&self) -> Vec<(&'static str, Vec<u8>)> {
        vec![
            (
                KEY_DEPOSIT_PARAMS,
                serde_json::to_vec(&self.deposit).expect(SERDE_JSON_CONVERSION),
            ),
            (
                KEY_VOTING_PARAMS,
                serde_json::to_vec(&self.voting).expect(SERDE_JSON_CONVERSION),
            ),
            (
                KEY_TALLY_PARAMS,
                serde_json::to_vec(&self.tally).expect(SERDE_JSON_CONVERSION),
            ),
        ]
    }
}

impl ParamsDeserialize for GovParams {
    fn from_raw(mut fields: HashMap<&'static str, Vec<u8>>) -> Self {
        Self {
            deposit: serde_json::from_slice(&fields.remove(KEY_DEPOSIT_PARAMS).expect(EXISTS))
                .expect(SERDE_JSON_CONVERSION),
            voting: serde_json::from_slice(&fields.remove(KEY_VOTING_PARAMS).expect(EXISTS))
                .expect(SERDE_JSON_CONVERSION),
            tally: serde_json::from_slice(&fields.remove(KEY_TALLY_PARAMS).expect(EXISTS))
                .expect(SERDE_JSON_CONVERSION),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GovParamsKeeper<PSK: ParamsSubspaceKey> {
    pub params_subspace_key: PSK,
}

impl<PSK: ParamsSubspaceKey> ParamsKeeper<PSK> for GovParamsKeeper<PSK> {
    type Param = GovParams;

    fn psk(&self) -> &PSK {
        &self.params_subspace_key
    }

    fn validate(key: impl AsRef<[u8]>, value: impl AsRef<[u8]>) -> bool {
        match String::from_utf8_lossy(key.as_ref()).as_ref() {
            KEY_DEPOSIT_PARAMS => serde_json::from_slice::<DepositParams>(value.as_ref()).is_ok(),
            KEY_VOTING_PARAMS => serde_json::from_slice::<VotingParams>(value.as_ref()).is_ok(),
            KEY_TALLY_PARAMS => serde_json::from_slice::<TallyParams>(value.as_ref()).is_ok(),
            _ => false,
        }
    }
}

mod inner {
    pub use ibc_proto::cosmos::gov::v1beta1::DepositParams;
    pub use ibc_proto::cosmos::gov::v1beta1::TallyParams;
    pub use ibc_proto::cosmos::gov::v1beta1::VotingParams;
    pub use ibc_proto::google::protobuf::Duration;
}

impl TryFrom<inner::DepositParams> for DepositParams {
    type Error = CoreError;

    fn try_from(
        inner::DepositParams {
            min_deposit,
            max_deposit_period,
        }: inner::DepositParams,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            min_deposit: {
                let mut result = Vec::with_capacity(min_deposit.len());

                for coin in min_deposit {
                    result.push(coin.try_into().map_err(
                        |e: gears::types::base::errors::CoinError| CoreError::Coin(e.to_string()),
                    )?)
                }

                Coins::new(result).map_err(|e| CoreError::Coins(e.to_string()))?
            },
            max_deposit_period: {
                let duration = max_deposit_period.ok_or(CoreError::MissingField(
                    "DepositParams: field `max_deposit_period`".to_owned(),
                ))?;

                Duration::try_new(duration.seconds, duration.nanos).map_err(|e| {
                    CoreError::MissingField(format!(
                        "DepositParams: field `max_deposit_period`: {}",
                        e,
                    ))
                })?
            },
        })
    }
}

impl From<DepositParams> for inner::DepositParams {
    fn from(
        DepositParams {
            min_deposit,
            max_deposit_period,
        }: DepositParams,
    ) -> Self {
        Self {
            min_deposit: min_deposit.into_iter().map(|e| e.into()).collect(),
            max_deposit_period: Some(inner::Duration {
                seconds: max_deposit_period.duration_seconds().into(),
                nanos: max_deposit_period.nanoseconds().into(),
            }),
        }
    }
}

impl Protobuf<inner::DepositParams> for DepositParams {}

impl TryFrom<inner::TallyParams> for TallyParams {
    type Error = CoreError;

    fn try_from(
        inner::TallyParams {
            quorum: _,
            threshold: _,
            veto_threshold: _,
        }: inner::TallyParams,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            quorum: Decimal256::one(),
            threshold: Decimal256::one(),
            veto_threshold: Decimal256::one(),
        }) // TODO:NOW
    }
}

impl From<TallyParams> for inner::TallyParams {
    fn from(_value: TallyParams) -> Self {
        todo!()
    }
}

impl Protobuf<inner::TallyParams> for TallyParams {}

impl TryFrom<inner::VotingParams> for VotingParams {
    type Error = ProtobufError;

    fn try_from(
        inner::VotingParams { voting_period }: inner::VotingParams,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            voting_period: {
                let duration = voting_period.ok_or(CoreError::MissingField(
                    "VotingParams: field `voting_period`".to_owned(),
                ))?;

                Duration::try_new(duration.seconds, duration.nanos)
                    .map_err(|err| anyhow::anyhow!("failed to map duration: {err}"))?
            },
        })
    }
}

impl From<VotingParams> for inner::VotingParams {
    fn from(VotingParams { voting_period }: VotingParams) -> Self {
        Self {
            voting_period: Some(inner::Duration {
                seconds: voting_period.duration_seconds().into(),
                nanos: voting_period.nanoseconds().into(),
            }),
        }
    }
}

impl Protobuf<inner::VotingParams> for VotingParams {}
