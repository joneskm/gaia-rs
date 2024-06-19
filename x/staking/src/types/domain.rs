use crate::{
    consts::{error::SERDE_ENCODING_DOMAIN_TYPE, keeper::VALIDATORS_BY_POWER_INDEX_KEY},
    Commission, CommissionRates, CommissionRaw, Description,
};
use chrono::Utc;
use gears::{
    core::{errors::CoreError, Protobuf},
    error::AppError,
    tendermint::types::{
        proto::{crypto::PublicKey, header::Header, validator::ValidatorUpdate},
        time::Timestamp,
    },
    types::{
        address::{AccAddress, ConsAddress, ValAddress},
        decimal256::Decimal256,
        uint::Uint256,
    },
};
use prost::{Enumeration, Message};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, fmt::Display, str::FromStr};

/// HistoricalInfo contains header and validator information for a given block.
/// It is stored as part of staking module's state, which persists the `n` most
/// recent HistoricalInfo
/// (`n` is set by the staking module's `historical_entries` parameter).
#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct HistoricalInfo {
    header: Header,
    validators: Vec<Validator>,
}

impl HistoricalInfo {
    /// Method will create a historical information struct from header and valset
    /// it will first sort valset before inclusion into historical info
    pub fn new(
        header: Header,
        mut validators: Vec<Validator>,
        power_reduction: i64,
    ) -> HistoricalInfo {
        fn less(v1: &Validator, v2: &Validator, power_reduction: i64) -> Ordering {
            let cons_power1 = v1.consensus_power(power_reduction);
            let cons_power2 = v2.consensus_power(power_reduction);
            if cons_power1 == cons_power2 {
                let addr1 = Vec::from(v1.cons_addr());
                let addr2 = Vec::from(v2.cons_addr());
                addr1.cmp(&addr2)
            } else {
                cons_power1.cmp(&cons_power2)
            }
        }
        validators.sort_by(|v1, v2| less(v1, v2, power_reduction));
        HistoricalInfo { header, validators }
    }
}

/// Last validator power, needed for validator set update logic
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct LastValidatorPower {
    pub address: ValAddress,
    pub power: i64,
}

/// Delegation represents the bond with tokens held by an account. It is
/// owned by one delegator, and is associated with the voting power of one
/// validator.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Delegation {
    pub delegator_address: AccAddress,
    pub validator_address: ValAddress,
    pub shares: Decimal256,
}

/// Delegation represents the bond with tokens held by an account. It is
/// owned by one delegator, and is associated with the voting power of one
/// validator.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct UnbondingDelegation {
    pub delegator_address: AccAddress,
    pub validator_address: ValAddress,
    pub entries: Vec<UnbondingDelegationEntry>,
}

/// UnbondingDelegationEntry - entry to an UnbondingDelegation
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct UnbondingDelegationEntry {
    pub creation_height: i64,
    pub completion_time: Timestamp,
    pub initial_balance: Uint256,
    pub balance: Uint256,
}

impl UnbondingDelegationEntry {
    pub fn is_mature(&self, time: chrono::DateTime<Utc>) -> bool {
        // TODO: consider to move the DataTime type and work with timestamps into Gears
        // The timestamp is provided by context and conversion won't fail.
        let completion_time = chrono::DateTime::from_timestamp(
            self.completion_time.seconds,
            self.completion_time.nanos as u32,
        )
        .unwrap();
        completion_time <= time
    }
}

/// Redelegation contains the list of a particular delegator's
/// redelegating bonds from a particular source validator to a
/// particular destination validator
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Redelegation {
    pub delegator_address: AccAddress,
    pub validator_src_address: ValAddress,
    pub validator_dst_address: ValAddress,
    pub entries: Vec<RedelegationEntry>,
}

impl Redelegation {
    pub fn add_entry(&mut self, redelegation_entry: RedelegationEntry) {
        self.entries.push(redelegation_entry);
    }
}

/// RedelegationEntry - entry to a Redelegation
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct RedelegationEntry {
    pub creation_height: u64,
    pub completion_time: Timestamp,
    pub initial_balance: Uint256,
    pub share_dst: Decimal256,
}

