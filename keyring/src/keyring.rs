use std::path::Path;

use crate::{
    error::Error,
    key::pair::{secp256k1_key_pair::Secp256k1KeyPair, KeyPair},
    key_store::file_store,
};
use bip32::Mnemonic;
use k256::elliptic_curve::rand_core;

use rand_core::OsRng;

/// Used to specify the type of key to generate.
#[derive(Debug)]
pub enum KeyType {
    Secp256k1,
}

#[derive(Debug, Clone, Copy)]
pub enum Backend<'a> {
    File(&'a Path),
    Test(&'a Path),
}

/// Generates a key pair from the mnemonic provided and stores the keypair.
pub fn add_key<S>(
    name: S,
    mnemonic: &Mnemonic,
    key_type: KeyType,
    backend: Backend<'_>,
) -> Result<KeyPair, Error>
where
    S: AsRef<str>,
{
    let key_pair = match key_type {
        KeyType::Secp256k1 => KeyPair::Secp256k1(Secp256k1KeyPair::from_mnemonic(mnemonic)),
    };

    match backend {
        Backend::File(path) => {
            file_store::set_key_pair(name, &key_pair, path, file_store::Backend::Encrypted)?;
        }
        Backend::Test(path) => {
            file_store::set_key_pair(name, &key_pair, path, file_store::Backend::Test)?;
        }
    };

    Ok(key_pair)
}

/// Generates a new random mnemonic and key pair, stores the new key pair and
/// returns the generated mnemonic.
pub fn create_key<S>(
    name: S,
    key_type: KeyType,
    backend: Backend<'_>,
) -> Result<(Mnemonic, KeyPair), Error>
where
    S: AsRef<str>,
{
    let mnemonic = Mnemonic::random(OsRng, bip32::Language::English);
    let key_pair = add_key(name, &mnemonic, key_type, backend)?;
    Ok((mnemonic, key_pair))
}

/// Get a key by name.
pub fn key_by_name<S>(name: &S, backend: Backend<'_>) -> Result<KeyPair, Error>
where
    S: AsRef<str> + ?Sized,
{
    match backend {
        Backend::File(path) => {
            file_store::get_key_by_name(name, path, file_store::Backend::Encrypted)
        }
        Backend::Test(path) => file_store::get_key_by_name(name, path, file_store::Backend::Test),
    }
    //TODO: return key wrapped in Secret
}

/// Delete a key by name.
pub fn delete_key_by_name<S>(name: S, backend: Backend<'_>) -> Result<(), Error>
where
    S: AsRef<str>,
{
    match backend {
        Backend::File(path) => {
            file_store::delete_key_by_name(name, path, file_store::Backend::Encrypted)
        }
        Backend::Test(path) => {
            file_store::delete_key_by_name(name, path, file_store::Backend::Test)
        }
    }
}

#[cfg(test)]
mod tests {

    use std::path::PathBuf;

    use extensions::testing::UnwrapTesting;

    use super::*;

    #[test]
    fn keyring_test_scenario_works() {
        let path = PathBuf::from("./tmp/keyring/src/keyring/keyring_test_scenario_works");
        let _ = std::fs::remove_dir_all(&path);

        // add key should succeed
        let mnemonic = "race draft rival universe maid cheese steel logic crowd fork comic easy truth drift tomorrow eye buddy head time cash swing swift midnight borrow";
        let mnemonic = Mnemonic::new(mnemonic, bip32::Language::English).unwrap_test();
        add_key("bob", &mnemonic, KeyType::Secp256k1, Backend::Test(&path))
            .expect("key should be added");

        // add key with same name should fail
        let error = add_key("bob", &mnemonic, KeyType::Secp256k1, Backend::Test(&path))
            .expect_err("key should not be added");
        assert!(matches!(error, Error::AlreadyExists { .. }));

        // get key should succeed
        key_by_name("bob", Backend::Test(&path)).expect("key should be retrieved");

        // delete key should succeed
        delete_key_by_name("bob", Backend::Test(&path)).expect("key should be deleted");

        // get key should fail
        let error =
            key_by_name("bob", Backend::Test(&path)).expect_err("key should not be retrieved");
        assert!(matches!(error, Error::DoesNotExist { .. }));

        // delete key should fail
        let error =
            delete_key_by_name("bob", Backend::Test(&path)).expect_err("key should not be deleted");
        assert!(matches!(error, Error::DoesNotExist { .. }));

        // create key should succeed
        create_key("bob", KeyType::Secp256k1, Backend::Test(&path)).expect("key should be created");

        // get key should succeed
        key_by_name("bob", Backend::Test(&path)).expect("key should be retrieved");

        std::fs::remove_dir_all(path.clone()).expect("tmp directory should be deleted");

        // get should fail
        let error =
            key_by_name("bob", Backend::Test(&path)).expect_err("keyring should fail to open");
        assert!(matches!(error, Error::KeyringDoesNotExist(_)));
    }
}
