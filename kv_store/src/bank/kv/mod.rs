//! Key value stores

pub mod application;
pub mod transaction;

#[cfg(test)]
mod test_utils {
    use std::sync::{Arc, RwLock};

    use database::MemDB;
    use trees::iavl::Tree;

    use crate::{cache::KVCache, TREE_CACHE_SIZE};

    use super::{application::ApplicationKVBank, transaction::TransactionKVBank};

    pub fn app_store_build(
        tree_val: impl IntoIterator<Item = (u8, u8)>,
        cache_set: impl IntoIterator<Item = (u8, u8)>,
        cache_del: impl IntoIterator<Item = u8>,
    ) -> ApplicationKVBank<MemDB> {
        let mut tree = Tree::new(
            MemDB::new(),
            None,
            TREE_CACHE_SIZE
                .try_into()
                .expect("Unreachable. Tree cache size is > 0"),
            None,
        )
        .expect("Failed to create Tree");

        for (key, value) in tree_val {
            tree.set(vec![key], vec![value]);
        }

        ApplicationKVBank {
            persistent: Arc::new(RwLock::new(tree)),
            cache: {
                let mut cache = KVCache::default();
                for (key, value) in cache_set {
                    cache.set(vec![key], vec![value]);
                }

                for del in cache_del {
                    cache.delete(&[del]);
                }

                cache
            },
        }
    }

    pub fn tx_store_build(
        tree_val: impl IntoIterator<Item = (u8, u8)>,
        tx_set: impl IntoIterator<Item = (u8, u8)>,
        block_set: impl IntoIterator<Item = (u8, u8)>,
        tx_del: impl IntoIterator<Item = u8>,
        block_del: impl IntoIterator<Item = u8>,
    ) -> TransactionKVBank<MemDB> {
        let mut tree = Tree::new(
            MemDB::new(),
            None,
            TREE_CACHE_SIZE
                .try_into()
                .expect("Unreachable. Tree cache size is > 0"),
            None,
        )
        .expect("Failed to create Tree");

        for (key, value) in tree_val {
            tree.set(vec![key], vec![value]);
        }

        TransactionKVBank {
            persistent: Arc::new(RwLock::new(tree)),
            tx: {
                let mut cache = KVCache::default();
                for (key, value) in tx_set {
                    cache.set(vec![key], vec![value]);
                }

                for del in tx_del {
                    cache.delete(&[del]);
                }

                cache
            },
            block: {
                let mut cache = KVCache::default();
                for (key, value) in block_set {
                    cache.set(vec![key], vec![value]);
                }

                for del in block_del {
                    cache.delete(&[del]);
                }

                cache
            },
        }
    }
}
