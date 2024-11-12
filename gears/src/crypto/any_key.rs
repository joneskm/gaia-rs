use keyring::key::pair::KeyPair;

use super::{
    keys::{GearsPublicKey, ReadAccAddress, SigningKey},
    ledger::{LedgerError, LedgerProxyKey},
};

#[derive(Debug)]
pub enum AnyKey {
    Local(KeyPair),
    Ledger(LedgerProxyKey),
}

impl ReadAccAddress for AnyKey {
    fn get_address(&self) -> address::AccAddress {
        match self {
            AnyKey::Local(k) => k.get_address(),
            AnyKey::Ledger(k) => k.get_address(),
        }
    }
}

impl GearsPublicKey for AnyKey {
    fn get_gears_public_key(&self) -> super::public::PublicKey {
        match self {
            AnyKey::Local(k) => k.get_gears_public_key(),
            AnyKey::Ledger(k) => k.get_gears_public_key(),
        }
    }
}

impl SigningKey for AnyKey {
    type Error = LedgerError;

    fn sign(&self, message: &[u8]) -> Result<Vec<u8>, Self::Error> {
        match self {
            AnyKey::Local(k) => Ok(k.sign(message)),
            AnyKey::Ledger(k) => k.sign(message),
        }
    }
}
