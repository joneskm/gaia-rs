use database::Database;
use kv_store::store::prefix::mutable::MutablePrefixStore;

use crate::store::{errors::GasStoreErrors, guard::GasGuard};

use super::GasPrefixStore;

#[derive(Debug)]
pub struct GasPrefixStoreMut<'a, DB> {
    inner: MutablePrefixStore<'a, DB>,
    guard: GasGuard,
}

impl<'a, DB> GasPrefixStoreMut<'a, DB> {
    pub(crate) fn new(guard: GasGuard, inner: MutablePrefixStore<'a, DB>) -> Self {
        Self { inner, guard }
    }

    pub fn to_immutable(&'a self) -> GasPrefixStore<'a, DB> {
        GasPrefixStore {
            inner: self.inner.to_immutable(),
            guard: self.guard.clone(),
        }
    }
}

impl<DB: Database> GasPrefixStoreMut<'_, DB> {
    pub fn get<T: AsRef<[u8]> + ?Sized>(&self, k: &T) -> Result<Option<Vec<u8>>, GasStoreErrors> {
        let value = self.inner.get(&k);

        self.guard.get(
            k.as_ref().len(),
            value.as_ref().map(|this| this.len()),
            k.as_ref(),
        )?;

        Ok(value)
    }

    pub fn set<KI: IntoIterator<Item = u8>, VI: IntoIterator<Item = u8>>(
        &mut self,
        k: KI,
        v: VI,
    ) -> Result<(), GasStoreErrors> {
        let key = k.into_iter().collect::<Vec<_>>();
        let value = v.into_iter().collect::<Vec<_>>();

        self.guard.set(key.len(), value.len(), &key)?;

        self.inner.set(key, value);

        Ok(())
    }

    pub fn delete(&mut self, k: &[u8]) -> Result<Option<Vec<u8>>, GasStoreErrors> {
        self.guard.delete(k)?;
        Ok(self.inner.delete(k))
    }
}
