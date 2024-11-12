//! Different helpers

use sha2::{Digest, Sha256};

const LEAF_PREFIX: [u8; 1] = [0];
const INNER_PREFIX: [u8; 1] = [1];
/// Hash when tree is empty
pub const EMPTY_HASH: [u8; 32] = [
    227, 176, 196, 66, 152, 252, 28, 20, 154, 251, 244, 200, 153, 111, 185, 36, 39, 174, 65, 228,
    100, 155, 147, 76, 164, 149, 153, 27, 120, 82, 184, 85,
]; // = Sha256::digest([]).into()

/// Length of hash
pub const HASH_LENGTH: usize = 32;

/// Alias to hash array
pub type Sha256Hash = [u8; HASH_LENGTH];

/// TODO: Move to kv_store
pub fn root_hash(items: &[Vec<u8>]) -> [u8; 32] {
    match items.len() {
        0 => EMPTY_HASH,
        1 => leaf_hash(&items[0]),
        n => {
            let k = get_split_point(n);
            let left = root_hash(&items[0..k]);
            let right = root_hash(&items[k..]);
            inner_hash(&left, &right)
        }
    }
}

/// Returns sha256(0x00 || leaf)
fn leaf_hash(leaf: &[u8]) -> [u8; 32] {
    Sha256::digest([&LEAF_PREFIX, leaf].concat()).into()
}

/// Returns sha256(0x01 || left || right)
fn inner_hash(left: &[u8], right: &[u8]) -> [u8; 32] {
    Sha256::digest([&INNER_PREFIX, left, right].concat()).into()
}

/// Returns the largest power of two less than length
fn get_split_point(length: usize) -> usize {
    if length == 0 {
        panic!("trying to split a tree with length zero")
    }

    let bit_len = length.ilog2();
    let mut k = 1usize << bit_len;
    if k == length {
        k >>= 1;
    };

    k
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn root_hash_works() {
        let items = [];
        assert_eq!(
            hex::encode(root_hash(&items)),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );

        let items = [vec![1, 2, 3]];
        assert_eq!(
            hex::encode(root_hash(&items)),
            "054edec1d0211f624fed0cbca9d4f9400b0e491c43742af2c5b0abebf0c990d8"
        );

        let items = [vec![]];
        assert_eq!(
            hex::encode(root_hash(&items)),
            "6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d"
        );

        let items = [vec![1, 2, 3], vec![4, 5, 6]];
        assert_eq!(
            hex::encode(root_hash(&items)),
            "82e6cfce00453804379b53962939eaa7906b39904be0813fcadd31b100773c4b"
        );

        let items = [vec![1, 2], vec![3, 4], vec![5, 6], vec![7, 8], vec![9, 10]];
        assert_eq!(
            hex::encode(root_hash(&items)),
            "f326493eceab4f2d9ffbc78c59432a0a005d6ea98392045c74df5d14a113be18"
        );
    }

    #[test]
    fn leaf_hash_works() {
        assert_eq!(
            hex::encode(leaf_hash(&[9])),
            "c87479cd656e7e3ad6bd8db402e8027df454b2b0c42ff29e093458beb98a23d4"
        );
    }

    #[test]
    fn inner_hash_works() {
        assert_eq!(
            hex::encode(inner_hash(&[9], &[12])),
            "7f02ddcdb6d47574560fe3b881eb6ccc1c795751d1d99b3e45b800e07c31cd62"
        );
    }

    #[test]
    fn get_split_point_works() {
        let split = get_split_point(100);
        assert_eq!(split, 64);

        let split = get_split_point(64);
        assert_eq!(split, 32);

        let split = get_split_point(1);
        assert_eq!(split, 0);
    }
}
