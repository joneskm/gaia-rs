//! Implementation of KV Store for storing data during transaction

use std::{
    borrow::Cow,
    collections::HashMap,
    ops::RangeBounds,
    sync::{Arc, RwLock},
};

use database::Database;
use trees::iavl::Tree;

use crate::{
    cache::{KVCache, KVCacheCollection},
    error::POISONED_LOCK,
    range::Range,
    store::{
        kv::{immutable::KVStore, mutable::KVStoreMut},
        prefix::{immutable::ImmutablePrefixStore, mutable::MutablePrefixStore},
    },
    utils::MergedRange,
};

use super::application::ApplicationKVBank;

/// Store used during processing of transactions.
/// In current implementation it contains 2 layers of cache before ~~commit~~
/// (this store couldn't commit changes to and instead cache should be taken and put into application).
/// which contains tx layer which get cleared if processing tx fails and block which contains cache of all
/// successful transactions.
#[derive(Debug)]
pub struct TransactionKVBank<DB> {
    pub(crate) persistent: Arc<RwLock<Tree<DB>>>,
    pub(crate) tx: KVCache,
    pub(crate) block: KVCache,
}

impl<DB: Database> TransactionKVBank<DB> {
    /// Read persistent database
    #[inline]
    fn persistent(&self) -> std::sync::RwLockReadGuard<'_, Tree<DB>> {
        self.persistent.read().expect(POISONED_LOCK)
    }

    /// Clear uncommitted cache for tx
    #[inline]
    pub fn tx_cache_clear(&mut self) {
        self.tx.storage.clear();
        self.tx.delete.clear();
    }

    /// Clear uncommitted cache for block
    #[inline]
    pub fn block_cache_clear(&mut self) {
        self.block.storage.clear();
        self.block.delete.clear();
    }

    /// Upgrade cache means push changes from tx to block
    pub fn upgrade_cache(&mut self) {
        let (set_values, delete) = self.tx.take();
        for (key, value) in set_values {
            self.block.set(key, value);
        }

        for del in delete {
            self.block.delete(&del);
        }
    }

    /// Take(leaving empty) cache from block layer
    pub fn take_block_cache(&mut self) -> KVCacheCollection {
        self.block.take()
    }

    /// Delete value from storage
    #[inline]
    pub fn delete(&mut self, k: &[u8]) -> Option<Vec<u8>> {
        self.tx
            .delete(k)
            .or_else(|| self.block.storage.get(k).cloned())
            .or_else(|| self.persistent().get(k))
    }

    /// Set or append value
    #[inline]
    pub fn set<KI: IntoIterator<Item = u8>, VI: IntoIterator<Item = u8>>(
        &mut self,
        key: KI,
        value: VI,
    ) {
        self.tx.set(key, value)
    }

    /// Clone changes from [ApplicationKVBank].
    /// This fn should be used in case changes made in application layer or [ApplicationKVBank].
    /// This is not efficient, but solved problem with cache sync without
    /// complicated relations between application and transaction layers of application.
    pub fn append_block_cache(&mut self, other: &mut ApplicationKVBank<DB>) {
        let (append, delete) = (other.cache.storage.clone(), other.cache.delete.clone());

        for (key, value) in append {
            self.block.set(key, value);
        }
        for key in delete {
            self.block.delete(&key);
        }
    }

    /// Return value of key from cache of persisted db.
    /// Value from tx layer overwrites block values.
    ///
    /// *Note*: value will be fetched in db only if no values found in both layers
    pub fn get<R: AsRef<[u8]> + ?Sized>(&self, k: &R) -> Option<Vec<u8>> {
        match self.tx.get(k.as_ref()).ok()? {
            Some(var) => Some(var.to_owned()),
            None => self
                .block
                .get(k.as_ref())
                .ok()?
                .cloned()
                .or_else(|| self.persistent().get(k.as_ref())),
        }
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
            .block
            .storage
            .range(range.clone())
            .chain(self.tx.storage.range(range.clone()))
            .map(|(first, second)| (Cow::Borrowed(first), Cow::Borrowed(second)))
            .collect::<HashMap<_, _>>();

        let tree = self.persistent();
        let persisted_values = tree
            .range(range)
            .filter(|(key, _)| {
                !(self.tx.delete.contains(&**key)
                    || (self.block.delete.contains(&**key)
                        && !self.tx.storage.contains_key(&**key)))
            })
            .map(|(first, second)| (Cow::Owned(first), Cow::Owned(second)));

        MergedRange::merge(cached_values.into_iter(), persisted_values).into()
    }
}

