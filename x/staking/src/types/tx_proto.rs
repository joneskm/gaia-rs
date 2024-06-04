use crate::consts::proto::*;
use gears::{
    core::{errors::Error, Protobuf},
    error::AppError,
    tendermint::types::{proto::crypto::PublicKey, time::Timestamp},
    types::{
        address::{AccAddress, ValAddress},
        auth::fee::inner::Coin as CoinRaw,
        base::coin::Coin,
        decimal256::{CosmosDecimalProtoString, Decimal256},
        errors::StdError,
        uint::Uint256,
    },
};
use prost::Message;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// CommissionRates defines the initial commission rates to be used for creating
/// a validator.
#[derive(Clone, PartialEq, Serialize, Deserialize, Message)]
pub struct CommissionRatesRaw {
    #[prost(string)]
    pub rate: String,
    #[prost(string)]
    pub max_rate: String,
    #[prost(string)]
    pub max_change_rate: String,
}

impl From<CommissionRates> for CommissionRatesRaw {
    fn from(value: CommissionRates) -> Self {
        Self {
            rate: value.rate.to_cosmos_proto_string(),
            max_rate: value.max_rate.to_cosmos_proto_string(),
            max_change_rate: value.max_change_rate.to_cosmos_proto_string(),
        }
    }
}

/// CommissionRates defines the initial commission rates to be used for creating
/// a validator.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommissionRates {
    /// rate is the commission rate charged to delegators, as a fraction.
    pub rate: Decimal256,
    /// max_rate defines the maximum commission rate which validator can ever charge, as a fraction.
    pub max_rate: Decimal256,
    /// max_change_rate defines the maximum daily increase of the validator commission, as a fraction.
    pub max_change_rate: Decimal256,
}

impl TryFrom<CommissionRatesRaw> for CommissionRates {
    type Error = StdError;
    fn try_from(value: CommissionRatesRaw) -> Result<Self, Self::Error> {
        Ok(Self {
            rate: Decimal256::from_cosmos_proto_string(&value.rate)?,
            max_rate: Decimal256::from_cosmos_proto_string(&value.max_rate)?,
            max_change_rate: Decimal256::from_cosmos_proto_string(&value.max_change_rate)?,
        })
    }
}

impl Protobuf<CommissionRatesRaw> for CommissionRates {}

/// Commission defines commission parameters for a given validator.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Commission {
    /// commission_rates defines the initial commission rates to be used for creating a validator.
    commission_rates: CommissionRates,
    /// update_time is the last time the commission rate was changed.
    update_time: Timestamp,
}

impl TryFrom<CommissionRaw> for Commission {
    type Error = Error;

    fn try_from(value: CommissionRaw) -> Result<Self, Self::Error> {
        Ok(Self {
            commission_rates: value
                .commission_rates
                .ok_or(Error::MissingField(
                    "Value should exists. It's the proto3 rule to have Option<T> instead of T"
                        .into(),
                ))?
                .try_into()
                .map_err(|e| Error::DecodeProtobuf(format!("{e}")))?,
            update_time: value
                .update_time
                .ok_or(Error::MissingField(
                    "Value should exists. It's the proto3 rule to have Option<T> instead of T"
                        .into(),
                ))?
                .try_into()
                .map_err(|e| Error::DecodeProtobuf(format!("{e}")))?,
        })
    }
}

impl Commission {
    pub fn new(
        commission_rates: CommissionRates,
        update_time: Timestamp,
    ) -> Result<Commission, AppError> {
        Self::validate_commission_rates(&commission_rates)?;
        Ok(Commission {
            commission_rates,
            update_time,
        })
    }

    pub fn validate_commission_rates(commission_rates: &CommissionRates) -> Result<(), AppError> {
        let CommissionRates {
            rate,
            max_rate,
            max_change_rate,
        } = commission_rates;

        if *max_rate > ONE_DEC {
            // max rate cannot be greater than 1
            return Err(AppError::Send("max_rate too huge".into()));
        }
        if *rate > *max_rate {
            // rate cannot be greater than the max rate
            return Err(AppError::Send("rate is bigger than max_rate".into()));
        }
        if *max_change_rate > *max_rate {
            // change rate cannot be greater than the max rate
            return Err(AppError::Send(
                "max_change_rate is bigger than max_rate".into(),
            ));
        }
        Ok(())
    }
}

/// Commission defines commission parameters for a given validator.
#[derive(Clone, PartialEq, Message)]
pub struct CommissionRaw {
    #[prost(message, optional)]
    pub commission_rates: Option<CommissionRatesRaw>,
    #[prost(message, optional)]
    pub update_time: Option<Timestamp>,
}

