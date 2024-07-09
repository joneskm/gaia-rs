use std::{
    borrow::Cow,
    ops::RangeBounds,
    sync::{Arc, RwLock},
};

use database::Database;
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

#[derive(Debug)]
pub struct ApplicationKVBank<DB> {
    pub(crate) persistent: Arc<RwLock<Tree<DB>>>,
    pub(crate) cache: KVCache,
}

impl<DB: Database> ApplicationKVBank<DB> {
    pub fn new(db: DB, target_version: Option<u32>) -> Result<Self, KVStoreError> {
        Ok(Self {
            persistent: Arc::new(RwLock::new(Tree::new(
                db,
                target_version,
                TREE_CACHE_SIZE
                    .try_into()
                    .expect("Unreachable. Tree cache size is > 0"),
            )?)),
            cache: Default::default(),
        })
    }

    /// Read persistent database
    #[inline]
    pub fn persistent(&self) -> std::sync::RwLockReadGuard<Tree<DB>> {
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
        self.cache.get(k.as_ref()).ok()?.cloned().or(self
            .persistent
            .read()
            .expect(POISONED_LOCK)
            .get(k.as_ref()))
    }

    pub fn prefix_store<I: IntoIterator<Item = u8>>(
        &self,
        prefix: I,
    ) -> ImmutablePrefixStore<'_, DB> {
        ImmutablePrefixStore {
            store: KVStore::from(self),
            prefix: prefix.into_iter().collect(),
        }
    }

    pub fn prefix_store_mut<I: IntoIterator<Item = u8>>(
        &mut self,
        prefix: I,
    ) -> MutablePrefixStore<'_, DB> {
        MutablePrefixStore {
            store: KVStoreMut::from(self),
            prefix: prefix.into_iter().collect(),
        }
    }

    pub fn range<R: RangeBounds<Vec<u8>> + Clone>(&self, range: R) -> Range<'_, DB> {
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

    pub fn consume_tx_cache(&mut self, other: &mut TransactionKVBank<DB>) {
        other.tx_cache_clear();
        let (set_values, del_values) = other.block.take();

        for (key, value) in set_values {
            self.cache.set(key, value)
        }

        for del in del_values {
            self.cache.delete(&del);
        }
    }

    pub fn commit(&mut self) -> [u8; 32] {
        let (cache, delete) = self.cache.take();

        let mut persistent = self.persistent.write().expect(POISONED_LOCK);

        cache
            .into_iter()
            .filter(|(key, _)| !delete.contains(key))
            .for_each(|(key, value)| persistent.set(key, value));

        for key in delete {
            let _ = persistent.remove(&key);
        }

        //TODO: is it safe to assume this won't ever error?
        persistent.save_version().ok().unwrap_or_default().0
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