#[cfg(test)]
mod tests {

    use std::collections::BTreeMap;

    use database::MemDB;

    use crate::{bank::kv::test_utils::tx_store_build, TREE_CACHE_SIZE};

    use super::*;

    /// # What
    /// Test checks that empty cache on upgrade still empty
    #[test]
    fn upgrade_cache_1() {
        let mut store = tx_store_build([(0, 0)], [], [(1, 11)], [], [2]);

        store.upgrade_cache();

        let expected_store = tx_store_build([(0, 0)], [], [(1, 11)], [], [2]);

        assert_eq!(expected_store.tx, store.tx);
        assert_eq!(expected_store.block, store.block);

        let expected_get = store.get(&[0]);
        assert_eq!(Some(vec![0]), expected_get)
    }

    /// # What
    /// Test checks that after upgrade cache is correct
    #[test]
    fn upgrade_cache_2() {
        let mut store = tx_store_build([(0, 0)], [(1, 111)], [(1, 11), (3, 33)], [3], [2]);

        store.upgrade_cache();

        let expected_store = tx_store_build([(0, 0)], [], [(1, 111)], [], [2, 3]);

        assert_eq!(expected_store.tx, store.tx);
        assert_eq!(expected_store.block, store.block);

        let expected_get = store.get(&[0]);
        assert_eq!(Some(vec![0]), expected_get)
    }

    /// # What
    /// Test checks that after upgrade cache is correct
    #[test]
    fn upgrade_cache_3() {
        let mut store = tx_store_build([(0, 0)], [(1, 111)], [(1, 11), (3, 33)], [0, 3], [2]);

        store.upgrade_cache();

        let expected_store = tx_store_build([(0, 0)], [], [(1, 111)], [], [2, 3, 0]);

        assert_eq!(expected_store.tx, store.tx);
        assert_eq!(expected_store.block, store.block);

        let expected_get = store.get(&[0]);
        assert_eq!(None, expected_get)
    }

    /// # What
    /// Test checks that after upgrade cache is correct
    #[test]
    fn upgrade_cache_4() {
        let mut store = tx_store_build(
            [(0, 0)],
            [(1, 111), (1, 0), (1, 10)],
            [(1, 11), (3, 33)],
            [0, 3],
            [2],
        );

        store.upgrade_cache();

        let expected_store = tx_store_build([(0, 0)], [], [(1, 10)], [], [2, 3, 0]);

        assert_eq!(expected_store.tx, store.tx);
        assert_eq!(expected_store.block, store.block);

        let expected_get = store.get(&[0]);
        assert_eq!(None, expected_get)
    }

    /// # What
    /// Test checks that we get value from tx cache
    #[test]
    fn get_from_tx_cache_empty_persisted() {
        let store = tx_store_build([], [(1, 11)], [], [], []);
        // ---
        let result = store.get(&[1]);
        // ---
        assert_eq!(Some(vec![11]), result)
    }

    /// # What
    /// Test checks that we get value from block cache
    #[test]
    fn get_from_block_cache_empty_persisted() {
        let store = tx_store_build([], [], [(1, 11)], [], []);
        // ---
        let result = store.get(&[1]);
        // ---
        assert_eq!(Some(vec![11]), result)
    }

