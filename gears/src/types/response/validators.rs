use crate::crypto::public::PublicKey;
use crate::types::pagination::response::PaginationResponse;
use core_types::any::google::Any;
use extensions::pagination::PaginationKey;
use serde::{Deserialize, Serialize};
use tendermint::informal::validator::Info;

/// GetLatestValidatorSetResponse is the response type for the
/// Query/GetValidatorSetByHeight RPC method.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct GetLatestValidatorSetResponse {
    pub block_height: i64,
    pub validators: Vec<Validator>,
    /// pagination defines an pagination for the response.
    pub pagination: Option<PaginationResponse>,
}

// TODO: consider to place in another place
/// Validator is the type for the validator-set.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Validator {
    pub address: String,
    pub pub_key: Option<Any>,
    pub voting_power: i64,
    pub proposer_priority: i64,
}

impl From<Info> for Validator {
    fn from(
        Info {
            address,
            pub_key,
            power,
            name: _,
            proposer_priority,
        }: Info,
    ) -> Self {
        Self {
            address: address.to_string(),
            pub_key: Some(PublicKey::from(pub_key).into()),
            voting_power: power.into(),
            proposer_priority: proposer_priority.into(),
        }
    }
}

impl PaginationKey for Validator {
    fn iterator_key(&self) -> std::borrow::Cow<'_, [u8]> {
        std::borrow::Cow::Borrowed(self.address.as_bytes())
    }
}
