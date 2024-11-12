pub mod data;
pub mod errors;
pub mod metadata;
pub mod signer;
use core_types::{any::google::Any, errors::CoreError, tx::signature::SignatureData, Protobuf};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use vec1::{vec1, Vec1};

use crate::crypto::public::PublicKey;

use self::{
    body::TxBody,
    errors::{EmptyMessagesError, TxError},
};

use super::{address::AccAddress, auth::info::AuthInfo, base::coins::UnsignedCoins};

pub mod body;
pub mod raw;

pub trait TxMessage:
    serde::Serialize + Clone + Send + Sync + 'static + Into<Any> + TryFrom<Any, Error = CoreError>
{
    fn get_signers(&self) -> Vec<&AccAddress>;

    fn type_url(&self) -> &'static str;

    fn amino_url(&self) -> &'static str {
        // we don't force to add legacy amino type url because the app should be focused
        // on better signing processes
        self.type_url()
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub enum NullTxMsg {}

impl TryFrom<Any> for NullTxMsg {
    type Error = CoreError;

    fn try_from(_: Any) -> Result<Self, Self::Error> {
        Err(CoreError::DecodeGeneral(
            "not allowed cast any to null tx msg".to_string(),
        ))
    }
}

impl From<NullTxMsg> for Any {
    fn from(_: NullTxMsg) -> Self {
        unreachable!()
    }
}

impl TxMessage for NullTxMsg {
    fn get_signers(&self) -> Vec<&AccAddress> {
        unreachable!()
    }

    fn type_url(&self) -> &'static str {
        unreachable!()
    }
}

/// Utility type that guarantees correctness of transaction messages set
#[derive(Debug)]
pub struct Messages<T: TxMessage> {
    messages: Vec1<T>,
    /// A number of messages in the transaction. Zero means unlimited number of messages.
    /// Default is 0
    chunk_size: usize,
}

impl<T: TxMessage> Messages<T> {
    pub fn new(messages: Vec<T>, chunk_size: usize) -> Result<Messages<T>, EmptyMessagesError> {
        Ok(Messages {
            messages: messages.try_into().map_err(|_| EmptyMessagesError)?,
            chunk_size,
        })
    }

    /// Converts instance into vector of messages
    pub fn into_msgs(self) -> Vec1<T> {
        self.messages
    }

    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }
}

impl<T: TxMessage> From<T> for Messages<T> {
    fn from(value: T) -> Self {
        Self {
            messages: vec1![value],
            chunk_size: 0,
        }
    }
}

impl<T: TxMessage> TryFrom<Vec<T>> for Messages<T> {
    type Error = EmptyMessagesError;

    fn try_from(messages: Vec<T>) -> Result<Self, Self::Error> {
        Self::new(messages, 0)
    }
}

mod inner {
    pub use core_types::tx::inner::Tx;
}

/// Tx is the standard type used for broadcasting transactions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tx<M> {
    /// body is the processable content of the transaction
    pub body: TxBody<M>,
    /// auth_info is the authorization related content of the transaction,
    /// specifically signers, signer modes and fee
    pub auth_info: AuthInfo,
    /// signatures is a list of signatures that matches the length and order of
    /// AuthInfo's signer_infos to allow connecting signature meta information like
    /// public key and signing mode by position.
    #[serde(
        serialize_with = "core_types::serializers::serialize_vec_of_vec_to_vec_of_base64",
        deserialize_with = "core_types::serializers::deserialize_vec_of_base64_to_vec_of_vec"
    )]
    pub signatures: Vec<Vec<u8>>,
    #[serde(skip_serializing, skip_deserializing)]
    pub signatures_data: Vec<SignatureData>, // TODO: DO WE REALLY NEED THIS FIELD?
}

// TODO:
// 0. Make TxWithRaw the Tx - move methods to TxWithRaw and rename
// 1. Many more checks are needed on DecodedTx::from_bytes see https://github.com/cosmos/cosmos-sdk/blob/2582f0aab7b2cbf66ade066fe570a4622cf0b098/x/auth/tx/decoder.go#L16
// 2. Implement equality on AccAddress to avoid conversion to string in get_signers()
// 3. Consider removing the "seen" hashset in get_signers()
// 4. Remove `get_` from method names.
impl<M: TxMessage> Tx<M> {
    pub fn get_msgs(&self) -> &Vec1<M> {
        &self.body.messages
    }