impl RedelegationEntry {
    pub fn is_mature(&self, time: chrono::DateTime<Utc>) -> bool {
        // TODO: consider to move the DataTime type and work with timestamps into Gears
        // The timestamp is provided by context and conversion won't fail.
        let completion_time = chrono::DateTime::from_timestamp(
            self.completion_time.seconds,
            self.completion_time.nanos as u32,
        )
        .unwrap();
        completion_time <= time
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DvvTriplet {
    pub del_addr: AccAddress,
    pub val_src_addr: ValAddress,
    pub val_dst_addr: ValAddress,
}
impl DvvTriplet {
    pub fn new(del_addr: AccAddress, val_src_addr: ValAddress, val_dst_addr: ValAddress) -> Self {
        Self {
            del_addr,
            val_src_addr,
            val_dst_addr,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DvPair {
    pub val_addr: ValAddress,
    pub del_addr: AccAddress,
}
impl DvPair {
    pub fn new(val_addr: ValAddress, del_addr: AccAddress) -> Self {
        Self { val_addr, del_addr }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Enumeration)]
pub enum BondStatus {
    Unbonded = 0,
    Unbonding = 1,
    Bonded = 2,
}

impl Display for BondStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BondStatus::Unbonded => write!(f, "Unbonded"),
            BondStatus::Unbonding => write!(f, "Unbonding"),
            BondStatus::Bonded => write!(f, "Bonded"),
        }
    }
}

/// Validator defines a validator, together with the total amount of the
/// Validator's bond shares and their exchange rate to coins. Slashing results in
/// a decrease in the exchange rate, allowing correct calculation of future
/// undelegations without iterating over delegators. When coins are delegated to
/// this validator, the validator is credited with a delegation whose number of
/// bond shares is based on the amount of coins delegated divided by the current
/// exchange rate. Voting power can be calculated as total bonded shares
/// multiplied by exchange rate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Validator {
    /// operator_address defines the address of the validator's operator; bech encoded in JSON.
    pub operator_address: ValAddress,
    /// delegator_shares defines total shares issued to a validator's delegators.
    pub delegator_shares: Decimal256,
    /// description defines the description terms for the validator.
    pub description: Description,
    /// consensus_pubkey is the consensus public key of the validator, as a Protobuf Any.
    pub consensus_pubkey: PublicKey,
    /// jailed defined whether the validator has been jailed from bonded status or not.
    pub jailed: bool,
    /// tokens define the delegated tokens (incl. self-delegation).
    pub tokens: Uint256,
    /// unbonding_height defines, if unbonding, the height at which this validator has begun unbonding.
    pub unbonding_height: u64,
    /// unbonding_time defines, if unbonding, the min time for the validator to complete unbonding.
    pub unbonding_time: Timestamp,
    /// commission defines the commission parameters.
    pub commission: Commission,
    pub min_self_delegation: Uint256,
    pub status: BondStatus,
}

impl Validator {
    pub fn new_with_defaults(
        operator_address: ValAddress,
        consensus_pubkey: PublicKey,
        description: Description,
    ) -> Validator {
        Validator {
            operator_address,
            delegator_shares: Decimal256::zero(),
            description,
            consensus_pubkey,
            jailed: false,
            tokens: Uint256::zero(),
            unbonding_height: 0,
            unbonding_time: Timestamp {
                seconds: 0,
                nanos: 0,
            },
            commission: Commission::new(
                CommissionRates {
                    rate: Decimal256::zero(),
                    max_rate: Decimal256::zero(),
                    max_change_rate: Decimal256::zero(),
                },
                Timestamp {
                    seconds: 0,
                    nanos: 0,
                },
            )
            .expect("creation of commission with zeros shouldn't fail"),
            min_self_delegation: Uint256::one(),
            status: BondStatus::Unbonded,
        }
    }

    pub fn abci_validator_update(&self, power: i64) -> ValidatorUpdate {
        ValidatorUpdate {
            pub_key: self.consensus_pubkey.clone(),
            power: self.consensus_power(power),
        }
    }
    pub fn abci_validator_update_zero(&self) -> ValidatorUpdate {
        self.abci_validator_update(0)
    }

    pub fn set_initial_commission(&mut self, commission: Commission) {
        self.commission = commission;
    }

