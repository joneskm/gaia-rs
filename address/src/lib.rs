//! Implementation of addresses used in application

use bech32::{FromBase32, ToBase32, Variant};
use serde::{Deserialize, Deserializer, Serialize};
use std::{
    fmt::{self, Display},
    marker::PhantomData,
    str::FromStr,
};

mod prefix;

pub use crate::prefix::*;

const MAX_ADDR_LEN: u8 = 255;

/// identifies users
pub type AccAddress = BaseAddress<Account>;
/// identifies validator operators
pub type ValAddress = BaseAddress<Validator>;
/// identifies validator nodes that are participating in consensus
pub type ConsAddress = BaseAddress<Consensus>;

/// Base address. Use [AccAddress], [ValAddress] or [ConsAddress] in your code
#[derive(Debug, PartialEq, Eq, Clone, Hash, PartialOrd, Ord)]
pub struct BaseAddress<T: AddressKind> {
    bytes: Vec<u8>,
    _marker: PhantomData<T>,
}

impl<T: AddressKind> BaseAddress<T> {
    fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            _marker: PhantomData,
        }
    }

    /// Parse address from [bech32](https://en.bitcoin.it/wiki/Bech32) string. Prefix would be added automatically
    ///
    /// # Example
    /// ```rust
    /// use address::AccAddress;
    ///
    /// let addr = AccAddress::from_bech32("cosmos1syavy2npfyt9tcncdtsdzf7kny9lh777pahuux");
    ///
    /// assert!(addr.is_ok())
    /// ```
    pub fn from_bech32(address: &str) -> Result<Self, AddressError> {
        let (hrp, data, variant) = bech32::decode(address)?;

        let prefix = T::prefix();

        if hrp != prefix {
            return Err(AddressError::InvalidPrefix {
                expected: prefix.into(),
                found: hrp,
            });
        };

        if let Variant::Bech32m = variant {
            return Err(AddressError::InvalidVariant {
                expected: "Bech32".into(),
                found: "Bech32m".into(),
            });
        }

        // It's unclear whether the conversion from base32 can ever fail. Since this method
        // already returns a Result there's no harm in returning an error here.
        let address = Vec::<u8>::from_base32(&data)?;

        Self::verify_length(&address)?;
        Ok(Self::new(address))
    }

    /// Return length of bytes in address
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> u8 {
        self.bytes
            .len()
            .try_into()
            .expect("MAX_ADDR_LEN is a u8 so this can't fail")
    }

    /// Returns the address bytes prefixed with the length of the address.
    pub fn prefix_len_bytes(&self) -> Vec<u8> {
        let len = self.len();
        [&[len], self.bytes.as_slice()].concat()
    }

    /// Returns the address bytes with the length prefix removed.
    pub fn try_from_prefix_length_bytes(v: &[u8]) -> Result<Self, AddressError> {
        if v.is_empty() {
            return Err(AddressError::EmptyAddress);
        }

        let len = v[0] as usize;
        if v.len() != len + 1 {
            return Err(AddressError::InvalidLengthPrefix {
                prefix: v[0],
                found: v.len() - 1,
            });
        }

        v[1..].try_into()
    }

    /// Return hex string representation
    pub fn as_hex(&self) -> String {
        data_encoding::HEXLOWER.encode(&self.bytes)
    }

    fn verify_length(v: &[u8]) -> Result<(), AddressError> {
        if v.len() > MAX_ADDR_LEN.into() {
            Err(AddressError::InvalidLength {
                max: MAX_ADDR_LEN,
                found: v.len(),
            })
        } else if v.is_empty() {
            Err(AddressError::EmptyAddress)
        } else {
            Ok(())
        }
    }
}

impl<T: AddressKind> AsRef<[u8]> for BaseAddress<T> {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

impl<T: AddressKind> Serialize for BaseAddress<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de, T: AddressKind> Deserialize<'de> for BaseAddress<T> {
    fn deserialize<D>(deserializer: D) -> Result<BaseAddress<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(BaseAddressVisitor(PhantomData))
    }
}

struct BaseAddressVisitor<T>(PhantomData<T>);