    /// # What
    /// Test checks that we get value from persisted db if any cache empty
    #[test]
    fn get_from_persisted() {
        let store = build_store(build_tree([(1, 22)]), None);
        // ---
        let result = store.get(&[1]);
        // ---
        assert_eq!(Some(vec![22]), result)
    }

    /// # What
    /// Test checks that we get None value from persisted db if value deleted in tx cache
    #[test]
    fn get_from_persisted_deleted_in_tx() {
        let mut store = build_store(build_tree([(1, 22)]), None);
        store.delete(&[1]);
        // ---
        let result = store.get(&[1]);
        // ---
        assert_eq!(None, result)
    }

    /// # What
    /// Test checks that we get None value from persisted db if value deleted in block cache
    #[test]
    fn get_from_persisted_deleted_in_block() {
        let mut store = build_store(build_tree([(1, 22)]), None);
        store.delete(&[1]);
        store.upgrade_cache();
        // ---
        let result = store.get(&[1]);
        // ---
        assert_eq!(None, result)
    }

    /// # What
    /// Test checks that we get None value from persisted db if value deleted in all caches
    #[test]
    fn get_from_persisted_deleted_in_block_and_tx() {
        let mut store = build_store(build_tree([(1, 22)]), None);
        store.delete(&[1]);
        store.upgrade_cache();
        store.delete(&[1]);
        // ---
        let result = store.get(&[1]);
        // ---
        assert_eq!(None, result)
    }

    /// # What
    /// Test checks that we try get value from store it gets overwritten by tx cache
    #[test]
    fn get_from_persisted_overwritten_by_tx() {
        let mut store = build_store(build_tree([(1, 22)]), None);
        store.set(vec![1], vec![11]);
        // ---
        let result = store.get(&[1]);
        // ---
        assert_eq!(Some(vec![11]), result)
    }

    /// # What
    /// Test checks that we try get value from store it gets overwritten by block cache
    #[test]
    fn get_from_persisted_overwritten_by_block() {
        let mut store = build_store(build_tree([(1, 22)]), None);
        store.set(vec![1], vec![11]);
        store.upgrade_cache();
        // ---
        let result = store.get(&[1]);
        // ---
        assert_eq!(Some(vec![11]), result)
    }

    /// # What
    /// Test checks that we get value from tx cache while key exists in block and persisted
    #[test]
    fn get_from_tx_cache_override_persisted_and_block() {
        let store = tx_store_build([(1, 11)], [(1, 22)], [(1, 33)], [], []);
        // ---
        let result = store.get(&[1]);
        // ---
        assert_eq!(Some(vec![22]), result)
    }

    /// # What
    /// Test checks that we get None from tx cache while key exists in block and persisted
    #[test]
    fn get_deleted_from_tx_cache_override_persisted_and_block() {
        let mut store = tx_store_build([(1, 11)], [(1, 22)], [(1, 33)], [], []);
        store.delete(&[1]);
        // ---
        let result = store.get(&[1]);
        // ---
        assert_eq!(None, result)
    }

    /// # What
    /// Test checks that we get value from tx while value deleted in block cache
    #[test]
    fn get_from_tx_cache_while_deleted_in_block() {
        let store = tx_store_build([(1, 11)], [(1, 22)], [(1, 33)], [], [1]);
        // ---
        let result = store.get(&[1]);
        // ---
        assert_eq!(Some(vec![22]), result)
    }

    /// # What
    /// Test checks that we get None value from block if value deleted in tx cache
    #[test]
    fn get_none_from_block_cache_cause_deleted_in_tx() {
        let store = tx_store_build([(1, 11)], [(1, 22)], [(1, 33)], [1], []);
        // ---
        let result = store.get(&[1]);
        // ---
        assert_eq!(None, result)
    }

    /// # What
    /// Test checks that we get deleted value from tx cache
    #[test]
    fn delete_taken_from_tx_cache() {
        let mut store = tx_store_build([(1, 11)], [(1, 22)], [(1, 33)], [], []);
        // ---
        let deleted = store.delete(&[1]);
        // ---
        assert_eq!(Some(vec![22]), deleted);
    }