    /// add_tokens_from_del adds tokens to a validator
    pub fn add_tokens_from_del(&mut self, amount: Uint256) -> Decimal256 {
        // calculate the shares to issue
        let issues_shares = if self.delegator_shares.is_zero() {
            // the first delegation to a validator sets the exchange rate to one
            // TODO: infallible in sdk
            Decimal256::from_atomics(amount, 0).unwrap()
        } else {
            // TODO: check the code, maybe remove unwrap
            self.shares_from_tokens(amount).unwrap()
        };

        self.tokens += amount;
        self.delegator_shares += issues_shares;
        issues_shares
    }

    pub fn shares_from_tokens(&self, amount: Uint256) -> anyhow::Result<Decimal256> {
        if self.tokens.is_zero() {
            return Err(AppError::Custom("insufficient shares".into()).into());
        }
        Ok(self
            .delegator_shares
            .checked_mul(Decimal256::from_atomics(amount, 0)?)?
            .checked_div(Decimal256::from_atomics(self.tokens, 0)?)?)
    }

    pub fn shares_from_tokens_truncated(&self, amount: Uint256) -> anyhow::Result<Decimal256> {
        if self.tokens.is_zero() {
            return Err(AppError::Custom("insufficient shares".into()).into());
        }

        let mul = self
            .delegator_shares
            .checked_mul(Decimal256::from_atomics(amount, 0)?)?;
        // TODO: check constant 18 in decimals
        let precision_reuse = Decimal256::from_atomics(10u64, 0)?.checked_pow(18)?;
        let mul2 = mul.checked_mul(precision_reuse)?;
        let div = mul2.checked_div(Decimal256::from_atomics(self.tokens, 0)?)?;
        Ok(div.checked_div(precision_reuse)?)
    }

    /// RemoveDelShares removes delegator shares from a validator.
    /// NOTE: because token fractions are left in the valiadator,
    ///       the exchange rate of future shares of this validator can increase.
    pub fn remove_del_shares(&mut self, del_shares: Decimal256) -> Uint256 {
        let remaining_shares = self.delegator_shares - del_shares;

        let issued_tokens = if remaining_shares.is_zero() {
            // last delegation share gets any trimmings
            let tokens = self.tokens;
            self.tokens = Uint256::zero();
            tokens
        } else {
            // leave excess tokens in the validator
            // however fully use all the delegator shares
            // TODO: infallible + floor
            let tokens = self.tokens_from_shares(del_shares).unwrap().to_uint_floor();
            // TODO: check of negative result
            self.tokens -= tokens;
            //         panic("attempting to remove more tokens than available in validator")
            tokens
        };

        self.delegator_shares = remaining_shares;
        issued_tokens
    }

    /// calculate the token worth of provided shares
    // TODO: infallible in sdk
    pub fn tokens_from_shares(&self, shares: Decimal256) -> anyhow::Result<Decimal256> {
        Ok(shares
            .checked_mul(Decimal256::from_atomics(self.tokens, 0)?)?
            .checked_div(self.delegator_shares)?)
    }

    pub fn invalid_ex_rate(&self) -> bool {
        self.tokens.is_zero() && (self.delegator_shares > Decimal256::zero())
    }

    pub fn cons_addr(&self) -> ConsAddress {
        self.consensus_pubkey.clone().into()
    }

    pub fn update_status(&mut self, status: BondStatus) {
        self.status = status;
    }

    pub fn tendermint_power(&self) -> i64 {
        if self.status == BondStatus::Bonded {
            return self.potential_tendermint_power();
        }
        0
    }

    pub fn potential_tendermint_power(&self) -> i64 {
        let amount = self.tokens / Uint256::from(10u64).pow(6);
        amount
            .to_string()
            .parse::<i64>()
            .expect("Unexpected conversion error")
    }

    pub fn consensus_power(&self, power: i64) -> i64 {
        match self.status {
            BondStatus::Bonded => self.potential_consensus_power(power),
            _ => 0,
        }
    }

    pub fn potential_consensus_power(&self, power: i64) -> i64 {
        self.tokens_to_consensus_power(power)
    }

    pub fn tokens_to_consensus_power(&self, power: i64) -> i64 {
        let amount = self.tokens / Uint256::from(power as u64);
        amount
            .to_string()
            .parse::<i64>()
            .expect("Unexpected conversion error")
    }

