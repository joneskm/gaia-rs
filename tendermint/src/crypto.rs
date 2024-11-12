use ed25519_consensus::SigningKey;
use rand::rngs::OsRng;
use thiserror::Error;

use crate::types::proto::crypto::PublicKey;

pub use tendermint_informal::private_key::Ed25519;
pub use tendermint_informal::PrivateKey;

pub fn new_private_key() -> tendermint_informal::PrivateKey {
    let csprng = OsRng {};
    let signing_key = SigningKey::new(csprng);

    tendermint_informal::PrivateKey::Ed25519(signing_key.as_bytes()[..].try_into().expect("cannot fail since as_bytes returns a &[u8; 32] and try_into method only fails if slice.len() != 32"))
}

impl TryFrom<tendermint_informal::PrivateKey> for PublicKey {
    type Error = ConversionError;

    fn try_from(value: tendermint_informal::PrivateKey) -> Result<Self, Self::Error> {
        match value {
            tendermint_informal::PrivateKey::Ed25519(key) => {
                Ok(PublicKey::Ed25519(key.as_bytes().to_vec())) // cannot fail since as_bytes returns a &[u8; 32] and try_into method only fails if slice.len() != 32
            }
            _ => Err(ConversionError {}),
        }
    }
}

#[derive(Error, Debug)]
#[error("only Ed25519 private keys are supported for conversion to public keys")]
pub struct ConversionError {}
