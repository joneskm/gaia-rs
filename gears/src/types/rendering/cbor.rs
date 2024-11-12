use std::{collections::BTreeMap, io::Write};

use ciborium::value::CanonicalValue;
use serde::Serialize;

// Cbor is a CBOR (RFC8949) data item that can be encoded to a stream.
pub trait Cbor {
    // `encode` deterministically writes the CBOR-encoded data to the stream.
    fn encode(&self, writter: &mut impl Write) -> Result<(), std::io::Error>;
}

impl<V: Serialize> Cbor for BTreeMap<CanonicalValue, V> {
    fn encode(&self, writter: &mut impl Write) -> Result<(), std::io::Error> {
        ciborium::into_writer(self, writter)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use ciborium::{
        value::{CanonicalValue, Integer},
        Value,
    };
    use extensions::testing::UnwrapTesting;

    use super::Cbor;

    #[test]
    fn check_hashmap() {
        // Examples come from RFC8949, Appendix A
        let mut var = BTreeMap::new();

        var.insert(Value::Integer(1.into()).into(), 2_u64);
        var.insert(Value::Integer(3.into()).into(), 4);

        let mut buf = Vec::new();

        var.encode(&mut buf).unwrap_test();

        let hex = data_encoding::HEXLOWER.encode(&buf);

        assert_eq!(&hex, "a201020304")
    }

    // Ensures that the serialized values are in the correct order. A naive
    // implementation would serialize the values in the order of the integer
    // values, which would result in the order -1,10. This is incorrect, as
    // the canonical order is 10,-1. This is because the CBOR serialization of
    // 10 is 0x0a,the serialization of -1 is 0x20 and 0x0a < 0x20.
    #[test]
    fn check_hashmap_serialized_values() {
        let mut final_map = BTreeMap::new();

        let key = Value::Integer(10.into());
        let canonical_key: CanonicalValue = key.into();
        final_map.insert(canonical_key, 2);

        let value = Value::Integer(Integer::from(-1));
        let canonical_value: CanonicalValue = value.into();
        final_map.insert(canonical_value, 3);

        let mut bytes = Vec::new();
        ciborium::into_writer(&final_map, &mut bytes).unwrap_test();
        let hex_bytes = hex::encode(bytes);

        assert_eq!(hex_bytes, "a20a022003")
    }
}