impl<'de, T: AddressKind> serde::de::Visitor<'de> for BaseAddressVisitor<T> {
    type Value = BaseAddress<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("bech32 encoded address")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        BaseAddress::from_str(v).map_err(|e| E::custom(format!("invalid address '{}' - {}", v, e)))
    }
}

impl<T: AddressKind> Display for BaseAddress<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hrp = T::prefix();

        let addr = bech32::encode(hrp, self.bytes.to_base32(), Variant::Bech32)
            .expect("method can only error if HRP is not valid, hard coded HRP is valid");
        write!(f, "{}", addr)
    }
}

impl<T: AddressKind> TryFrom<String> for BaseAddress<T> {
    type Error = AddressError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_bech32(&value)
    }
}

impl<T: AddressKind> From<BaseAddress<T>> for String {
    fn from(v: BaseAddress<T>) -> String {
        format!("{}", v)
    }
}

impl<T: AddressKind> FromStr for BaseAddress<T> {
    type Err = AddressError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_bech32(s)
    }
}

impl<T: AddressKind> TryFrom<Vec<u8>> for BaseAddress<T> {
    type Error = AddressError;

    fn try_from(v: Vec<u8>) -> Result<BaseAddress<T>, AddressError> {
        Self::verify_length(&v)?;
        Ok(BaseAddress::new(v))
    }
}

impl<T: AddressKind> TryFrom<&[u8]> for BaseAddress<T> {
    type Error = AddressError;

    fn try_from(v: &[u8]) -> Result<BaseAddress<T>, AddressError> {
        v.to_vec().try_into()
    }
}

impl<T: AddressKind> From<BaseAddress<T>> for Vec<u8> {
    fn from(v: BaseAddress<T>) -> Vec<u8> {
        v.bytes
    }
}

// TODO: CHECK IS IT SAFE TO CONVERT ONE KEY TO OTHER
impl From<AccAddress> for ValAddress {
    fn from(value: AccAddress) -> Self {
        Self::new(value.bytes)
    }
}
impl From<ValAddress> for ConsAddress {
    fn from(value: ValAddress) -> Self {
        Self::new(value.bytes)
    }
}
impl From<ValAddress> for AccAddress {
    fn from(value: ValAddress) -> Self {
        Self::new(value.bytes)
    }
}

/// Address parsing errors
#[allow(missing_docs)]
#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
pub enum AddressError {
    #[error(transparent)]
    Decode(#[from] bech32::Error),

    #[error("address has wrong prefix (expected {expected:?}, found {found:?})")]
    InvalidPrefix { expected: String, found: String },

    #[error("invalid variant (expected {expected:?}, found {found:?})")]
    InvalidVariant { expected: String, found: String },

    #[error("invalid length, max length is: {max:?}, found {found:?})")]
    InvalidLength { max: u8, found: usize },

    #[error("address is empty")]
    EmptyAddress,

    #[error("length prefix does not match length (prefix length {prefix:?}, found {found:?})")]
    InvalidLengthPrefix { prefix: u8, found: usize },
}

#[cfg(test)]
mod tests {

    use bech32::ToBase32;
    use extensions::testing::UnwrapTesting;

    use super::*;

    #[test]
    fn from_bech32_success() {
        let input_address = vec![0x00, 0x01, 0x02];
        let encoded = bech32::encode(
            BECH_32_PREFIX_ACC_ADDR,
            input_address.to_base32(),
            Variant::Bech32,
        )
        .unwrap_test();

        let expected_address = BaseAddress::<Account>::new(input_address);

        let address = AccAddress::from_bech32(&encoded);

        assert_eq!(Ok(expected_address), address);
    }

    #[test]
    fn from_bech32_failure_checksum() {
        let input_address = vec![0x00, 0x01, 0x02];
        let mut encoded = bech32::encode(
            BECH_32_PREFIX_ACC_ADDR,
            input_address.to_base32(),
            Variant::Bech32,
        )
        .unwrap_test();

        encoded.pop();

        let err = AccAddress::from_bech32(&encoded);

        assert_eq!(
            err,
            Err(AddressError::Decode(bech32::Error::InvalidChecksum))
        );
    }

