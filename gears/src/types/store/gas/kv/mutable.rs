use std::ops::Bound;

use database::Database;
use kv_store::types::kv::mutable::KVStoreMut;

use crate::types::store::gas::{
    errors::GasStoreErrors,
    guard::GasGuard,
    prefix::{mutable::GasPrefixStoreMut, GasPrefixStore},
    range::GasRange,
};

use super::GasKVStore;

#[derive(Debug)]
pub struct GasKVStoreMut<'a, DB> {
    pub(super) guard: GasGuard,
    pub(super) inner: KVStoreMut<'a, DB>,
}

impl<'a, DB: Database> GasKVStoreMut<'a, DB> {
    pub(crate) fn new(guard: GasGuard, inner: KVStoreMut<'a, DB>) -> Self {
        Self { guard, inner }
    }

    pub fn to_immutable(&'a self) -> GasKVStore<'a, DB> {
        GasKVStore {
            guard: self.guard.clone(),
            inner: self.inner.to_immutable(),
        }
    }

    pub fn into_range(self, range: (Bound<Vec<u8>>, Bound<Vec<u8>>)) -> GasRange<'a, DB> {
        GasRange::new_kv(self.inner.into_range(range), self.guard.clone())
    }

    pub fn prefix_store<I: IntoIterator<Item = u8>>(self, prefix: I) -> GasPrefixStore<'a, DB> {
        GasPrefixStore::new(self.guard, self.inner.prefix_store(prefix))
    }

    pub fn prefix_store_mut(
        self,
        prefix: impl IntoIterator<Item = u8>,
    ) -> GasPrefixStoreMut<'a, DB> {
        GasPrefixStoreMut::new(self.guard, self.inner.prefix_store_mut(prefix))
    }
}

impl<DB: Database> GasKVStoreMut<'_, DB> {
    pub fn get<R: AsRef<[u8]> + ?Sized>(&self, k: &R) -> Result<Option<Vec<u8>>, GasStoreErrors> {
        let value = self.inner.get(&k);

        self.guard
            .get(k.as_ref().len(), value.as_ref().map(|this| this.len()))?;

        Ok(value)
    }

    pub fn set<KI: IntoIterator<Item = u8>, VI: IntoIterator<Item = u8>>(
        &mut self,
        key: KI,
        value: VI,
    ) -> Result<(), GasStoreErrors> {
        let key = key.into_iter().collect::<Vec<_>>();
        let value = value.into_iter().collect::<Vec<_>>();

        self.guard.set(key.len(), value.len())?;

        self.inner.set(key, value);

        Ok(())
    }

    pub fn delete(&mut self, k: &[u8]) -> Result<Option<Vec<u8>>, GasStoreErrors> {
        self.guard.delete()?;

        Ok(self.inner.delete(k))
    }
}
