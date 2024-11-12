use std::ops::RangeBounds;

use database::Database;
use kv_store::store::kv::immutable::KVStore;

use super::{prefix::PrefixStore, range::StoreRange};

use gas::store::{errors::GasStoreErrors, kv::GasKVStore};

pub mod mutable;

#[derive(Debug)]
pub enum StoreBackend<'a, DB> {
    Gas(GasKVStore<'a, DB>),
    Kv(KVStore<'a, DB>),
}

#[derive(Debug)]
pub struct Store<'a, DB>(StoreBackend<'a, DB>);

impl<'a, DB> From<GasKVStore<'a, DB>> for Store<'a, DB> {
    fn from(value: GasKVStore<'a, DB>) -> Self {
        Self(StoreBackend::Gas(value))
    }
}

impl<'a, DB> From<KVStore<'a, DB>> for Store<'a, DB> {
    fn from(value: KVStore<'a, DB>) -> Self {
        Self(StoreBackend::Kv(value))
    }
}

impl<'a, DB: Database> Store<'a, DB> {
    pub fn into_range<R: RangeBounds<Vec<u8>> + Clone>(
        self,
        range: R,
    ) -> StoreRange<'a, DB, Vec<u8>, R> {
        match self.0 {
            StoreBackend::Gas(var) => StoreRange::from(var.into_range(range)),
            StoreBackend::Kv(var) => StoreRange::from(var.into_range(range)),
        }
    }

    pub fn prefix_store<I: IntoIterator<Item = u8>>(self, prefix: I) -> PrefixStore<'a, DB> {
        match self.0 {
            StoreBackend::Gas(var) => var.prefix_store(prefix).into(),
            StoreBackend::Kv(var) => var.prefix_store(prefix).into(),
        }
    }
}

impl<DB: Database> Store<'_, DB> {
    pub fn get<R: AsRef<[u8]> + ?Sized>(&self, k: &R) -> Result<Option<Vec<u8>>, GasStoreErrors> {
        match &self.0 {
            StoreBackend::Gas(var) => Ok(var.get(k)?),
            StoreBackend::Kv(var) => Ok(var.get(k)),
        }
    }
}
