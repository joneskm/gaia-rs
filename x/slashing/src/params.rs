use anyhow::anyhow;
use gears::{
    context::{InfallibleContext, InfallibleContextMut, QueryableContext, TransactionalContext},
    core::{serializers::serialize_number_to_string, Protobuf},
    extensions::corruption::UnwrapCorrupt,
    gas::store::errors::GasStoreErrors,
    params::{
        gas, infallible_subspace, infallible_subspace_mut, ParamKind, ParamsDeserialize,
        ParamsSerialize, ParamsSubspaceKey,
    },
    store::{database::Database, StoreKey},
    types::{
        decimal256::{Decimal256, PRECISION_REUSE},
        errors::StdError,
        uint::Uint256,
    },
};
use prost::Message;
use serde::{Deserialize, Serialize};
use serde_aux::prelude::deserialize_number_from_string;
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

const KEY_SIGNED_BLOCKS_WINDOW: &str = "SignedBlocksWindow";
const KEY_MIN_SIGNED_PER_WINDOW: &str = "MinSignedPerWindow";
const KEY_DOWNTIME_JAIL_DURATION: &str = "DowntimeJailDuration";
const KEY_SLASH_FRACTION_DOUBLE_SIGN: &str = "SlashFractionDoubleSign";
const KEY_SLASH_FRACTION_DOWNTIME: &str = "SlashFractionDowntime";

#[derive(Clone, Serialize, Message)]
pub struct SlashingParamsRaw {
    #[prost(int64, tag = "1")]
    pub signed_blocks_window: i64,
    #[prost(string, tag = "2")]
    pub min_signed_per_window: String,
    #[prost(int64, tag = "3")]
    pub downtime_jail_duration: i64,
    #[prost(string, tag = "4")]
    pub slash_fraction_double_sign: String,
    #[prost(string, tag = "5")]
    pub slash_fraction_downtime: String,
}

impl From<SlashingParams> for SlashingParamsRaw {
    fn from(value: SlashingParams) -> Self {
        Self {
            signed_blocks_window: value.signed_blocks_window,
            min_signed_per_window: value.min_signed_per_window.to_string(),
            downtime_jail_duration: value.downtime_jail_duration,
            slash_fraction_double_sign: value.slash_fraction_double_sign.to_string(),
            slash_fraction_downtime: value.slash_fraction_downtime.to_string(),
        }
    }
}

/// SlashingParams represents the parameters used for by the slashing module.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SlashingParams {
    #[serde(serialize_with = "serialize_number_to_string")]
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub signed_blocks_window: i64,
    pub min_signed_per_window: Decimal256,
    #[serde(serialize_with = "serialize_number_to_string")]
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub downtime_jail_duration: i64, //TODO: why isn't this a Duration?
    pub slash_fraction_double_sign: Decimal256,
    pub slash_fraction_downtime: Decimal256,
}

impl SlashingParams {
    pub fn min_signed_per_window_u32(&self) -> anyhow::Result<u32> {
        // NOTE: RoundInt64 will never panic as minSignedPerWindow is less than 1.
        let mul = self
            .min_signed_per_window
            .checked_mul(Decimal256::from_atomics(self.signed_blocks_window as u64, 0).unwrap())?;
        // get Uint256 representation and cut fractional part
        let full = mul.atomics().div_ceil(PRECISION_REUSE);
        if full <= Uint256::from(u32::MAX) {
            // get fractional part that is equivalent to PRECISION_REUSE * 10^(-1), i.e. 10^17
            let fraction_with_first_decimal = PRECISION_REUSE
                .checked_div(Decimal256::from_atomics(10u64, 0).unwrap())
                .unwrap();
            let full_dec = mul.atomics().div_ceil(fraction_with_first_decimal);
            if full
                .mul_ceil(Decimal256::from_atomics(10u64, 0).unwrap())
                .wrapping_sub(full_dec)
                >= Uint256::from(5u64)
            {
                return Ok(full.wrapping_sub(1u64.into()).to_string().parse::<u32>()?);
            } else {
                return Ok(full.to_string().parse::<u32>()?);
            }
        }

        Err(anyhow!(
            "Cannot convert `min_signed_per_window` value to u32".to_string()
        ))
    }
}

impl TryFrom<SlashingParamsRaw> for SlashingParams {
    type Error = StdError;

    fn try_from(value: SlashingParamsRaw) -> Result<Self, Self::Error> {
        Ok(Self {
            signed_blocks_window: value.signed_blocks_window,
            min_signed_per_window: Decimal256::from_str(&value.min_signed_per_window)?,
            downtime_jail_duration: value.downtime_jail_duration,
            slash_fraction_double_sign: Decimal256::from_str(&value.slash_fraction_double_sign)?,
            slash_fraction_downtime: Decimal256::from_str(&value.slash_fraction_downtime)?,
        })
    }
}

impl Protobuf<SlashingParamsRaw> for SlashingParams {}