    #[test]
    fn from_bech32_failure_wrong_prefix() {
        let mut hrp = BECH_32_PREFIX_ACC_ADDR.to_string();
        hrp.push_str("atom"); // adding to the BECH_32_PREFIX_ACC_ADDR ensures that hrp is different
        let encoded =
            bech32::encode(&hrp, vec![0x00, 0x01, 0x02].to_base32(), Variant::Bech32).unwrap_test();

        let err = AccAddress::from_bech32(&encoded).unwrap_err();

        assert_eq!(
            err,
            AddressError::InvalidPrefix {
                expected: BECH_32_PREFIX_ACC_ADDR.into(),
                found: hrp,
            }
        );
    }

    #[test]
    fn from_bech32_failure_wrong_variant() {
        let encoded = bech32::encode(
            BECH_32_PREFIX_ACC_ADDR,
            vec![0x00, 0x01, 0x02].to_base32(),
            Variant::Bech32m,
        )
        .unwrap_test();

        let err = AccAddress::from_bech32(&encoded).unwrap_err();

        assert_eq!(
            err,
            AddressError::InvalidVariant {
                expected: "Bech32".into(),
                found: "Bech32m".into()
            }
        );
    }

    #[test]
    fn from_bech32_failure_too_long() {
        let encoded = bech32::encode(
            BECH_32_PREFIX_ACC_ADDR,
            vec![0x00; 300].to_base32(),
            Variant::Bech32,
        )
        .unwrap_test();

        let err = AccAddress::from_bech32(&encoded).unwrap_err();

        assert_eq!(
            err,
            AddressError::InvalidLength {
                max: 255,
                found: 300
            }
        );
    }

    #[test]
    fn from_bech32_failure_empty_address() {
        let encoded = bech32::encode(BECH_32_PREFIX_ACC_ADDR, vec![].to_base32(), Variant::Bech32)
            .unwrap_test();

        let err = AccAddress::from_bech32(&encoded).unwrap_err();

        assert_eq!(err, AddressError::EmptyAddress);
    }

    #[test]
    fn from_slice_failure_empty_address() {
        let address: Vec<u8> = vec![];
        let err = AccAddress::try_from(address.as_slice()).unwrap_err();
        assert_eq!(err, AddressError::EmptyAddress);
    }

    #[test]
    fn to_string_success() {
        let addr = "cosmos1syavy2npfyt9tcncdtsdzf7kny9lh777pahuux".to_string();

        let acc_addr = AccAddress::from_bech32(&addr).unwrap_test();

        assert_eq!(addr, acc_addr.to_string());
    }

    #[test]
    fn string_from_self_success() {
        let addr = "cosmos1syavy2npfyt9tcncdtsdzf7kny9lh777pahuux".to_string();

        let acc_addr = AccAddress::from_bech32(&addr).unwrap_test();

        assert_eq!(addr, String::from(acc_addr));
    }

    #[test]
    fn serialize_works() {
        let addr = "cosmos1syavy2npfyt9tcncdtsdzf7kny9lh777pahuux".to_string();
        let acc_addr = AccAddress::from_bech32(&addr).unwrap_test();

        let json = serde_json::to_string(&acc_addr).unwrap_test();

        assert_eq!(json, r#""cosmos1syavy2npfyt9tcncdtsdzf7kny9lh777pahuux""#);
    }

    #[test]
    fn deserialize_works() {
        let json = r#""cosmos1syavy2npfyt9tcncdtsdzf7kny9lh777pahuux""#;
        let addr = serde_json::from_str::<AccAddress>(json).unwrap_test();
        assert_eq!(
            addr,
            AccAddress::from_bech32("cosmos1syavy2npfyt9tcncdtsdzf7kny9lh777pahuux").unwrap_test()
        )
    }

    #[test]
    fn prefix_len_bytes_works() {
        let addr = vec![0x00, 0x01, 0x02];
        let acc_addr = AccAddress::try_from(addr.as_slice()).unwrap_test();

        let prefixed = acc_addr.prefix_len_bytes();

        assert_eq!(vec![3, 0x00, 0x01, 0x02], prefixed);
    }
}
