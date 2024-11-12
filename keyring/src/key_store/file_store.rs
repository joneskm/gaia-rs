use std::{
    fs::{self, remove_file, OpenOptions},
    io::{ErrorKind, Write},
    path::Path,
};

use std::fs::File;

use crate::{error::Error, key::pair::KeyPair};

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use eth_keystore::{decrypt_key_string, encrypt_key_string};

use std::os::unix::fs::PermissionsExt;

const JSON_EXTENSION: &str = "json";
const KEY_HASH_FILE: &str = "key_hash";

fn verify_password(
    password: Option<impl AsRef<str>>,
    password_hash: &str,
    key_hash_path: &Path,
) -> Result<(), Error> {
    match password {
        Some(password) => {
            let parsed_hash = PasswordHash::new(password_hash).map_err(|e| Error::KeyHash {
                source: e,
                path: key_hash_path.display().to_string(),
                msg: e.to_string(),
            })?;

            // NOTE: hash params from `parsed_hash` are used instead of what is configured in the
            // `Argon2` instance.
            Argon2::default()
                .verify_password(password.as_ref().as_bytes(), &parsed_hash)
                .map_err(|_| Error::IncorrectPassword)
        }
        None => {
            if password_hash.is_empty() {
                Ok(())
            } else {
                Err(Error::IncorrectPassword)
            }
        }
    }
}

fn calculate_password_hash(password: Option<impl AsRef<str>>) -> Result<String, Error> {
    if let Some(password) = password {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        Ok(argon2
            .hash_password(password.as_ref().as_bytes(), &salt)
            .map_err(|e| Error::InvalidPassword {
                source: e,
                msg: e.to_string(),
            })?
            .to_string())
    } else {
        Ok(String::new())
    }
}

fn set_readonly(file: File, path: String) -> Result<(), Error> {
    let mut perms = file
        .metadata()
        .map_err(|e| Error::ReadOnly {
            msg: e.to_string(),
            source: e,
            path: path.clone(),
        })?
        .permissions();

    #[cfg(unix)]
    {
        PermissionsExt::set_mode(&mut perms, 0o400);
    }

    #[cfg(not(unix))]
    {
        perms.set_readonly(true);
    }

    file.set_permissions(perms).map_err(|e| Error::ReadOnly {
        msg: e.to_string(),
        source: e,
        path,
    })?;
    Ok(())
}

/// Opens an existing keyring or creates a new one if `create` is `true`.
fn open(path: impl AsRef<Path>, create: bool, backend: Backend) -> Result<Option<String>, Error> {
    let key_hash_path = path.as_ref().join(KEY_HASH_FILE);

    match fs::read_to_string(&key_hash_path) {
        Ok(password_hash) => {
            if backend == Backend::Test {
                if password_hash.is_empty() {
                    Ok(None)
                } else {
                    Err(Error::IncorrectBackend {
                        path: path.as_ref().display().to_string(),
                        expected: backend.into(),
                        found: Backend::Encrypted.into(),
                    })
                }
            } else if password_hash.is_empty() {
                Err(Error::IncorrectBackend {
                    path: path.as_ref().display().to_string(),
                    expected: backend.into(),
                    found: Backend::Test.into(),
                })
            } else {
                let password = Some(
                    //TODO: wrap password in secret
                    rpassword::prompt_password("Enter keyring passphrase: ").map_err(|e| {
                        Error::IO {
                            msg: e.to_string(),
                            source: e,
                        }
                    })?,
                );
                verify_password(password.as_deref(), &password_hash, &key_hash_path)?;
                Ok(password)
            }
        }
        Err(e) => {
            if e.kind() == ErrorKind::NotFound {
                if create {
                    fs::create_dir_all(&path).map_err(|e| Error::FileIO {
                        msg: e.to_string(),
                        source: e,
                        path: path.as_ref().display().to_string(),
                    })?;

                    let password = if backend == Backend::Test {
                        None
                    } else {
                        let password = rpassword::prompt_password("Enter keyring passphrase: ")
                            .map_err(|e| Error::IO {
                                msg: e.to_string(),
                                source: e,
                            })?;
                        Some(password)
                    };

                    let password_hash = calculate_password_hash(password.as_deref())?;

                    let mut file = OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(&key_hash_path)
                        .map_err(|e| Error::FileIO {
                            msg: e.to_string(),
                            source: e,
                            path: path.as_ref().display().to_string(),
                        })?;

                    file.write_all(password_hash.as_bytes())
                        .map_err(|e| Error::FileIO {
                            msg: e.to_string(),
                            source: e,
                            path: path.as_ref().display().to_string(),
                        })?;

                    set_readonly(file, key_hash_path.display().to_string())?;

                    Ok(password)
                } else {
                    Err(Error::KeyringDoesNotExist(
                        path.as_ref().display().to_string(),
                    ))
                }
            } else {
                Err(Error::FileIO {
                    msg: e.to_string(),
                    source: e,
                    path: key_hash_path.display().to_string(),
                })
            }
        }
    }
}

