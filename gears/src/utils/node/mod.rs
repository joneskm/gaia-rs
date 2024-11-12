mod ctx;
mod helpers;
mod mock;
mod presets;

use address::AccAddress;
use keyring::key::pair::KeyPair;

pub use ctx::*;
pub use helpers::*;
pub use mock::*;
pub use presets::*;

use crate::{application::ApplicationInfo, crypto::keys::ReadAccAddress};

#[derive(Debug)]
pub struct User {
    pub key_pair: KeyPair,
    pub account_number: u64,
}

impl User {
    pub fn address(&self) -> AccAddress {
        self.key_pair.get_address()
    }

    pub fn from_bech32(mnemonic: impl AsRef<str>, account_number: u64) -> Option<Self> {
        let mnemonic = bip32::Mnemonic::new(mnemonic, bip32::Language::English).ok()?;
        let key_pair = KeyPair::from_mnemonic(&mnemonic);

        Some(Self {
            key_pair,
            account_number,
        })
    }
}

#[derive(Debug, Clone)]
pub struct MockApplication;

impl ApplicationInfo for MockApplication {}
