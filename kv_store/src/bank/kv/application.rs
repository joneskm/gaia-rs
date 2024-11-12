//! Implementation of KV Store for storing data while app running and commit changes

use std::{
    borrow::Cow,
    collections::BTreeMap,
    num::NonZero,
    ops::RangeBounds,
    sync::{Arc, RwLock},
};

use database::Database;
use extensions::corruption::UnwrapCorrupt;
use trees::iavl::Tree;

use crate::{
    cache::KVCache,
    error::{KVStoreError, POISONED_LOCK},
    range::Range,
    store::{
        kv::{immutable::KVStore, mutable::KVStoreMut},
        prefix::{immutable::ImmutablePrefixStore, mutable::MutablePrefixStore},
    },
    utils::MergedRange,
    TREE_CACHE_SIZE,
};

use super::transaction::TransactionKVBank;

/// Store used during processing of transactions.
/// This store contains single layer of cache which may be commit'ed to tree and persisted on drive.
/// Generally this store should be used in query or {begin/end}_block methods of application.
///
/// *Note*: ordering of insertion doesn't impact state hash as it does with plain tree.
/// This is due all insertion and deletion is sorted in lexicographical order and executed during [Self::commit]
#[derive(Debug)]
pub struct ApplicationKVBank<DB> {
    pub(crate) persistent: Arc<RwLock<Tree<DB>>>,
    pub(crate) cache: KVCache,
}

impl<DB: Database> ApplicationKVBank<DB> {
    /// Create new `self`
    pub fn new(
        db: DB,
        target_version: Option<NonZero<u32>>,
        name: Option<String>,
    ) -> Result<Self, KVStoreError> {
        Ok(Self {
            persistent: Arc::new(RwLock::new(Tree::new(
                db,
                target_version,
                TREE_CACHE_SIZE
                    .try_into()
                    .expect("Unreachable. Tree cache size is > 0"),
                name,
            )?)),
            cache: Default::default(),
        })
    }

    /// Read persistent database
    #[inline]
    pub fn persistent(&self) -> std::sync::RwLockReadGuard<'_, Tree<DB>> {
        self.persistent.read().expect(POISONED_LOCK)
    }

    /// Clear uncommitted cache
    #[inline]
    pub fn cache_clear(&mut self) {
        self.cache.storage.clear();
        self.cache.delete.clear();
    }

    /// Return transaction store with same tree and copied cache
    #[inline]
    pub fn to_tx_kind(&self) -> TransactionKVBank<DB> {
        TransactionKVBank {
            persistent: Arc::clone(&self.persistent),
            tx: Default::default(),
            block: self.cache.clone(),
        }
    }

    /// Delete key from storage
    #[inline]
    pub fn delete(&mut self, k: &[u8]) -> Option<Vec<u8>> {
        self.cache.delete(k).or(self.persistent().get(k))
    }

    /// Set or append new key to storage
    #[inline]
    pub fn set<KI: IntoIterator<Item = u8>, VI: IntoIterator<Item = u8>>(
        &mut self,
        key: KI,
        value: VI,
    ) {
        self.cache.set(key, value)
    }

    /// Return value of key in storage.
    ///
    /// _Note_: deleted keys wont be returned even before commit.
    pub fn get<R: AsRef<[u8]> + ?Sized>(&self, k: &R) -> Option<Vec<u8>> {
        self.cache
            .get(k.as_ref())
            .ok()?
            .cloned()
            .or(self.persistent().get(k.as_ref()))
    }

    /// Return store which uses prefix for all store methods
    pub fn prefix_store<I: IntoIterator<Item = u8>>(
        &self,
        prefix: I,
    ) -> ImmutablePrefixStore<'_, DB> {
        ImmutablePrefixStore {
            store: KVStore::from(self),
            prefix: prefix.into_iter().collect(),
        }
    }

    /// Return store which uses prefix for all store methods
    pub fn prefix_store_mut<I: IntoIterator<Item = u8>>(
        &mut self,
        prefix: I,
    ) -> MutablePrefixStore<'_, DB> {
        MutablePrefixStore {
            store: KVStoreMut::from(self),
            prefix: prefix.into_iter().collect(),
        }
    }

    /// Return range which iterates over values in tree and cache without cloning cache values
    pub fn range<R: RangeBounds<Vec<u8>> + Clone>(&self, range: R) -> Range<'_, DB, Vec<u8>, R> {
        let cached_values = self
            .cache
            .storage
            .range(range.clone())
            .map(|(first, second)| (Cow::Borrowed(first), Cow::Borrowed(second)));

        let tree = self.persistent.read().expect(POISONED_LOCK);
        let persisted_values = tree
            .range(range)
            // NOTE: Keys filtered only for persisted 'cause cache structure should remove inserted values on delete, but if this change then it's a place for a bug
            .filter(|(key, _)| !self.cache.delete.contains(&**key))
            .map(|(first, second)| (Cow::Owned(first), Cow::Owned(second)));

        MergedRange::merge(cached_values, persisted_values).into()
    }

    /// Consume(take, leave empty) cache of successful transactions from [TransactionKVBank].
    /// This method don't clear any state from transaction layer of cache
    pub fn consume_block_cache(&mut self, other: &mut TransactionKVBank<DB>) {
        let (set_values, del_values) = other.block.take();

        for (key, value) in set_values {
            self.cache.set(key, value)
        }

        for del in del_values {
            self.cache.delete(&del);
        }
    }

    /// Commit changes from cache to tree and return state hash
    ///
    /// # Panics
    /// Currently this method could panic if fails to persist changes to disk.
    /// This is matter of changes and should be discussed.
    pub fn commit(&mut self) -> [u8; 32] {
        let (insert, delete) = self.cache.take();

        let mut persistent = self.persistent.write().expect(POISONED_LOCK);

        let cache = insert
            .into_iter()
            .map(|(key, value)| (key, Some(value)))
            .chain(delete.into_iter().map(|key| (key, None)))
            .collect::<BTreeMap<_, _>>();

        for (key, value) in cache {
            match value {
                Some(value) => persistent.set(key, value),
                None => {
                    let _ = persistent.remove(&key);
                }
            }
        }

        //TODO: is it safe to assume this won't ever error?
        persistent.save_version().unwrap_or_corrupt().0
    }
}

