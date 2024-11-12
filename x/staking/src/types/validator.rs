use crate::{
    consts::keeper::VALIDATORS_BY_POWER_INDEX_KEY, Commission, CommissionRates, Description,
};
use gears::derive::Protobuf;
use gears::{
    core::serializers::serialize_number_to_string,
    tendermint::types::time::timestamp::NewTimestampError,
    types::decimal256::CosmosDecimalProtoString,
};
use gears::{
    core::{errors::CoreError, Protobuf},
    error::{MathOperation, NumericError},
    tendermint::types::{
        proto::{
            crypto::PublicKey,
            validator::{ValidatorUpdate, VotingPower},
        },
        time::timestamp::Timestamp,
    },
    types::{
        address::{ConsAddress, ValAddress},
        decimal256::{Decimal256, PRECISION_REUSE},
        uint::Uint256,
    },
    x::types::validator::{BondStatus, StakingValidator},
};
use serde::{Deserialize, Serialize};
use serde_aux::prelude::deserialize_number_from_string;
use std::{collections::HashSet, str::FromStr};
use thiserror::Error;

/// Last validator power, needed for validator set update logic
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct LastValidatorPower {
    pub address: ValAddress,
    #[serde(serialize_with = "serialize_number_to_string")]
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub power: i64,
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
    #[serde(serialize_with = "serialize_number_to_string")]
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub unbonding_height: u32,
    /// unbonding_time defines, if unbonding, the min time for the validator to complete unbonding.
    pub unbonding_time: Timestamp,
    /// commission defines the commission parameters.
    pub commission: Commission,
    pub min_self_delegation: Uint256,
    pub status: BondStatus,
}

impl TryFrom<Vec<u8>> for Validator {
    type Error = CoreError;

    fn try_from(raw: Vec<u8>) -> Result<Self, Self::Error> {
        Validator::decode_vec(&raw).map_err(|e| CoreError::DecodeGeneral(e.to_string()))
    }
}

impl From<Validator> for Vec<u8> {
    fn from(value: Validator) -> Self {
        value.encode_vec()
    }
}

impl StakingValidator for Validator {
    fn operator(&self) -> &ValAddress {
        &self.operator_address
    }

    fn tokens(&self) -> Uint256 {
        self.tokens
    }

    fn bonded_tokens(&self) -> Uint256 {
        self.bonded_tokens()
    }

    fn delegator_shares(&self) -> Decimal256 {
        self.delegator_shares
    }

    fn cons_pub_key(&self) -> &PublicKey {
        &self.consensus_pubkey
    }

    fn is_jailed(&self) -> bool {
        self.jailed
    }

    fn min_self_delegation(&self) -> Uint256 {
        self.min_self_delegation
    }

    fn commission(&self) -> Decimal256 {
        self.commission.commission_rates().rate()
    }

    fn status(&self) -> BondStatus {
        self.status
    }

