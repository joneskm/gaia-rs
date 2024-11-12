//! Transaction kind of multi store

use database::{prefix::PrefixDB, Database};

use crate::{bank::kv::transaction::TransactionKVBank, cache::KVCacheCollection, StoreKey};

use super::*;

/// Backend for transaction multi store
#[derive(Debug)]
pub struct TransactionStore<DB, SK>(pub(crate) HashMap<SK, TransactionKVBank<PrefixDB<DB>>>);

impl<SK, DB> MultiBankBackend<DB, SK> for TransactionStore<DB, SK> {
    type Bank = TransactionKVBank<PrefixDB<DB>>;

    fn stores(&self) -> &HashMap<SK, Self::Bank> {
        &self.0
    }

    fn stores_mut(&mut self) -> &mut HashMap<SK, Self::Bank> {
        &mut self.0
    }
}

impl<DB: Database, SK: StoreKey> MultiBank<DB, SK, TransactionStore<DB, SK>> {
    /// Clear transactions result
    pub fn tx_cache_clear(&mut self) {
        for store in self.backend.0.values_mut() {
            store.tx_cache_clear()
        }
    }

    /// Clear block cache. Meaning all changes during block will be discarded
    pub fn block_cache_clear(&mut self) {
        for store in self.backend.0.values_mut() {
            store.block_cache_clear()
        }
    }

    /// Move values from tx layer to block
    pub fn upgrade_cache(&mut self) {
        for store in self.backend.0.values_mut() {
            store.upgrade_cache()
        }
    }

    /// Clone changes from application to block layer
    pub fn append_block_cache(&mut self, other: &mut ApplicationMultiBank<DB, SK>) {
        for (sk, store) in &mut self.backend.0 {
            store.append_block_cache(other.kv_store_mut(sk))
        }
    }

    /// Take(leaving empty) block cache of all stores
    pub fn take_block_cache(&mut self) -> HashMap<SK, KVCacheCollection> {
        let mut set = HashMap::with_capacity(self.backend.0.len());
        for (sk, store) in &mut self.backend.0 {
            set.insert(sk.clone(), store.take_block_cache());
        }

        set
    }
}