#[cfg(test)]
mod tests {

    use std::collections::BTreeMap;

    use database::MemDB;

    use crate::{
        bank::kv::test_utils::{app_store_build, tx_store_build},
        TREE_CACHE_SIZE,
    };

    use super::*;

    #[test]
    fn tree_commit() {
        let mut store = app_store_build([(1, 11)], [(2, 22), (3, 33)], [4, 5]);

        store.set([20], [10]);
        store.set([30], [20]);
        let _ = store.delete(&[10]);
        store.set([40], [50]);
        store.set([50], [50]);
        let _ = store.delete(&[20]);

        let resulted_cache = store.commit();
        let expected_hash = [
            27, 142, 171, 11, 85, 248, 28, 55, 237, 188, 171, 213, 171, 72, 204, 33, 55, 29, 113,
            175, 221, 165, 53, 187, 80, 14, 185, 198, 52, 197, 207, 47,
        ];

        assert_eq!(resulted_cache, expected_hash)
    }

    #[test]
    fn to_tx_kind_returns_empty() {
        let store = app_store_build([], [], []);

        let result = store.to_tx_kind();
        let expected = tx_store_build([], [], [], [], []);

        assert_eq!(result.block, expected.block);
        assert_eq!(result.tx, expected.tx);
    }

    #[test]
    fn to_tx_kind_returns_with_cache() {
        let store = app_store_build([(1, 11)], [(2, 22), (3, 33)], [4, 5]);

        let result = store.to_tx_kind();
        let expected = tx_store_build([(1, 11)], [], [(2, 22), (3, 33)], [], [4, 5]);

        assert_eq!(result.block, expected.block);
        assert_eq!(result.tx, expected.tx);

        let result_get = result.get(&[1]);

        assert_eq!(Some(vec![11]), result_get)
    }

    #[test]
    fn delete_empty_cache() {
        let mut tree = build_tree();

        let key = vec![1];

        tree.set(key.clone(), vec![2]);

        let mut store = build_store(tree, None);

        // ---
        let deleted = store.delete(&key);

        // ---
        assert_eq!(Some(vec![2]), deleted);
    }

    #[test]
    fn delete_taken_from_cache() {
        let mut tree = build_tree();

        let key = vec![1];

        tree.set(key.clone(), vec![2]);

        let mut cache = KVCache::default();

        cache.storage.insert(key.clone(), vec![3]);

        let mut store = build_store(tree, Some(cache));

        // ---
        let deleted = store.delete(&key);

        // ---
        assert_eq!(Some(vec![3]), deleted);
    }

    #[test]
    fn get_empty_cache() {
        let mut tree = build_tree();

        let key = vec![1];

        tree.set(key.clone(), vec![2]);

        let store = build_store(tree, None);

        // ---
        let result = store.get(&key);

        // ---
        assert_eq!(Some(vec![2]), result);
    }

    #[test]
    fn get_from_cache() {
        let mut tree = build_tree();

        let key = vec![1];

        tree.set(key.clone(), vec![2]);

        let mut cache = KVCache::default();

        cache.storage.insert(key.clone(), vec![3]);

        let store = build_store(tree, Some(cache));

        // ---
        let result = store.get(&key);

        // ---
        assert_eq!(Some(vec![3]), result);
    }

    #[test]
    fn get_deleted() {
        let mut tree = build_tree();

        let key = vec![1];

        tree.set(key.clone(), vec![2]);

        let mut cache = KVCache::default();

        cache.delete.insert(key.clone());

        let store = build_store(tree, Some(cache));

        // ---
        let result = store.get(&key);

        // ---
        assert_eq!(None, result);
    }