    fn tokens_from_shares(&self, shares: Decimal256) -> Result<Decimal256, NumericError> {
        self.tokens_from_shares(shares)
    }
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
            unbonding_time: Timestamp::UNIX_EPOCH,
            commission: Commission::new(
                CommissionRates::new(Decimal256::zero(), Decimal256::zero(), Decimal256::zero())
                    .expect("creation of hardcoded commission rates won't fail"),
                Timestamp::UNIX_EPOCH,
            ),
            min_self_delegation: Uint256::one(),
            status: BondStatus::Unbonded,
        }
    }

    pub fn abci_validator_update(&self, power: u64) -> anyhow::Result<ValidatorUpdate> {
        Ok(ValidatorUpdate {
            pub_key: self.consensus_pubkey.clone(),
            power: VotingPower::new(self.consensus_power(power))?,
        })
    }
    pub fn abci_validator_update_zero(&self) -> ValidatorUpdate {
        self.abci_validator_update(0)
            .expect("hardcoded value is less than max voting power")
    }

    pub fn bonded_tokens(&self) -> Uint256 {
        if self.status == BondStatus::Bonded {
            self.tokens
        } else {
            Uint256::zero()
        }
    }

    pub fn set_initial_commission(&mut self, commission: Commission) {
        self.commission = commission;
    }

    pub fn shares_from_tokens(&self, amount: Uint256) -> anyhow::Result<Decimal256> {
        if self.tokens.is_zero() {
            return Err(anyhow::anyhow!("insufficient shares"));
        }
        Ok(self
            .delegator_shares
            .checked_mul(Decimal256::from_atomics(amount, 0)?)?
            .checked_div(Decimal256::from_atomics(self.tokens, 0)?)?)
    }

    pub fn shares_from_tokens_truncated(&self, amount: Uint256) -> anyhow::Result<Decimal256> {
        if self.tokens.is_zero() {
            return Err(anyhow::anyhow!("insufficient shares"));
        }
        let mul = self
            .delegator_shares
            .checked_mul(Decimal256::from_atomics(amount, 0)?)?;
        let mul2 = mul.checked_mul(PRECISION_REUSE)?;
        let div = mul2.checked_div(Decimal256::from_atomics(self.tokens, 0)?)?;
        Ok(div.checked_div(PRECISION_REUSE)?)
    }

    /// calculate the token worth of provided shares
    pub fn tokens_from_shares(&self, shares: Decimal256) -> Result<Decimal256, NumericError> {
        shares
            .checked_mul(Decimal256::from_atomics(self.tokens, 0)?)
            .map_err(|_| NumericError::Overflow(MathOperation::Mul))?
            .checked_div(self.delegator_shares)
            .map_err(|_| NumericError::Overflow(MathOperation::Div))
    }

    /// add_tokens_from_del adds tokens to a validator
    pub fn add_tokens_from_del(&mut self, amount: Uint256) -> anyhow::Result<Decimal256> {
        // calculate the shares to issue
        let issues_shares = if self.delegator_shares.is_zero() {
            // the first delegation to a validator sets the exchange rate to one
            Decimal256::from_atomics(amount, 0)?
        } else {
            self.shares_from_tokens(amount)?
        };

        self.tokens = self.tokens.checked_add(amount)?;
        self.delegator_shares = self.delegator_shares.checked_add(issues_shares)?;
        Ok(issues_shares)
    }

    /// remove_del_shares removes delegator shares from a validator.
    /// NOTE: because token fractions are left in the valiadator,
    ///       the exchange rate of future shares of this validator can increase.
    pub fn remove_del_shares(&mut self, del_shares: Decimal256) -> anyhow::Result<Uint256> {
        let remaining_shares = self.delegator_shares.checked_sub(del_shares)?;

        let issued_tokens = if remaining_shares.is_zero() {
            // last delegation share gets any trimmings
            let tokens = self.tokens;
            self.tokens = Uint256::zero();
            tokens
        } else {
            // leave excess tokens in the validator
            // however fully use all the delegator shares
            let tokens = self.tokens_from_shares(del_shares)?.to_uint_floor();
            // the library panics on substruct with overflow and this behavior is identical to sdk
            self.tokens = self.tokens.checked_sub(tokens)?;
            tokens
        };

        self.delegator_shares = remaining_shares;
        Ok(issued_tokens)
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

    pub fn consensus_power(&self, power: u64) -> u64 {
        match self.status {
            BondStatus::Bonded => self.potential_consensus_power(power),
            _ => 0,
        }
    }

    pub fn potential_consensus_power(&self, power: u64) -> u64 {
        self.tokens_to_consensus_power(power)
    }

    pub fn tokens_to_consensus_power(&self, power: u64) -> u64 {
        let amount = self.tokens / Uint256::from(power);
        amount.to_string().parse::<u64>().unwrap() // TODO: unwrap
    }

    /// GetValidatorsByPowerIndexKey creates the validator by power index.
    /// Power index is the key used in the power-store, and represents the relative
    /// power ranking of the validator.
    /// VALUE: validator operator address ([]byte)
    pub fn key_by_power_index_key(&self, power_reduction: u64) -> Vec<u8> {
        // NOTE the address doesn't need to be stored because counter bytes must always be different
        // NOTE the larger values are of higher value
        let consensus_power = self.tokens_to_consensus_power(power_reduction);
        let consensus_power_bytes = consensus_power.to_be_bytes();

        let oper_addr_invr: Vec<u8> = self.operator_address.clone().into();
        let oper_addr_invr = oper_addr_invr.iter().map(|b| 255 ^ b).collect::<Vec<_>>();

        // key is of format prefix || powerbytes || addrLen (1byte) || addrBytes
        let mut key = VALIDATORS_BY_POWER_INDEX_KEY.to_vec();
        key.extend_from_slice(&consensus_power_bytes);
        key.push(self.operator_address.len());
        key.extend_from_slice(&oper_addr_invr);
        key
    }
}