impl ParamsSerialize for SlashingParams {
    fn keys() -> HashSet<&'static str> {
        [
            KEY_SIGNED_BLOCKS_WINDOW,
            KEY_MIN_SIGNED_PER_WINDOW,
            KEY_DOWNTIME_JAIL_DURATION,
            KEY_SLASH_FRACTION_DOUBLE_SIGN,
            KEY_SLASH_FRACTION_DOWNTIME,
        ]
        .into_iter()
        .collect()
    }

    fn to_raw(&self) -> Vec<(&'static str, Vec<u8>)> {
        let mut raws = Vec::with_capacity(5);
        raws.push((
            KEY_SIGNED_BLOCKS_WINDOW,
            // TODO: check during integration of genesis
            format!("\"{}\"", self.signed_blocks_window).into_bytes(),
        ));
        raws.push((
            // TODO: check during integration of genesis
            KEY_MIN_SIGNED_PER_WINDOW,
            self.min_signed_per_window.to_string().into_bytes(),
        ));
        raws.push((
            KEY_DOWNTIME_JAIL_DURATION,
            // TODO: check during integration of genesis
            format!("\"{}\"", self.downtime_jail_duration).into_bytes(),
        ));
        raws.push((
            KEY_SLASH_FRACTION_DOUBLE_SIGN,
            self.slash_fraction_double_sign.to_string().into_bytes(),
        ));
        raws.push((
            KEY_SLASH_FRACTION_DOWNTIME,
            self.slash_fraction_downtime.to_string().into_bytes(),
        ));
        raws
    }
}

impl ParamsDeserialize for SlashingParams {
    fn from_raw(mut fields: HashMap<&'static str, Vec<u8>>) -> Self {
        Self {
            signed_blocks_window: ParamKind::I64
                .parse_param(fields.remove(KEY_SIGNED_BLOCKS_WINDOW).unwrap_or_corrupt())
                .signed_64()
                .unwrap_or_corrupt(),
            min_signed_per_window: Decimal256::from_str(
                &String::from_utf8(
                    ParamKind::Bytes
                        .parse_param(fields.remove(KEY_MIN_SIGNED_PER_WINDOW).unwrap_or_corrupt())
                        .bytes()
                        .unwrap_or_corrupt(),
                )
                .unwrap_or_corrupt(),
            )
            .unwrap_or_corrupt(),
            downtime_jail_duration: ParamKind::I64
                .parse_param(
                    fields
                        .remove(KEY_DOWNTIME_JAIL_DURATION)
                        .unwrap_or_corrupt(),
                )
                .signed_64()
                .unwrap_or_corrupt(),
            slash_fraction_double_sign: Decimal256::from_str(
                &String::from_utf8(
                    ParamKind::Bytes
                        .parse_param(
                            fields
                                .remove(KEY_SLASH_FRACTION_DOUBLE_SIGN)
                                .unwrap_or_corrupt(),
                        )
                        .bytes()
                        .unwrap_or_corrupt(),
                )
                .unwrap_or_corrupt(),
            )
            .unwrap_or_corrupt(),
            slash_fraction_downtime: Decimal256::from_str(
                &String::from_utf8(
                    ParamKind::Bytes
                        .parse_param(
                            fields
                                .remove(KEY_SLASH_FRACTION_DOWNTIME)
                                .unwrap_or_corrupt(),
                        )
                        .bytes()
                        .unwrap_or_corrupt(),
                )
                .unwrap_or_corrupt(),
            )
            .unwrap_or_corrupt(),
        }
    }
}

impl Default for SlashingParams {
    fn default() -> Self {
        // TODO: check defaults, especially with division
        Self {
            signed_blocks_window: 100,
            min_signed_per_window: Decimal256::from_atomics(5u64, 1).expect("default is valid"),
            downtime_jail_duration: 60 * 10 * 1_000_000_000,
            slash_fraction_double_sign: Decimal256::one()
                .checked_div(Decimal256::from_atomics(20u64, 0).expect("default is valid"))
                .expect("default is valid"),
            slash_fraction_downtime: Decimal256::one()
                .checked_div(Decimal256::from_atomics(100u64, 0).expect("default is valid"))
                .expect("default is valid"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SlashingParamsKeeper<PSK: ParamsSubspaceKey> {
    pub params_subspace_key: PSK,
}

impl<PSK: ParamsSubspaceKey> SlashingParamsKeeper<PSK> {
    pub fn get<DB: Database, SK: StoreKey, CTX: InfallibleContext<DB, SK>>(
        &self,
        ctx: &CTX,
    ) -> SlashingParams {
        let store = infallible_subspace(ctx, &self.params_subspace_key);
        store.params().unwrap_or(SlashingParams::default())
    }

    pub fn try_get<DB: Database, SK: StoreKey, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
    ) -> Result<SlashingParams, GasStoreErrors> {
        let store = gas::subspace(ctx, &self.params_subspace_key);

        Ok(store.params()?.unwrap_or(SlashingParams::default()))
    }

    pub fn set<DB: Database, SK: StoreKey, KV: InfallibleContextMut<DB, SK>>(
        &self,
        ctx: &mut KV,
        params: SlashingParams,
    ) {
        let mut store = infallible_subspace_mut(ctx, &self.params_subspace_key);
        store.params_set(&params)
    }

    pub fn try_set<DB: Database, SK: StoreKey, KV: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut KV,
        params: SlashingParams,
    ) -> Result<(), GasStoreErrors> {
        let mut store = gas::subspace_mut(ctx, &self.params_subspace_key);
        store.params_set(&params)
    }
}