impl From<Commission> for CommissionRaw {
    fn from(value: Commission) -> Self {
        Self {
            commission_rates: Some(value.commission_rates.into()),
            update_time: Some(value.update_time),
        }
    }
}

impl Protobuf<CommissionRaw> for Commission {}

/// Description defines a validator description.
#[derive(Clone, PartialEq, Serialize, Deserialize, Message)]
pub struct Description {
    /// moniker defines a human-readable name for the validator.
    #[prost(string)]
    pub moniker: String,
    /// identity defines an optional identity signature (ex. UPort or Keybase).
    #[prost(string)]
    pub identity: String,
    /// website defines an optional website link.
    #[prost(string)]
    pub website: String,
    /// security_contact defines an optional email for security contact.
    #[prost(string)]
    pub security_contact: String,
    /// details define other optional details.
    #[prost(string)]
    pub details: String,
}

impl Protobuf<Description> for Description {}

impl Description {
    pub fn ensure_length(&self) -> Result<(), AppError> {
        if self.moniker.len() > MAX_MONIKER_LENGTH {
            return Err(self.form_ensure_length_err(
                "moniker",
                self.moniker.len(),
                MAX_MONIKER_LENGTH,
            ));
        }
        if self.identity.len() > MAX_IDENTITY_LENGTH {
            return Err(self.form_ensure_length_err(
                "identity",
                self.identity.len(),
                MAX_IDENTITY_LENGTH,
            ));
        }
        if self.website.len() > MAX_WEBSITE_LENGTH {
            return Err(self.form_ensure_length_err(
                "website",
                self.website.len(),
                MAX_WEBSITE_LENGTH,
            ));
        }
        if self.security_contact.len() > MAX_SECURITY_CONTACT_LENGTH {
            return Err(self.form_ensure_length_err(
                "security_contact",
                self.security_contact.len(),
                MAX_SECURITY_CONTACT_LENGTH,
            ));
        }
        if self.details.len() > MAX_DETAILS_LENGTH {
            return Err(self.form_ensure_length_err(
                "details",
                self.details.len(),
                MAX_DETAILS_LENGTH,
            ));
        }
        Ok(())
    }

    fn form_ensure_length_err(&self, name: &str, got: usize, max: usize) -> AppError {
        AppError::InvalidRequest(format!("invalid {name} length; got: {got}, max: {max}"))
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Message)]
pub struct CreateValidatorRaw {
    #[prost(message, optional)]
    pub description: Option<Description>,
    #[prost(message, optional)]
    pub commission: Option<CommissionRatesRaw>,
    #[prost(string)]
    pub min_self_delegation: String,
    #[prost(string)]
    pub delegator_address: String,
    #[prost(string)]
    pub validator_address: String,
    #[prost(bytes)]
    pub pub_key: Vec<u8>,
    #[prost(message, optional)]
    pub value: Option<CoinRaw>,
}

impl From<CreateValidator> for CreateValidatorRaw {
    fn from(src: CreateValidator) -> Self {
        Self {
            description: Some(src.description),
            commission: Some(src.commission.into()),
            min_self_delegation: src.min_self_delegation.to_string(),
            delegator_address: src.delegator_address.to_string(),
            validator_address: src.validator_address.to_string(),
            // TODO: Consider to implement Protobuf on PublicKey
            pub_key: serde_json::to_vec(&src.pub_key).expect("Expected valid public key that can be converted into vector of bytes using serde_json"),
            value: Some(src.value.into()),
        }
    }
}

/// CreateValidator defines a SDK message for creating a new validator.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateValidator {
    pub description: Description,
    pub commission: CommissionRates,
    pub min_self_delegation: Uint256,
    pub delegator_address: AccAddress,
    pub validator_address: ValAddress,
    pub pub_key: PublicKey,
    pub value: Coin,
}

impl TryFrom<CreateValidatorRaw> for CreateValidator {
    type Error = Error;