mod inner {
    pub use ibc_proto::cosmos::staking::v1beta1::Description;
    pub use ibc_proto::cosmos::staking::v1beta1::ValAddresses;
    pub use ibc_proto::cosmos::staking::v1beta1::Validator;
}
#[derive(Debug, Clone, Deserialize, Serialize, Protobuf)]
#[proto(raw = "inner::ValAddresses")]
pub struct ValAddresses {
    #[proto(repeated)]
    pub addresses: Vec<ValAddress>,
}

impl From<Validator> for inner::Validator {
    fn from(value: Validator) -> Self {
        Self {
            operator_address: value.operator_address.to_string(),
            delegator_shares: value.delegator_shares.to_cosmos_proto_string(),
            description: Some(inner::Description {
                moniker: value.description.moniker,
                identity: value.description.identity,
                website: value.description.website,
                security_contact: value.description.security_contact,
                details: value.description.details,
            }),
            consensus_pubkey: Some(
                gears::crypto::public::PublicKey::from(value.consensus_pubkey).into(),
            ),
            jailed: value.jailed,
            tokens: value.tokens.to_string(),
            unbonding_height: value.unbonding_height as i64,
            unbonding_time: Some(value.unbonding_time.into()),
            commission: Some(ibc_proto::cosmos::staking::v1beta1::Commission {
                commission_rates: Some(ibc_proto::cosmos::staking::v1beta1::CommissionRates {
                    rate: value
                        .commission
                        .commission_rates()
                        .rate()
                        .to_cosmos_proto_string(),
                    max_rate: value
                        .commission
                        .commission_rates()
                        .max_rate()
                        .to_cosmos_proto_string(),
                    max_change_rate: value
                        .commission
                        .commission_rates()
                        .max_change_rate()
                        .to_cosmos_proto_string(),
                }),
                update_time: Some(value.commission.update_time().to_owned().into()),
            }),
            min_self_delegation: value.min_self_delegation.to_string(),
            status: value.status.into(),
        }
    }
}

impl TryFrom<inner::Validator> for Validator {
    type Error = CoreError;
    fn try_from(value: inner::Validator) -> Result<Self, Self::Error> {
        let status = value.status();
        let description = value.description.ok_or(CoreError::MissingField(
            "Missing field 'description'.".into(),
        ))?;
        let consensus_pubkey = value.consensus_pubkey.ok_or(CoreError::MissingField(
            "Missing field 'consensus_pubkey'.".into(),
        ))?;
        let consensus_pubkey = gears::crypto::public::PublicKey::try_from(consensus_pubkey)
            .map_err(|e| CoreError::DecodeGeneral(e.to_string()))?;

        let commission = value.commission.ok_or(CoreError::MissingField(
            "Missing field 'commission'.".into(),
        ))?;

        let commission_rates = commission.commission_rates.ok_or(CoreError::MissingField(
            "Missing field 'commission_rates'.".into(),
        ))?;

        let commission_rates = CommissionRates::new(
            Decimal256::from_cosmos_proto_string(&commission_rates.rate)
                .map_err(|e| CoreError::DecodeGeneral(e.to_string()))?,
            Decimal256::from_cosmos_proto_string(&commission_rates.max_rate)
                .map_err(|e| CoreError::DecodeGeneral(e.to_string()))?,
            Decimal256::from_cosmos_proto_string(&commission_rates.max_change_rate)
                .map_err(|e| CoreError::DecodeGeneral(e.to_string()))?,
        )
        .map_err(|e| CoreError::DecodeGeneral(e.to_string()))?;

        let commission = Commission::new(
            commission_rates,
            commission
                .update_time
                .ok_or(CoreError::MissingField(
                    "Missing field 'update_time'.".into(),
                ))?
                .try_into()
                .map_err(|e: NewTimestampError| CoreError::DecodeGeneral(e.to_string()))?,
        );
        Ok(Self {
            operator_address: ValAddress::from_bech32(&value.operator_address)
                .map_err(|e| CoreError::DecodeAddress(e.to_string()))?,
            delegator_shares: Decimal256::from_cosmos_proto_string(&value.delegator_shares)
                .map_err(|e| CoreError::DecodeGeneral(e.to_string()))?,
            description: Description {
                moniker: description.moniker,
                identity: description.identity,
                website: description.website,
                security_contact: description.security_contact,
                details: description.details,
            },
            consensus_pubkey: consensus_pubkey.into(),
            jailed: value.jailed,
            tokens: Uint256::from_str(&value.tokens)
                .map_err(|e| CoreError::DecodeGeneral(e.to_string()))?,
            unbonding_height: u32::try_from(value.unbonding_height).map_err(|e| {
                CoreError::DecodeGeneral(format!("Failed to convert unbonding_height: {}", e))
            })?,
            unbonding_time: value
                .unbonding_time
                .ok_or(CoreError::MissingField(
                    "Missing field 'unbonding_time'.".into(),
                ))?
                .try_into()
                .map_err(|e| {
                    CoreError::DecodeGeneral(format!("Failed to convert unbonding_time: {}", e))
                })?,
            commission,
            min_self_delegation: Uint256::from_str(&value.min_self_delegation)
                .map_err(|e| CoreError::DecodeGeneral(e.to_string()))?,
            status: match status {
                ibc_proto::cosmos::staking::v1beta1::BondStatus::Unspecified => {
                    return Err(CoreError::DecodeGeneral(
                        "Invalid bond status: Unspecified".into(),
                    ))
                }
                ibc_proto::cosmos::staking::v1beta1::BondStatus::Unbonded => BondStatus::Unbonded,
                ibc_proto::cosmos::staking::v1beta1::BondStatus::Unbonding => BondStatus::Unbonding,
                ibc_proto::cosmos::staking::v1beta1::BondStatus::Bonded => BondStatus::Bonded,
            },
        })
    }
}

