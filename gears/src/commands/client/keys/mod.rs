use anyhow::Result;
use bip32::Mnemonic;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use strum::Display;
use text_io::read;

use crate::crypto::keys::ReadAccAddress;

const KEYRING_SUB_DIR_FILE: &str = "keyring-file";
const KEYRING_SUB_DIR_TEST: &str = "keyring-test";

#[derive(Clone, Default, Debug, Display, Deserialize, Serialize)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
pub enum KeyringBackend {
    #[default]
    #[strum(to_string = "file")]
    File,
    #[strum(to_string = "test")]
    Test,
}

impl KeyringBackend {
    pub fn get_sub_dir(&self) -> &str {
        match self {
            KeyringBackend::File => KEYRING_SUB_DIR_FILE,
            KeyringBackend::Test => KEYRING_SUB_DIR_TEST,
        }
    }

    pub fn to_keyring_backend<'a>(&self, path: &'a std::path::Path) -> keyring::Backend<'a> {
        match self {
            KeyringBackend::File => keyring::Backend::File(path),
            KeyringBackend::Test => keyring::Backend::Test(path),
        }
    }
}

#[derive(Debug, Clone)]
pub enum KeyCommand {
    Add(AddKeyCommand),
}

#[derive(Debug, Clone, former::Former)]
pub struct AddKeyCommand {
    pub name: String,
    pub recover: bool,
    pub home: PathBuf,
    pub keyring_backend: KeyringBackend,
    pub bip39_mnemonic: Option<String>,
}

// TODO: remove this cli code
pub fn keys(command: KeyCommand) -> Result<()> {
    match command {
        KeyCommand::Add(cmd) => {
            let AddKeyCommand {
                name,
                recover,
                home,
                keyring_backend,
                bip39_mnemonic,
            } = cmd;

            let keyring_home = home.join(keyring_backend.get_sub_dir());

            let backend = keyring_backend.to_keyring_backend(&keyring_home);

            if recover {
                let phrase = if let Some(bip) = bip39_mnemonic {
                    bip
                } else {
                    println!("> Enter your bip39 mnemonic");
                    let phrase: String = read!("{}\n");
                    phrase
                };

                let mnemonic = Mnemonic::new(phrase, bip32::Language::English)?;

                keyring::add_key(&name, &mnemonic, keyring::KeyType::Secp256k1, backend)?;
            } else {
                let (mnemonic, key_pair) =
                    keyring::create_key(&name, keyring::KeyType::Secp256k1, backend)?;

                println!("Created key {}\nAddress: {}", name, key_pair.get_address());

                println!("\n**Important** write this mnemonic phrase in a safe place.\nIt is the only way to recover your account.\n");
                println!("{}", mnemonic.phrase());
            }
        }
    }

    Ok(())
}
