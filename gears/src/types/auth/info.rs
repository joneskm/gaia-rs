use core_types::{errors::CoreError, Protobuf};
use serde::{Deserialize, Serialize};

use crate::types::signing::SignerInfo;

use super::{
    fee::{Fee, FeeError},
    tip::{Tip, TipError},
};

pub mod inner {
    pub use core_types::auth::AuthInfo;
}

/// AuthInfo describes the fee and signer modes that are used to sign a
/// transaction.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AuthInfo {
    /// signer_infos defines the signing modes for the required signers. The number
    /// and order of elements must match the required signers from TxBody's
    /// messages. The first element is the primary signer and the one which pays
    /// the fee.
    pub signer_infos: Vec<SignerInfo>,
    /// Fee is the fee and gas limit for the transaction. The first signer is the
    /// primary signer and the one which pays the fee. The fee can be calculated
    /// based on the cost of evaluating the body and doing signature verification
    /// of the signers. This can be estimated via simulation.
    pub fee: Fee,
    // Tip is the optional tip used for transactions fees paid in another denom.
    //
    // This field is ignored if the chain didn't enable tips, i.e. didn't add the
    // `TipDecorator` in its posthandler.
    //
    // Since: cosmos-sdk 0.46
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tip: Option<Tip>,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum AuthError {
    #[error("signer info decode: {0}")]
    Decode(String),
    #[error("missing field: fee")]
    MissingField,
    #[error("{0}")]
    Tip(#[from] TipError),
    #[error("{0}")]
    FeeError(#[from] FeeError),
}

impl TryFrom<inner::AuthInfo> for AuthInfo {
    type Error = AuthError;

    fn try_from(raw: inner::AuthInfo) -> Result<Self, Self::Error> {
        let signer_infos: Result<Vec<SignerInfo>, CoreError> = raw
            .signer_infos
            .into_iter()
            .map(|info| info.try_into())
            .collect();

        let tip = raw.tip.map(|tip| tip.try_into()).transpose()?;

        Ok(AuthInfo {
            signer_infos: signer_infos.map_err(|e| AuthError::Decode(e.to_string()))?,
            fee: raw.fee.ok_or(AuthError::MissingField)?.try_into()?,
            tip,
        })
    }
}

impl From<AuthInfo> for inner::AuthInfo {
    fn from(auth_info: AuthInfo) -> inner::AuthInfo {
        let sig_infos: Vec<SignerInfo> = auth_info.signer_infos;
        let sig_infos = sig_infos
            .into_iter()
            .map(|sig_info| sig_info.into())
            .collect();

        Self {
            signer_infos: sig_infos,
            fee: Some(auth_info.fee.into()),
            tip: auth_info.tip.map(|tip| tip.into()),
        }
    }
}

impl Protobuf<inner::AuthInfo> for AuthInfo {}