impl Protobuf<inner::Validator> for Validator {}

// TODO: remove and update logic after update of ibc-proto dependency
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IbcV046Validator {
    /// operator_address defines the address of the validator's operator; bech encoded in JSON.
    pub operator_address: ValAddress,
    /// delegator_shares defines total shares issued to a validator's delegators.
    pub delegator_shares: Decimal256,
    /// description defines the description terms for the validator.
    pub description: Description,
    /// consensus_pubkey is the consensus public key of the validator, as a Protobuf Any.
    pub consensus_pubkey: gears::crypto::public::PublicKey,
    /// jailed defined whether the validator has been jailed from bonded status or not.
    pub jailed: bool,
    /// tokens define the delegated tokens (incl. self-delegation).
    pub tokens: Uint256,
    /// unbonding_height defines, if unbonding, the height at which this validator has begun unbonding.
    #[serde(serialize_with = "serialize_number_to_string")]
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub unbonding_height: u32,
    /// unbonding_time defines, if unbonding, the min time for the validator to complete unbonding.
    pub unbonding_time: Timestamp,
    /// commission defines the commission parameters.
    pub commission: Commission,
    pub min_self_delegation: Uint256,
    pub status: BondStatus,
    /// list of unbonding ids, each uniquely identifing an unbonding of this validator
    pub unbonding_ids: Vec<u64>,
    pub unbonding_on_hold_ref_count: Uint256,
    pub validator_bond_shares: Decimal256,
    pub liquid_shares: Decimal256,
}

impl From<Validator> for IbcV046Validator {
    fn from(
        Validator {
            operator_address,
            delegator_shares,
            description,
            consensus_pubkey,
            jailed,
            tokens,
            unbonding_height,
            unbonding_time,
            commission,
            min_self_delegation,
            status,
        }: Validator,
    ) -> Self {
        Self {
            operator_address,
            delegator_shares,
            description,
            consensus_pubkey: consensus_pubkey.into(),
            jailed,
            tokens,
            unbonding_height,
            unbonding_time,
            commission,
            min_self_delegation,
            status,
            unbonding_ids: vec![],
            unbonding_on_hold_ref_count: Uint256::default(),
            validator_bond_shares: Decimal256::default(),
            liquid_shares: Decimal256::default(),
        }
    }
}

