pub mod mutable;

use std::ops::RangeBounds;

use database::Database;
use kv_store::store::kv::immutable::KVStore;

use super::{errors::GasStoreErrors, guard::GasGuard, prefix::GasPrefixStore, range::GasRange};

#[derive(Debug, Clone)]
pub struct GasKVStore<'a, DB> {
    pub(super) guard: GasGuard,
    pub(super) inner: KVStore<'a, DB>,
}

impl<'a, DB: Database> GasKVStore<'a, DB> {
    pub fn new(guard: GasGuard, inner: KVStore<'a, DB>) -> Self {
        Self { guard, inner }
    }

    pub fn into_range<R: RangeBounds<Vec<u8>> + Clone>(
        self,
        range: R,
    ) -> GasRange<'a, DB, Vec<u8>, R> {
        GasRange::new_kv(self.inner.into_range(range), self.guard.clone())
    }

    pub fn prefix_store<I: IntoIterator<Item = u8>>(self, prefix: I) -> GasPrefixStore<'a, DB> {
        GasPrefixStore::new(self.guard, self.inner.prefix_store(prefix))
    }
}

impl<DB: Database> GasKVStore<'_, DB> {
    pub fn get<R: AsRef<[u8]> + ?Sized>(&self, k: &R) -> Result<Option<Vec<u8>>, GasStoreErrors> {
        let value = self.inner.get(&k);

        self.guard.get(
            k.as_ref().len(),
            value.as_ref().map(|this| this.len()),
            k.as_ref(),
        )?;

        Ok(value)
    }
}