    pub fn get_signers(&self) -> Vec<&AccAddress> {
        let mut signers = vec![];
        let mut seen = HashSet::new();

        for msg in &self.body.messages {
            for addr in msg.get_signers() {
                if seen.insert(addr.to_string()) {
                    signers.push(addr);
                }
            }
        }

        // ensure any specified fee payer is included in the required signers (at the end)
        let fee_payer = &self.auth_info.fee.payer;

        if let Some(addr) = fee_payer {
            if seen.insert(addr.to_string()) {
                signers.push(addr);
            }
        }

        signers
    }

    // TODO: Remove this method and read valid structure
    pub fn set_signatures_data(&mut self) {
        let mut signatures_data = Vec::with_capacity(self.signatures.len());
        for (i, signature) in self.signatures.iter().enumerate() {
            signatures_data.push(SignatureData {
                signature: signature.clone(),
                // the check above, tx.signatures.len() != tx.auth_info.signer_infos.len(), ensures that this indexing is safe
                sequence: self.auth_info.signer_infos[i].sequence,
                mode_info: self.auth_info.signer_infos[i].mode_info.clone(),
            })
        }

        self.signatures_data = signatures_data;
    }

    pub fn get_signatures(&self) -> &Vec<Vec<u8>> {
        &self.signatures
    }

    pub fn get_signatures_data(&self) -> &Vec<SignatureData> {
        &self.signatures_data
    }

    pub fn get_timeout_height(&self) -> u32 {
        self.body.timeout_height
    }

    pub fn get_memo(&self) -> &str {
        &self.body.memo
    }

    pub fn get_fee(&self) -> Option<&UnsignedCoins> {
        self.auth_info.fee.amount.as_ref()
    }

    pub fn get_fee_payer(&self) -> &AccAddress {
        if let Some(payer) = &self.auth_info.fee.payer {
            payer
        } else {
            // At least one signer exists due to Ante::validate_basic_ante_handler()
            return self.get_signers()[0];
        }
    }

    pub fn get_public_keys(&self) -> Vec<Option<&PublicKey>> {
        self.auth_info
            .signer_infos
            .iter()
            .map(|si| si.public_key.as_ref())
            .collect()
    }
}

impl<M: TxMessage> TryFrom<inner::Tx> for Tx<M> {
    type Error = TxError;

    fn try_from(raw: inner::Tx) -> Result<Self, Self::Error> {
        let body = raw.body.ok_or(TxError::MissingField("body".to_owned()))?;

        // This covers the SDK RejectExtensionOptions ante handler
        // https://github.com/cosmos/cosmos-sdk/blob/2582f0aab7b2cbf66ade066fe570a4622cf0b098/x/auth/ante/ext.go#L27-L36
        if !body.extension_options.is_empty() {
            return Err(TxError::DecodeGeneral(
                "unknown extension options".to_owned(),
            ));
        }

        let auth_info: AuthInfo = raw
            .auth_info
            .ok_or(TxError::MissingField("auth_info".to_owned()))?
            .try_into()?;

        // extract signatures data when decoding - this isn't done in the SDK
        if raw.signatures.len() != auth_info.signer_infos.len() {
            return Err(TxError::DecodeGeneral(
                "signatures list does not match signer_infos length".to_owned(),
            ));
        }
        let mut signatures_data = Vec::with_capacity(raw.signatures.len());
        for (i, signature) in raw.signatures.iter().enumerate() {
            signatures_data.push(SignatureData {
                signature: signature.clone(),
                // the check above, tx.signatures.len() != tx.auth_info.signer_infos.len(), ensures that this indexing is safe
                sequence: auth_info.signer_infos[i].sequence,
                mode_info: auth_info.signer_infos[i].mode_info.clone(),
            })
        }

        Ok(Tx {
            body: body.try_into()?,
            auth_info,
            signatures: raw.signatures,
            signatures_data,
        })
    }
}

impl<M: TxMessage> Protobuf<inner::Tx> for Tx<M> {}

impl<M: TxMessage> From<Tx<M>> for inner::Tx {
    fn from(tx: Tx<M>) -> inner::Tx {
        inner::Tx {
            body: Some(tx.body.into()),
            auth_info: Some(tx.auth_info.into()),
            signatures: tx.signatures,
        }
    }
}
