//! Application kind of multi store

use std::{collections::HashMap, sync::Arc};

use database::{prefix::PrefixDB, Database};

use crate::{
    bank::kv::application::ApplicationKVBank, build_prefixed_stores, error::MultiStoreError,
    hash::StoreInfo, StoreKey,
};

use super::*;

/// Backend for application multi store
#[derive(Debug)]
pub struct ApplicationStore<DB, SK>(pub(crate) HashMap<SK, ApplicationKVBank<PrefixDB<DB>>>);

impl<SK, DB> MultiBankBackend<DB, SK> for ApplicationStore<DB, SK> {
    type Bank = ApplicationKVBank<PrefixDB<DB>>;

    fn stores(&self) -> &HashMap<SK, Self::Bank> {
        &self.0
    }

    fn stores_mut(&mut self) -> &mut HashMap<SK, Self::Bank> {
        &mut self.0
    }
}

impl<DB: Database, SK: StoreKey> MultiBank<DB, SK, ApplicationStore<DB, SK>> {
    /// Return new `self`.
    /// Method create a prefixed db for each store
    /// and makes sure that no overlap exists
    pub fn new(db: Arc<DB>) -> Result<Self, MultiStoreError<SK>> {
        let mut store_infos = Vec::new();
        let mut head_version = 0;

        let map = build_prefixed_stores::<_, SK>(db);
        let mut stores = HashMap::with_capacity(map.len());
        for (store_key, store) in map {
            let kv_store = ApplicationKVBank::new(store, None, Some(store_key.name().to_owned()))
                .map_err(|err| MultiStoreError {
                sk: store_key.clone(),
                err,
            })?;

            let store_info = StoreInfo {
                name: store_key.name().into(),
                hash: kv_store.persistent().root_hash(),
            };

            store_infos.push(store_info);
            head_version = kv_store.persistent().loaded_version();

            stores.insert(store_key, kv_store);
        }

        Ok(MultiBank {
            head_version,
            head_commit_hash: crate::hash::hash_store_infos(store_infos),
            backend: ApplicationStore(stores),
            _marker: PhantomData,
        })
    }

    /// Return tx kind of store. You need to create application kind before creation of transaction
    pub fn to_tx_kind(&self) -> TransactionMultiBank<DB, SK> {
        TransactionMultiBank {
            head_version: self.head_version,
            head_commit_hash: self.head_commit_hash,
            backend: TransactionStore(
                self.backend
                    .0
                    .iter()
                    .map(|(sk, store)| (sk.to_owned(), store.to_tx_kind()))
                    .collect(),
            ),
            _marker: PhantomData,
        }
    }

    /// Consume block cache of transaction stores
    pub fn consume_block_cache(&mut self, other: &mut TransactionMultiBank<DB, SK>) {
        for (sk, store) in &mut self.backend.0 {
            store.consume_block_cache(other.kv_store_mut(sk))
        }
    }

    /// Commit changes for all kv stores and get application hash
    pub fn commit(&mut self) -> [u8; 32] {
        let mut store_infos = vec![];
        for (store, kv_store) in &mut self.backend.0 {
            let store_info = StoreInfo {
                name: store.name().into(),
                hash: kv_store.commit(),
            };

            store_infos.push(store_info)
        }

        let hash = crate::hash::hash_store_infos(store_infos);

        self.head_commit_hash = hash;
        self.head_version = match self.head_version.checked_add(1) {
            Some(head_version) => head_version,
            None => panic!("version overflow"),
        };
        hash
    }

    /// Clear cache of all stores
    pub fn clear_cache(&mut self) {
        for store in self.backend.0.values_mut() {
            store.cache_clear();
        }
    }
}