    /// # What
    /// Test checks that we get deleted value from tx cache
    #[test]
    fn set_override_another_set() {
        let mut store = tx_store_build([(1, 0)], [], [(1, 0)], [], []);
        store.set(vec![1], vec![11]);
        store.upgrade_cache();
        store.set(vec![1], vec![22]);
        store.upgrade_cache();
        store.set(vec![1], vec![33]);
        store.upgrade_cache();

        // ---
        let get = store.get(&[1]);
        // ---
        assert_eq!(Some(vec![33]), get);
    }

    /// # What
    /// Test checks that we really overset value in tx
    #[test]
    fn set_then_get_then_set_then_get_in_tx() {
        let mut store = tx_store_build([(1, 0)], [], [], [], []);

        let get = store.get(&[1]);
        assert_eq!(Some(vec![0]), get);

        store.set(vec![1], vec![11]);
        let get = store.get(&[1]);
        assert_eq!(Some(vec![11]), get);

        store.set(vec![1], vec![22]);
        let get = store.get(&[1]);
        assert_eq!(Some(vec![22]), get);

        store.set(vec![1], vec![33]);
        let get = store.get(&[1]);
        assert_eq!(Some(vec![33]), get);
    }

    /// # What
    /// Test checks that we really overset value with upgrades of cache.
    #[test]
    fn set_then_get_then_set_then_get_with_upgrades() {
        let mut store = tx_store_build([(1, 0)], [], [], [], []);

        let get = store.get(&[1]);
        assert_eq!(Some(vec![0]), get);

        store.set(vec![1], vec![11]);
        let get = store.get(&[1]);
        assert_eq!(Some(vec![11]), get);
        store.upgrade_cache();
        let get = store.get(&[1]);
        assert_eq!(Some(vec![11]), get);

        store.set(vec![1], vec![22]);
        let get = store.get(&[1]);
        assert_eq!(Some(vec![22]), get);
        store.upgrade_cache();
        let get = store.get(&[1]);
        assert_eq!(Some(vec![22]), get);

        store.set(vec![1], vec![33]);
        let get = store.get(&[1]);
        assert_eq!(Some(vec![33]), get);
        store.upgrade_cache();
        let get = store.get(&[1]);
        assert_eq!(Some(vec![33]), get);
    }

    /// ================================== OLD =============

    #[test]
    fn get_empty_cache() {
        let mut tree = build_tree([]);

        let key = vec![1];

        tree.set(key.clone(), vec![2]);

        let store = build_store(tree, None);

        // ---
        let result = store.get(&key);

        // ---
        assert_eq!(Some(vec![2]), result);
    }

