use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

use gears::{
    context::{InfallibleContext, InfallibleContextMut, QueryableContext, TransactionalContext},
    core::Protobuf,
    extensions::corruption::UnwrapCorrupt,
    gas::store::errors::GasStoreErrors,
    params::{
        gas, infallible_subspace, infallible_subspace_mut, ParamKind, ParamsDeserialize,
        ParamsSerialize, ParamsSubspaceKey,
    },
    store::{database::Database, StoreKey},
    types::{decimal256::Decimal256, errors::StdError},
};
use prost::Message;
use serde::{Deserialize, Serialize};

const KEY_COMMUNITY_TAX: &str = "communitytax";
const KEY_BASE_PROPOSER_REWARD: &str = "baseproposerreward";
const KEY_BONUS_PROPOSER_REWARD: &str = "bonusproposerreward";
const KEY_WITHDRAW_ADDR_ENABLED: &str = "withdrawaddrenabled";

#[derive(Clone, Serialize, Message)]
pub struct DistributionParamsRaw {
    #[prost(string, tag = "1")]
    pub community_tax: String,
    #[prost(string, tag = "2")]
    pub base_proposer_reward: String,
    #[prost(string, tag = "3")]
    pub bonus_proposer_reward: String,
    #[prost(bool, tag = "4")]
    pub withdraw_addr_enabled: bool,
}

impl From<DistributionParams> for DistributionParamsRaw {
    fn from(
        DistributionParams {
            community_tax,
            base_proposer_reward,
            bonus_proposer_reward,
            withdraw_addr_enabled,
        }: DistributionParams,
    ) -> Self {
        Self {
            community_tax: community_tax.to_string(),
            base_proposer_reward: base_proposer_reward.to_string(),
            bonus_proposer_reward: bonus_proposer_reward.to_string(),
            withdraw_addr_enabled,
        }
    }
}

/// DistributionParams represents the parameters used for by the distribution module.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DistributionParams {
    pub community_tax: Decimal256,
    pub base_proposer_reward: Decimal256,
    pub bonus_proposer_reward: Decimal256,
    pub withdraw_addr_enabled: bool,
}

impl TryFrom<DistributionParamsRaw> for DistributionParams {
    type Error = StdError;
    fn try_from(
        DistributionParamsRaw {
            community_tax,
            base_proposer_reward,
            bonus_proposer_reward,
            withdraw_addr_enabled,
        }: DistributionParamsRaw,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            community_tax: Decimal256::from_str(&community_tax)?,
            base_proposer_reward: Decimal256::from_str(&base_proposer_reward)?,
            bonus_proposer_reward: Decimal256::from_str(&bonus_proposer_reward)?,
            withdraw_addr_enabled,
        })
    }
}

impl Protobuf<DistributionParamsRaw> for DistributionParams {}

impl ParamsSerialize for DistributionParams {
    fn keys() -> HashSet<&'static str> {
        [
            KEY_COMMUNITY_TAX,
            KEY_BASE_PROPOSER_REWARD,
            KEY_BONUS_PROPOSER_REWARD,
            KEY_WITHDRAW_ADDR_ENABLED,
        ]
        .into_iter()
        .collect()
    }

    fn to_raw(&self) -> Vec<(&'static str, Vec<u8>)> {
        let mut raws = Vec::with_capacity(5);
        // TODO: check all parameters serialization during integration of genesis
        raws.push((
            KEY_COMMUNITY_TAX,
            self.community_tax.to_string().into_bytes(),
        ));
        raws.push((
            KEY_BASE_PROPOSER_REWARD,
            self.base_proposer_reward.to_string().into_bytes(),
        ));
        raws.push((
            KEY_BONUS_PROPOSER_REWARD,
            self.bonus_proposer_reward.to_string().into_bytes(),
        ));
        raws.push((
            KEY_WITHDRAW_ADDR_ENABLED,
            self.withdraw_addr_enabled.to_string().into_bytes(),
        ));
        raws
    }
}

impl ParamsDeserialize for DistributionParams {
    fn from_raw(mut fields: HashMap<&'static str, Vec<u8>>) -> Self {
        Self {
            community_tax: Decimal256::from_str(
                &String::from_utf8(
                    ParamKind::Bytes
                        .parse_param(fields.remove(KEY_COMMUNITY_TAX).unwrap_or_corrupt())
                        .bytes()
                        .unwrap_or_corrupt(),
                )
                .unwrap_or_corrupt(),
            )
            .unwrap_or_corrupt(),
            base_proposer_reward: Decimal256::from_str(
                &String::from_utf8(
                    ParamKind::Bytes
                        .parse_param(fields.remove(KEY_BASE_PROPOSER_REWARD).unwrap_or_corrupt())
                        .bytes()
                        .unwrap_or_corrupt(),
                )
                .unwrap_or_corrupt(),
            )
            .unwrap_or_corrupt(),
            bonus_proposer_reward: Decimal256::from_str(
                &String::from_utf8(
                    ParamKind::Bytes
                        .parse_param(fields.remove(KEY_BONUS_PROPOSER_REWARD).unwrap_or_corrupt())
                        .bytes()
                        .unwrap_or_corrupt(),
                )
                .unwrap_or_corrupt(),
            )
            .unwrap_or_corrupt(),
            withdraw_addr_enabled: ParamKind::Bool
                .parse_param(fields.remove(KEY_WITHDRAW_ADDR_ENABLED).unwrap_or_corrupt())
                .boolean()
                .unwrap_or_corrupt(),
        }
    }
}

impl Default for DistributionParams {
    fn default() -> Self {
        Self {
            community_tax: Decimal256::from_atomics(2u64, 2).expect("hardcoded value cannot fail"),
            base_proposer_reward: Decimal256::from_atomics(1u64, 2)
                .expect("hardcoded value cannot fail"),
            bonus_proposer_reward: Decimal256::from_atomics(4u64, 2)
                .expect("hardcoded value cannot fail"),
            withdraw_addr_enabled: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DistributionParamsKeeper<PSK: ParamsSubspaceKey> {
    pub params_subspace_key: PSK,
}

#[allow(dead_code)]
impl<PSK: ParamsSubspaceKey> DistributionParamsKeeper<PSK> {
    pub fn get<DB: Database, SK: StoreKey, CTX: InfallibleContext<DB, SK>>(
        &self,
        ctx: &CTX,
    ) -> DistributionParams {
        let store = infallible_subspace(ctx, &self.params_subspace_key);
        store.params().unwrap_or(DistributionParams::default())
    }

    pub fn try_get<DB: Database, SK: StoreKey, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
    ) -> Result<DistributionParams, GasStoreErrors> {
        let store = gas::subspace(ctx, &self.params_subspace_key);

        Ok(store.params()?.unwrap_or(DistributionParams::default()))
    }

    pub fn set<DB: Database, SK: StoreKey, KV: InfallibleContextMut<DB, SK>>(
        &self,
        ctx: &mut KV,
        params: DistributionParams,
    ) {
        let mut store = infallible_subspace_mut(ctx, &self.params_subspace_key);
        store.params_set(&params)
    }

    pub fn try_set<DB: Database, SK: StoreKey, KV: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut KV,
        params: DistributionParams,
    ) -> Result<(), GasStoreErrors> {
        let mut store = gas::subspace_mut(ctx, &self.params_subspace_key);
        store.params_set(&params)
    }
}