    fn try_from(src: CreateValidatorRaw) -> Result<Self, Self::Error> {
        Ok(CreateValidator {
            description: src.description.ok_or(Error::MissingField(
                "Value should exists. It's the proto3 rule to have Option<T> instead of T".into(),
            ))?,
            commission: src
                .commission
                .ok_or(Error::MissingField(
                    "Value should exists. It's the proto3 rule to have Option<T> instead of T"
                        .into(),
                ))?
                .try_into()
                .map_err(|e| Error::DecodeProtobuf(format!("{e}")))?,
            min_self_delegation: Uint256::from_str(&src.min_self_delegation)
                .map_err(|e| Error::DecodeGeneral(e.to_string()))?,
            delegator_address: AccAddress::from_bech32(&src.delegator_address)
                .map_err(|e| Error::DecodeAddress(e.to_string()))?,
            validator_address: ValAddress::from_bech32(&src.validator_address)
                .map_err(|e| Error::DecodeAddress(e.to_string()))?,
            pub_key: serde_json::from_slice(&src.pub_key)
                .map_err(|e| Error::DecodeGeneral(e.to_string()))?,
            value: src
                .value
                .ok_or(Error::MissingField(
                    "Value should exists. It's the proto3 rule to have Option<T> instead of T"
                        .into(),
                ))?
                .try_into()
                .map_err(|e| Error::Coin(format!("{e}")))?,
        })
    }
}

impl Protobuf<CreateValidatorRaw> for CreateValidator {}

#[derive(Clone, PartialEq, Serialize, Deserialize, Message)]
pub struct DelegateMsgRaw {
    #[prost(string)]
    pub delegator_address: String,
    #[prost(string)]
    pub validator_address: String,
    #[prost(message, optional)]
    pub amount: Option<CoinRaw>,
}

impl From<DelegateMsg> for DelegateMsgRaw {
    fn from(src: DelegateMsg) -> Self {
        Self {
            delegator_address: src.delegator_address.to_string(),
            validator_address: src.validator_address.to_string(),
            amount: Some(src.amount.into()),
        }
    }
}

/// Creates a new DelegateMsg transaction message instance.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct DelegateMsg {
    pub delegator_address: AccAddress,
    pub validator_address: ValAddress,
    pub amount: Coin,
}

impl TryFrom<DelegateMsgRaw> for DelegateMsg {
    type Error = Error;

    fn try_from(src: DelegateMsgRaw) -> Result<Self, Self::Error> {
        Ok(DelegateMsg {
            delegator_address: AccAddress::from_bech32(&src.delegator_address)
                .map_err(|e| Error::DecodeAddress(e.to_string()))?,
            validator_address: ValAddress::from_bech32(&src.validator_address)
                .map_err(|e| Error::DecodeAddress(e.to_string()))?,
            amount: src
                .amount
                .ok_or(Error::MissingField(
                    "Value should exists. It's the proto3 rule to have Option<T> instead of T"
                        .into(),
                ))?
                .try_into()
                .map_err(|e| Error::Coin(format!("{e}")))?,
        })
    }
}

impl Protobuf<DelegateMsgRaw> for DelegateMsg {}

#[derive(Clone, PartialEq, Serialize, Deserialize, Message)]
pub struct RedelegateMsgRaw {
    #[prost(string)]
    pub delegator_address: String,
    #[prost(string)]
    pub src_validator_address: String,
    #[prost(string)]
    pub dst_validator_address: String,
    #[prost(message, optional)]
    pub amount: Option<CoinRaw>,
}

impl From<RedelegateMsg> for RedelegateMsgRaw {
    fn from(src: RedelegateMsg) -> Self {
        Self {
            delegator_address: src.delegator_address.to_string(),
            src_validator_address: src.src_validator_address.to_string(),
            dst_validator_address: src.dst_validator_address.to_string(),
            amount: Some(src.amount.into()),
        }
    }
}

/// Creates a new RedelegateMsg transaction message instance.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct RedelegateMsg {
    pub delegator_address: AccAddress,
    pub src_validator_address: ValAddress,
    pub dst_validator_address: ValAddress,
    pub amount: Coin,
}

impl TryFrom<RedelegateMsgRaw> for RedelegateMsg {
    type Error = Error;

    fn try_from(src: RedelegateMsgRaw) -> Result<Self, Self::Error> {
        Ok(RedelegateMsg {
            delegator_address: AccAddress::from_bech32(&src.delegator_address)
                .map_err(|e| Error::DecodeAddress(e.to_string()))?,
            src_validator_address: ValAddress::from_bech32(&src.src_validator_address)
                .map_err(|e| Error::DecodeAddress(e.to_string()))?,
            dst_validator_address: ValAddress::from_bech32(&src.dst_validator_address)
                .map_err(|e| Error::DecodeAddress(e.to_string()))?,
            amount: src
                .amount
                .ok_or(Error::MissingField(
                    "Value should exists. It's the proto3 rule to have Option<T> instead of T"
                        .into(),
                ))?
                .try_into()
                .map_err(|e| Error::Coin(format!("{e}")))?,
        })
    }
}

impl Protobuf<RedelegateMsgRaw> for RedelegateMsg {}