impl From<IbcV046Validator> for inner::Validator {
    fn from(value: IbcV046Validator) -> Self {
        Self {
            operator_address: value.operator_address.to_string(),
            delegator_shares: value.delegator_shares.to_cosmos_proto_string(),
            description: Some(inner::Description {
                moniker: value.description.moniker,
                identity: value.description.identity,
                website: value.description.website,
                security_contact: value.description.security_contact,
                details: value.description.details,
            }),
            consensus_pubkey: Some(value.consensus_pubkey.into()),
            jailed: value.jailed,
            tokens: value.tokens.to_string(),
            unbonding_height: value.unbonding_height as i64,
            unbonding_time: Some(value.unbonding_time.into()),
            commission: Some(ibc_proto::cosmos::staking::v1beta1::Commission {
                commission_rates: Some(ibc_proto::cosmos::staking::v1beta1::CommissionRates {
                    rate: value
                        .commission
                        .commission_rates()
                        .rate()
                        .to_cosmos_proto_string(),
                    max_rate: value
                        .commission
                        .commission_rates()
                        .max_rate()
                        .to_cosmos_proto_string(),
                    max_change_rate: value
                        .commission
                        .commission_rates()
                        .max_change_rate()
                        .to_cosmos_proto_string(),
                }),
                update_time: Some(value.commission.update_time().to_owned().into()),
            }),
            min_self_delegation: value.min_self_delegation.to_string(),
            status: value.status.into(),
        }
    }
}

impl TryFrom<inner::Validator> for IbcV046Validator {
    type Error = CoreError;
    fn try_from(value: inner::Validator) -> Result<Self, Self::Error> {
        let status = value.status();
        let description = value.description.ok_or(CoreError::MissingField(
            "Missing field 'description'.".into(),
        ))?;
        let consensus_pubkey = value.consensus_pubkey.ok_or(CoreError::MissingField(
            "Missing field 'consensus_pubkey'.".into(),
        ))?;
        let consensus_pubkey = gears::crypto::public::PublicKey::try_from(consensus_pubkey)
            .map_err(|e| CoreError::DecodeGeneral(e.to_string()))?;

        let commission = value.commission.ok_or(CoreError::MissingField(
            "Missing field 'commission'.".into(),
        ))?;

        let commission_rates = commission.commission_rates.ok_or(CoreError::MissingField(
            "Missing field 'commission_rates'.".into(),
        ))?;

        let commission_rates = CommissionRates::new(
            Decimal256::from_cosmos_proto_string(&commission_rates.rate)
                .map_err(|e| CoreError::DecodeGeneral(e.to_string()))?,
            Decimal256::from_cosmos_proto_string(&commission_rates.max_rate)
                .map_err(|e| CoreError::DecodeGeneral(e.to_string()))?,
            Decimal256::from_cosmos_proto_string(&commission_rates.max_change_rate)
                .map_err(|e| CoreError::DecodeGeneral(e.to_string()))?,
        )
        .map_err(|e| CoreError::DecodeGeneral(e.to_string()))?;

        let commission = Commission::new(
            commission_rates,
            commission
                .update_time
                .ok_or(CoreError::MissingField(
                    "Missing field 'update_time'.".into(),
                ))?
                .try_into()
                .map_err(|e: NewTimestampError| CoreError::DecodeGeneral(e.to_string()))?,
        );
        Ok(Self {
            operator_address: ValAddress::from_bech32(&value.operator_address)
                .map_err(|e| CoreError::DecodeAddress(e.to_string()))?,
            delegator_shares: Decimal256::from_cosmos_proto_string(&value.delegator_shares)
                .map_err(|e| CoreError::DecodeGeneral(e.to_string()))?,
            description: Description {
                moniker: description.moniker,
                identity: description.identity,
                website: description.website,
                security_contact: description.security_contact,
                details: description.details,
            },
            consensus_pubkey,
            jailed: value.jailed,
            tokens: Uint256::from_str(&value.tokens)
                .map_err(|e| CoreError::DecodeGeneral(e.to_string()))?,
            unbonding_height: u32::try_from(value.unbonding_height).map_err(|e| {
                CoreError::DecodeGeneral(format!("Failed to convert unbonding_height: {}", e))
            })?,
            unbonding_time: value
                .unbonding_time
                .ok_or(CoreError::MissingField(
                    "Missing field 'unbonding_time'.".into(),
                ))?
                .try_into()
                .map_err(|e| {
                    CoreError::DecodeGeneral(format!("Failed to convert unbonding_time: {}", e))
                })?,
            commission,
            min_self_delegation: Uint256::from_str(&value.min_self_delegation)
                .map_err(|e| CoreError::DecodeGeneral(e.to_string()))?,
            status: match status {
                ibc_proto::cosmos::staking::v1beta1::BondStatus::Unspecified => {
                    return Err(CoreError::DecodeGeneral(
                        "Invalid bond status: Unspecified".into(),
                    ))
                }
                ibc_proto::cosmos::staking::v1beta1::BondStatus::Unbonded => BondStatus::Unbonded,
                ibc_proto::cosmos::staking::v1beta1::BondStatus::Unbonding => BondStatus::Unbonding,
                ibc_proto::cosmos::staking::v1beta1::BondStatus::Bonded => BondStatus::Bonded,
            },
            unbonding_ids: vec![],
            unbonding_on_hold_ref_count: Uint256::default(),
            validator_bond_shares: Decimal256::default(),
            liquid_shares: Decimal256::default(),
        })
    }
}