    #[test]
    fn range_work_for_persist_values() {
        let mut tree = build_tree();

        let values_insert = [
            (1, 11),
            (2, 22),
            (3, 33),
            (4, 44),
            (5, 55),
            (6, 66),
            (7, 77),
            (8, 88),
            (9, 99),
            (10, 100),
        ]
        .into_iter()
        .map(|(key, value)| (vec![key], vec![value]))
        .collect::<BTreeMap<_, _>>();

        for (key, value) in values_insert.clone() {
            tree.set(key, value);
        }

        let range = vec![4]..vec![8];

        let expected_range = values_insert
            .into_iter()
            .collect::<BTreeMap<_, _>>()
            .range(range.clone())
            .map(|(key, value)| (Cow::Owned(key.clone()), Cow::Owned(value.clone())))
            .collect::<BTreeMap<_, _>>();

        let store = build_store(tree, None);

        // ---
        let range = store.range(range).collect::<BTreeMap<_, _>>();

        // ---
        assert_eq!(expected_range, range);
    }

    #[test]
    fn range_work_for_persist_and_cached_values() {
        let mut tree = build_tree();

        for (key, value) in [
            (1, 11),
            (2, 22),
            (3, 33),
            (4, 44),
            (5, 55),
            (6, 66),
            (8, 88),
            (9, 99),
            (10, 100),
        ] {
            tree.set(vec![key], vec![value]);
        }

        let mut cache = KVCache::default();

        cache.storage.insert(vec![6], vec![60]); // Overrides old value
        cache.storage.insert(vec![7], vec![77]); // Adds new value

        let range = vec![4]..vec![8];

        let store = build_store(tree, Some(cache));

        // ---
        let result_range = store.range(range.clone()).collect::<BTreeMap<_, _>>();

        // ---

        let expected_range = [
            (vec![4_u8], vec![44_u8]),
            (vec![5], vec![55]),
            (vec![6], vec![60]),
            (vec![7], vec![77]),
        ]
        .into_iter()
        .collect::<BTreeMap<_, _>>()
        .range(range)
        .map(|(key, value)| (Cow::Owned(key.clone()), Cow::Owned(value.clone())))
        .collect::<BTreeMap<_, _>>();

        assert_eq!(expected_range, result_range);
    }

    #[test]
    fn range_work_for_persist_values_without_deleted() {
        let mut tree = build_tree();

        for (key, value) in [
            (1, 11),
            (2, 22),
            (3, 33),
            (4, 44),
            (5, 55),
            (6, 66),
            (7, 77),
            (8, 88),
            (9, 99),
            (10, 100),
        ] {
            tree.set(vec![key], vec![value]);
        }

        let mut cache = KVCache::default();

        cache.delete.insert(vec![5]);
        cache.delete.insert(vec![6]);

        let range = vec![4]..vec![8];

        let store = build_store(tree, Some(cache));

        // ---
        let result_range = store.range(range.clone()).collect::<BTreeMap<_, _>>();

        // ---

        let expected_range = [(vec![4_u8], vec![44_u8]), (vec![7], vec![77])]
            .into_iter()
            .collect::<BTreeMap<_, _>>()
            .range(range)
            .map(|(key, value)| (Cow::Owned(key.clone()), Cow::Owned(value.clone())))
            .collect::<BTreeMap<_, _>>();

        assert_eq!(expected_range, result_range);
    }

    #[test]
    fn range_work_for_persist_and_cached_values_without_deleted() {
        let mut tree = build_tree();

        for (key, value) in [
            (1, 11),
            (2, 22),
            (3, 33),
            (4, 44),
            (5, 55),
            (6, 66),
            (7, 77),
            (8, 88),
            (9, 99),
            (10, 100),
        ] {
            tree.set(vec![key], vec![value]);
        }

        let mut cache = KVCache::default();

        cache.storage.insert(vec![4], vec![40]);
        cache.delete.insert(vec![5]);
        cache.delete.insert(vec![6]);

        let range = vec![4]..vec![8];

        let store = build_store(tree, Some(cache));

        // ---
        let result_range = store.range(range.clone()).collect::<BTreeMap<_, _>>();

        // ---

        let expected_range = [(vec![4_u8], vec![40_u8]), (vec![7], vec![77])]
            .into_iter()
            .collect::<BTreeMap<_, _>>()
            .range(range)
            .map(|(key, value)| (Cow::Owned(key.clone()), Cow::Owned(value.clone())))
            .collect::<BTreeMap<_, _>>();

        assert_eq!(expected_range, result_range);
    }

    fn build_tree() -> Tree<MemDB> {
        Tree::new(
            MemDB::new(),
            None,
            TREE_CACHE_SIZE
                .try_into()
                .expect("Unreachable. Tree cache size is > 0"),
            None,
        )
        .expect("Failed to create Tree")
    }

    fn build_store(tree: Tree<MemDB>, cache: Option<KVCache>) -> ApplicationKVBank<MemDB> {
        ApplicationKVBank {
            persistent: Arc::new(RwLock::new(tree)),
            cache: cache.unwrap_or_default(),
        }
    }
}
