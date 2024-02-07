use std::{collections::HashMap, ops::RangeBounds};

use database::{Database, PrefixDB};
use trees::iavl::QueryTree;

use crate::{error::Error, ImmutablePrefixStore, KVStore, MultiStore, StoreKey};

pub struct QueryMultiStore<'a, DB, SK> {
    //head_version: u32,
    //head_commit_hash: [u8; 32],
    stores: HashMap<&'a SK, QueryKVStore<'a, PrefixDB<DB>>>,
}

impl<'a, DB: Database, SK: StoreKey> QueryMultiStore<'a, DB, SK> {
    pub fn new(multi_store: &'a MultiStore<DB, SK>, version: u32) -> Result<Self, Error> {
        let mut stores = HashMap::new();
        for (store, kv_store) in &multi_store.stores {
            stores.insert(store, QueryKVStore::new(kv_store, version)?);
        }

        Ok(Self {
            //head_version: version,
            //head_commit_hash: multi_store.head_commit_hash, //TODO: get the proper commit hash,
            stores,
        })
    }

    pub fn get_kv_store(&self, store_key: &SK) -> &QueryKVStore<'_, PrefixDB<DB>> {
        self.stores
            .get(store_key)
            .expect("a store for every key is guaranteed to exist")
    }
}

pub struct QueryKVStore<'a, DB> {
    persistent_store: QueryTree<'a, DB>,
}

impl<'a, DB: Database> QueryKVStore<'a, DB> {
    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.persistent_store.get(key)
    }

    pub fn new(kv_store: &'a KVStore<DB>, version: u32) -> Result<Self, Error> {
        Ok(QueryKVStore {
            persistent_store: QueryTree::new(&kv_store.persistent_store, version)?,
        })
    }

    pub fn range<'b, R>(&'b self, range: R) -> impl Iterator<Item = (Vec<u8>, Vec<u8>)> + 'b
    where
        R: RangeBounds<Vec<u8>> + Clone + 'b,
    {
        self.persistent_store.range(range)
    }

    pub fn get_immutable_prefix_store(&'a self, prefix: Vec<u8>) -> ImmutablePrefixStore<'_, DB> {
        ImmutablePrefixStore {
            store: self.into(),
            prefix,
        }
    }
}