impl Protobuf<inner::Validator> for IbcV046Validator {}

/// [`Validators`] is a collection of [`Validator`] with some guarantees:
/// - the collection cannot have duplicated validators by public key
/// - no validator can be bonded and jailed at the same time // TODO: should this be a property of the validator itself?
/// - no bonded/unbonded validator can have zero delegator shares // TODO: should this be a property of the validator itself?
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(try_from = "Vec<Validator>")]
pub struct Validators(Vec<Validator>);

impl IntoIterator for Validators {
    type Item = Validator;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl TryFrom<Vec<Validator>> for Validators {
    type Error = ValidatorsError;

    fn try_from(validators: Vec<Validator>) -> Result<Self, Self::Error> {
        Self::validate_validators(&validators)?;
        Ok(Validators(validators))
    }
}

impl Validators {
    fn validate_validators(validators: &[Validator]) -> Result<(), ValidatorsError> {
        let mut addr_set: HashSet<&[u8]> = HashSet::new();
        for v in validators.iter() {
            let cons_pub_key_raw = v.consensus_pubkey.raw();
            if addr_set.contains(cons_pub_key_raw) {
                let str_pub_addr: ConsAddress = v.consensus_pubkey.clone().into();
                return Err(ValidatorsError::Duplicate {
                    moniker: v.description.moniker.to_string(),
                    cons_addr: str_pub_addr,
                });
            }

            if v.jailed && v.status == BondStatus::Bonded {
                let str_pub_addr: ConsAddress = v.consensus_pubkey.clone().into();
                return Err(ValidatorsError::BondedAndJailed {
                    moniker: v.description.moniker.to_string(),
                    cons_addr: str_pub_addr,
                });
            }

            if v.delegator_shares.is_zero() && v.status != BondStatus::Unbonding {
                return Err(ValidatorsError::BondedUnbondedZeroShares(
                    v.operator_address.clone(),
                ));
            }

            addr_set.insert(cons_pub_key_raw);
        }
        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum ValidatorsError {
    #[error("duplicate validator in genesis state: moniker {moniker}, address {cons_addr}")]
    Duplicate {
        moniker: String,
        cons_addr: ConsAddress,
    },

    #[error(
        "validator is bonded and jailed in genesis state: moniker {moniker}, address {cons_addr}"
    )]
    BondedAndJailed {
        moniker: String,
        cons_addr: ConsAddress,
    },

    #[error("bonded/unbonded genesis validator cannot have zero delegator shares, validator: {0}")]
    BondedUnbondedZeroShares(ValAddress),
}

#[cfg(test)]
mod tests {
    use data_encoding::HEXLOWER;
    use gears::extensions::testing::UnwrapTesting;

    use super::*;

    #[test]
    fn test_validator_json_deserialize_proto_serialize() {
        let val_raw = r#"{
          "commission": {
            "commission_rates": {
              "max_change_rate": "0.100000000000000000",
              "max_rate": "0.200000000000000000",
              "rate": "0.100000000000000000"
            },
            "update_time": "2024-07-30T17:10:11.032635319Z"
          },
          "consensus_pubkey": {
            "type": "tendermint/PubKeyEd25519",
            "value": "6Ob7SEB++IzwqXQQ/pgsD/bkxXNl+LDBhJZwpKuvnMo="
          },
          "delegator_shares": "5.000000000000000000",
          "description": {
            "details": "",
            "identity": "",
            "moniker": "my_val",
            "security_contact": "",
            "website": ""
          },
          "jailed": false,
          "min_self_delegation": "1",
          "operator_address": "cosmosvaloper1syavy2npfyt9tcncdtsdzf7kny9lh777yfrfs4",
          "status": "BOND_STATUS_BONDED",
          "tokens": "5",
          "unbonding_height": "0",
          "unbonding_time": "1970-01-01T00:00:00Z"
        }"#;