/// Gets the entry with the given name.
/// Returns [`Error`] if no entry with the given name can be found.
pub fn get_key_by_name<S>(
    name: &S,
    path: impl AsRef<Path>,
    backend: Backend,
) -> Result<KeyPair, Error>
where
    S: AsRef<str> + ?Sized,
{
    let password = open(&path, false, backend)?;
    let mut path = path.as_ref().join(name.as_ref());
    path.set_extension(JSON_EXTENSION);

    fs::read(&path)
        .map_err(|e| {
            if e.kind() == ErrorKind::NotFound {
                Error::DoesNotExist {
                    name: name.as_ref().into(),
                    location: path.display().to_string(),
                }
            } else {
                Error::FileIO {
                    msg: e.to_string(),
                    source: e,
                    path: path.display().to_string(),
                }
            }
        })
        .and_then(|v| {
            let raw_key = String::from_utf8(v).map_err(|e| Error::InvalidUTF8 {
                msg: e.to_string(),
                source: e,
                path: path.display().to_string(),
            })?;

            let json_key = if let Some(password) = password {
                let raw_key =
                    decrypt_key_string(raw_key, password).map_err(|e| Error::KEYSTORE {
                        msg: e.to_string(),
                        source: e,
                        path: path.display().to_string(),
                    })?;
                String::from_utf8(raw_key).map_err(|e| Error::InvalidUTF8 {
                    msg: e.to_string(),
                    source: e,
                    path: path.display().to_string(),
                })?
            } else {
                raw_key
            };

            serde_json::from_str(&json_key).map_err(|e| Error::JSON {
                msg: e.to_string(),
                source: e,
                path: path.display().to_string(),
            })
        })
}

/// Returns an [`Error`] if an entry with the same name already exists. If an entry already exists for
/// the given key but with a different name then a new separate entry will be created.
pub fn set_key_pair<S: AsRef<str>>(
    key_name: S,
    key_pair: &KeyPair,
    path: impl AsRef<Path>,
    backend: Backend,
) -> Result<(), Error> {
    let password = open(&path, true, backend)?;

    let mut path = path.as_ref().join(key_name.as_ref());
    path.set_extension(JSON_EXTENSION);

    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .map_err(|e| {
            if e.kind() == ErrorKind::AlreadyExists {
                Error::AlreadyExists {
                    name: key_name.as_ref().into(),
                    location: path.display().to_string(),
                }
            } else {
                Error::FileIO {
                    msg: e.to_string(),
                    source: e,
                    path: path.display().to_string(),
                }
            }
        })?;

    let serialized_key_pair = serde_json::to_string(&key_pair).expect("serialization won't fail");
    let key = match password {
        Some(password) => encrypt_key_string(&mut OsRng, serialized_key_pair, password).0,
        None => serde_json::to_string_pretty(&key_pair).expect("key pair will always serialize"),
    };

    file.write_all(key.as_bytes()).map_err(|e| Error::FileIO {
        msg: e.to_string(),
        source: e,
        path: path.display().to_string(),
    })?;

    set_readonly(file, path.display().to_string())
}

/// Deletes the entry with the given name.
/// Returns [`Error`] if no entry with the given name can be found.
pub fn delete_key_by_name<S>(name: S, path: impl AsRef<Path>, backend: Backend) -> Result<(), Error>
where
    S: AsRef<str>,
{
    open(&path, false, backend)?;

    let mut path = path.as_ref().join(name.as_ref());
    path.set_extension(JSON_EXTENSION);

    remove_file(&path).map_err(|e| {
        if e.kind() == ErrorKind::NotFound {
            Error::DoesNotExist {
                name: name.as_ref().into(),
                location: path.display().to_string(),
            }
        } else {
            Error::FileIO {
                msg: e.to_string(),
                source: e,
                path: path.display().to_string(),
            }
        }
    })
}

#[derive(Debug, PartialEq)]
pub enum Backend {
    Test,
    Encrypted,
}

impl From<Backend> for String {
    fn from(backend: Backend) -> Self {
        match backend {
            Backend::Test => "test".into(),
            Backend::Encrypted => "encrypted".into(),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::path;

    #[test]
    fn calc_none_password_hash() {
        let password: Option<&str> = None;
        let password_hash = calculate_password_hash(password).expect("password is valid");
        assert_eq!(&password_hash, "");
    }

    #[test]
    fn calc_password_hash() {
        let password = "password";

        let password_hash = calculate_password_hash(Some(password)).expect("password is valid");
        assert_eq!(&password_hash[0..31], "$argon2id$v=19$m=19456,t=2,p=1$");
    }

    #[test]
    fn verify_none_password_success() {
        let path = path::PathBuf::from("test");
        let password: Option<&str> = None;
        verify_password(password, "", &path).expect("password is valid");
    }

    #[test]
    fn verify_password_success() {
        let password = "pastword";
        let password_hash = "$argon2id$v=19$m=19456,t=2,p=1$0Wzy7RqEZA/HDKXNHAy6lQ$QAhM6YYWd5ZLZcgcMXYIRztBL0IfyHjacsF6X4kbYR0";
        let path = path::PathBuf::from("test");

        verify_password(Some(password), password_hash, &path).expect("password is valid");
    }

    #[test]
    fn verify_password_wrong_password() {
        let password = "wrong_password";
        let password_hash = "$argon2id$v=19$m=19456,t=2,p=1$0Wzy7RqEZA/HDKXNHAy6lQ$QAhM6YYWd5ZLZcgcMXYIRztBL0IfyHjacsF6X4kbYR0";
        let path = path::PathBuf::from("test");

        let err = verify_password(Some(password), password_hash, &path).unwrap_err();

        assert!(matches!(err, Error::IncorrectPassword));
    }

    #[test]
    fn verify_password_invalid_hash() {
        let password = "pastword";
        let password_hash = "1vcy/0FkRHslP417Yk0VtQ$aIBL2rzqEOiYUPQbHXuPr6Nz9a0zPImBtmNPu1FSVfw";
        let path = path::PathBuf::from("test");

        let err = verify_password(Some(password), password_hash, &path).unwrap_err();
        assert!(matches!(err, Error::KeyHash { .. }));
    }

    #[test]
    fn e2e_password_hash() {
        let password = "password123";
        let path = path::PathBuf::from("test");

        let password_hash = calculate_password_hash(Some(password)).expect("password is valid");
        verify_password(Some(password), &password_hash, &path).expect("password matches hash");
    }
}