    #[test]
    fn get_from_tx_cache() {
        let mut tree = build_tree([]);

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
    fn get_from_block_cache() {
        let mut tree = build_tree([]);

        let key = vec![1];

        tree.set(key.clone(), vec![2]);

        let mut cache = KVCache::default();

        cache.storage.insert(key.clone(), vec![3]);

        let mut store = build_store(tree, Some(cache));
        store.upgrade_cache();

        // ---
        let result = store.get(&key);

        // ---
        assert_eq!(Some(vec![3]), result);
    }

    #[test]
    fn get_from_tx_overwriting_block_cache() {
        let mut tree = build_tree([]);
        tree.set(vec![1], vec![2]);

        let mut cache = KVCache::default();

        cache.storage.insert(vec![1], vec![3]);

        let mut store = build_store(tree, Some(cache));
        store.upgrade_cache();
        store.set(vec![1], vec![4]);

        // ---
        let result = store.get(&vec![1]);

        // ---
        assert_eq!(Some(vec![4]), result);
    }

    #[test]
    fn get_deleted_in_tx() {
        let mut tree = build_tree([]);

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
        let mut tree = build_tree([]);

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
        let mut tree = build_tree([]);

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
    fn range_work_for_persist_and_cached_values_with_block() {
        let mut tree = build_tree([]);

        for (key, value) in [(1, 11), (2, 22), (3, 33), (4, 44)] {
            tree.set(vec![key], vec![value]);
        }

        let mut cache = KVCache::default();

        cache.storage.insert(vec![1], vec![111]); // Overrides old value

        let range = ..vec![4];

        let mut store = build_store(tree, Some(cache));
        store.upgrade_cache();

        store.set(vec![2], vec![222]);

        // ---
        let result_range = store.range(range.clone()).collect::<BTreeMap<_, _>>();

        // ---

        let expected_range = [
            (vec![1_u8], vec![111_u8]),
            (vec![2], vec![222]),
            (vec![3], vec![33]),
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
        let mut tree = build_tree([]);

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
    fn range_work_for_persist_values_without_deleted_with_block() {
        let mut tree = build_tree([]);

        for (key, value) in [(1, 11), (2, 22), (3, 33), (4, 44), (5, 55), (6, 66)] {
            tree.set(vec![key], vec![value]);
        }

        let mut cache = KVCache::default();

        cache.delete.insert(vec![1]);
        cache.delete.insert(vec![2]);

        let range = ..vec![6];

        let mut store = build_store(tree, Some(cache));
        store.upgrade_cache();
        store.delete(&[3]);

        // ---
        let result_range = store.range(range.clone()).collect::<BTreeMap<_, _>>();

        // ---

        let expected_range = [(vec![4_u8], vec![44_u8]), (vec![5], vec![55])]
            .into_iter()
            .collect::<BTreeMap<_, _>>()
            .range(range)
            .map(|(key, value)| (Cow::Owned(key.clone()), Cow::Owned(value.clone())))
            .collect::<BTreeMap<_, _>>();

        assert_eq!(expected_range, result_range);
    }

    #[test]
    fn range_work_for_persist_and_cached_values_without_deleted() {
        let mut tree = build_tree([]);

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

    #[test]
    fn range_work_for_persist_and_cached_values_without_deleted_with_block() {
        let mut tree = build_tree([]);

        for (key, value) in [(1, 11), (2, 22), (3, 33), (4, 44), (5, 55), (6, 66)] {
            tree.set(vec![key], vec![value]);
        }

        let mut cache = KVCache::default();

        cache.storage.insert(vec![1], vec![111]);
        cache.delete.insert(vec![2]);
        cache.delete.insert(vec![5]);

        let range = ..vec![6];

        let mut store = build_store(tree, Some(cache));
        store.upgrade_cache();

        store.set(vec![1], vec![1]);
        store.set(vec![3], vec![3]);
        store.set(vec![5], vec![55]);
        store.delete(&[4]);

        // ---
        let result_range = store.range(range.clone()).collect::<BTreeMap<_, _>>();

        // ---

        let expected_range = [
            (vec![1_u8], vec![1_u8]),
            (vec![3], vec![3]),
            (vec![5], vec![55]),
        ]
        .into_iter()
        .collect::<BTreeMap<_, _>>()
        .range(range)
        .map(|(key, value)| (Cow::Owned(key.clone()), Cow::Owned(value.clone())))
        .collect::<BTreeMap<_, _>>();

        assert_eq!(expected_range, result_range);
    }

    fn build_tree(values: impl IntoIterator<Item = (u8, u8)>) -> Tree<MemDB> {
        let mut tree = Tree::new(
            MemDB::new(),
            None,
            TREE_CACHE_SIZE
                .try_into()
                .expect("Unreachable. Tree cache size is > 0"),
            None,
        )
        .expect("Failed to create Tree");

        for (key, value) in values {
            tree.set(vec![key], vec![value]);
        }

        tree
    }

    fn build_store(tree: Tree<MemDB>, cache: Option<KVCache>) -> TransactionKVBank<MemDB> {
        TransactionKVBank {
            persistent: Arc::new(RwLock::new(tree)),
            tx: cache.unwrap_or_default(),
            block: Default::default(),
        }
    }
}