    /// GetValidatorsByPowerIndexKey creates the validator by power index.
    /// Power index is the key used in the power-store, and represents the relative
    /// power ranking of the validator.
    /// VALUE: validator operator address ([]byte)
    pub fn key_by_power_index_key(&self, power_reduction: i64) -> Vec<u8> {
        // NOTE the address doesn't need to be stored because counter bytes must always be different
        // NOTE the larger values are of higher value
        let consensus_power = self.tokens_to_consensus_power(power_reduction);
        let consensus_power_bytes = consensus_power.to_le_bytes();

        let oper_addr_invr = self
            .operator_address
            .to_string()
            .as_bytes()
            .iter()
            .map(|b| 255 ^ b)
            .collect::<Vec<_>>();

        // key is of format prefix || powerbytes || addrLen (1byte) || addrBytes
        let mut key = VALIDATORS_BY_POWER_INDEX_KEY.to_vec();
        key.extend_from_slice(&consensus_power_bytes);
        key.push(oper_addr_invr.len() as u8);
        key.extend_from_slice(&oper_addr_invr);
        key
    }
}

impl TryFrom<ValidatorRaw> for Validator {
    type Error = CoreError;
    fn try_from(value: ValidatorRaw) -> Result<Self, Self::Error> {
        let status = value.status();
        Ok(Self {
            operator_address: ValAddress::from_bech32(&value.operator_address)
                .map_err(|e| CoreError::DecodeAddress(e.to_string()))?,
            delegator_shares: Decimal256::from_str(&value.delegator_shares)
                .map_err(|e| CoreError::DecodeGeneral(e.to_string()))?,
            description: value.description.ok_or(CoreError::MissingField(
                "Missing field 'description'.".into(),
            ))?,
            consensus_pubkey: serde_json::from_slice(&value.consensus_pubkey)
                .map_err(|e| CoreError::DecodeGeneral(e.to_string()))?,
            jailed: value.jailed,
            tokens: Uint256::from_str(&value.tokens)
                .map_err(|e| CoreError::DecodeGeneral(e.to_string()))?,
            unbonding_height: value.unbonding_height,
            unbonding_time: value.unbonding_time.ok_or(CoreError::MissingField(
                "Missing field 'unbonding_time'.".into(),
            ))?,
            commission: value
                .commission
                .ok_or(CoreError::MissingField(
                    "Missing field 'description'.".into(),
                ))?
                .try_into()?,
            min_self_delegation: Uint256::from_str(&value.min_self_delegation)
                .map_err(|e| CoreError::DecodeGeneral(e.to_string()))?,
            status,
        })
    }
}

#[derive(Clone, PartialEq, Message)]
pub struct ValidatorRaw {
    #[prost(string)]
    pub operator_address: String,
    #[prost(string)]
    pub delegator_shares: String,
    #[prost(message, optional)]
    pub description: Option<Description>,
    #[prost(bytes)]
    pub consensus_pubkey: Vec<u8>,
    #[prost(bool)]
    pub jailed: bool,
    #[prost(string)]
    pub tokens: String,
    #[prost(uint64)]
    pub unbonding_height: u64,
    #[prost(message, optional)]
    pub unbonding_time: Option<Timestamp>,
    #[prost(message, optional)]
    pub commission: Option<CommissionRaw>,
    #[prost(string)]
    pub min_self_delegation: String,
    #[prost(enumeration = "BondStatus")]
    pub status: i32,
}

impl From<Validator> for ValidatorRaw {
    fn from(value: Validator) -> Self {
        Self {
            operator_address: value.operator_address.to_string(),
            delegator_shares: value.delegator_shares.to_string(),
            description: Some(value.description),
            consensus_pubkey: serde_json::to_vec(&value.consensus_pubkey)
                .expect(SERDE_ENCODING_DOMAIN_TYPE),
            jailed: value.jailed,
            tokens: value.tokens.to_string(),
            unbonding_height: value.unbonding_height,
            unbonding_time: Some(value.unbonding_time),
            commission: Some(value.commission.into()),
            min_self_delegation: value.min_self_delegation.to_string(),
            status: value.status.into(),
        }
    }
}

impl Protobuf<ValidatorRaw> for Validator {}