        let val: Validator = serde_json::from_str(val_raw).unwrap_test();
        let val_proto = val.encode_vec();

        assert_eq!(
            val_proto,
            vec![
                10, 52, 99, 111, 115, 109, 111, 115, 118, 97, 108, 111, 112, 101, 114, 49, 115,
                121, 97, 118, 121, 50, 110, 112, 102, 121, 116, 57, 116, 99, 110, 99, 100, 116,
                115, 100, 122, 102, 55, 107, 110, 121, 57, 108, 104, 55, 55, 55, 121, 102, 114,
                102, 115, 52, 18, 67, 10, 29, 47, 99, 111, 115, 109, 111, 115, 46, 99, 114, 121,
                112, 116, 111, 46, 101, 100, 50, 53, 53, 49, 57, 46, 80, 117, 98, 75, 101, 121, 18,
                34, 10, 32, 232, 230, 251, 72, 64, 126, 248, 140, 240, 169, 116, 16, 254, 152, 44,
                15, 246, 228, 197, 115, 101, 248, 176, 193, 132, 150, 112, 164, 171, 175, 156, 202,
                32, 3, 42, 1, 53, 50, 19, 53, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
                48, 48, 48, 48, 48, 58, 8, 10, 6, 109, 121, 95, 118, 97, 108, 74, 0, 82, 75, 10,
                60, 10, 18, 49, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
                18, 18, 50, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 26,
                18, 49, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 18, 11,
                8, 243, 188, 164, 181, 6, 16, 183, 243, 199, 15, 90, 1, 49
            ]
        )
    }

    #[test]
    fn test_key_by_power_index_key() {
        let val_raw = r#"
{
    "operator_address": "cosmosvaloper1v0thzgvzp8vt6q7ystmfm7a9wvg0ppsfetur3d",
    "consensus_pubkey": {
        "type": "tendermint/PubKeyEd25519",
        "value": "CiBO6qrfEwEg7eOTlqlaSKRjd+GoFQOxFhp3cRblbJyBdA=="
    },
    "jailed": false,
    "status": "BOND_STATUS_UNBONDED",
    "tokens": "1099511627776000000",
    "delegator_shares": "0.000000000000000000",
    "description": {
        "moniker": "",
        "identity": "",
        "website": "",
        "security_contact": "",
        "details": ""
    },
    "unbonding_height": "0",
    "unbonding_time": "1970-01-01T00:00:00Z",
    "commission": {
        "commission_rates": {
            "rate": "0.000000000000000000",
            "max_rate": "0.000000000000000000",
            "max_change_rate": "0.000000000000000000"
        },
        "update_time": "1970-01-01T00:00:00Z"
    },
    "min_self_delegation": "1"
}
          "#;

        let val: Validator = serde_json::from_str(val_raw).unwrap_test();

        let res = HEXLOWER.encode(&val.key_by_power_index_key(1000000));
        let expected = "230000010000000000149c288ede7df62742fc3b7d0962045a8cef0f79f6";

        assert_eq!(res, expected);
    }

    #[test]
    fn test_tokens_to_consensus_power() {
        let val_raw = r#"
{
    "operator_address": "cosmosvaloper1v0thzgvzp8vt6q7ystmfm7a9wvg0ppsfetur3d",
    "consensus_pubkey": {
        "type": "tendermint/PubKeyEd25519",
        "value": "CiBO6qrfEwEg7eOTlqlaSKRjd+GoFQOxFhp3cRblbJyBdA=="
    },
    "jailed": false,
    "status": "BOND_STATUS_UNBONDED",
    "tokens": "1099511627776000000",
    "delegator_shares": "0.000000000000000000",
    "description": {
        "moniker": "",
        "identity": "",
        "website": "",
        "security_contact": "",
        "details": ""
    },
    "unbonding_height": "0",
    "unbonding_time": "1970-01-01T00:00:00Z",
    "commission": {
        "commission_rates": {
            "rate": "0.000000000000000000",
            "max_rate": "0.000000000000000000",
            "max_change_rate": "0.000000000000000000"
        },
        "update_time": "1970-01-01T00:00:00Z"
    },
    "min_self_delegation": "1"
}
          "#;

        let val: Validator = serde_json::from_str(val_raw).unwrap_test();
        let res = val.tokens_to_consensus_power(1000000);
        assert_eq!(res, 1099511627776);
    }
}
