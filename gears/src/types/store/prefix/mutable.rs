use database::Database;
use kv_store::store::prefix::mutable::MutablePrefixStore;

use gas::store::{errors::GasStoreErrors, prefix::mutable::GasPrefixStoreMut};

use super::PrefixStore;

#[derive(Debug)]
enum PrefixStoreMutBackend<'a, DB> {
    Gas(GasPrefixStoreMut<'a, DB>),
    Kv(MutablePrefixStore<'a, DB>),
}

#[derive(Debug)]
pub struct PrefixStoreMut<'a, DB>(PrefixStoreMutBackend<'a, DB>);

impl<'a, DB: Database> PrefixStoreMut<'a, DB> {
    pub fn to_immutable(&'a self) -> PrefixStore<'a, DB> {
        match &self.0 {
            PrefixStoreMutBackend::Gas(var) => {
                PrefixStore(super::PrefixStoreBackend::Gas(var.to_immutable()))
            }
            PrefixStoreMutBackend::Kv(var) => {
                PrefixStore(super::PrefixStoreBackend::Kv(var.to_immutable()))
            }
        }
    }

    pub fn delete(&mut self, k: &[u8]) -> Result<Option<Vec<u8>>, GasStoreErrors> {
        match &mut self.0 {
            PrefixStoreMutBackend::Gas(var) => Ok(var.delete(k)?),
            PrefixStoreMutBackend::Kv(var) => Ok(var.delete(k)),
        }
    }

    pub fn get<T: AsRef<[u8]> + ?Sized>(&self, k: &T) -> Result<Option<Vec<u8>>, GasStoreErrors> {
        match &self.0 {
            PrefixStoreMutBackend::Gas(var) => Ok(var.get(k)?),
            PrefixStoreMutBackend::Kv(var) => Ok(var.get(k)),
        }
    }

    pub fn set<KI: IntoIterator<Item = u8>, VI: IntoIterator<Item = u8>>(
        &mut self,
        k: KI,
        v: VI,
    ) -> Result<(), GasStoreErrors> {
        match &mut self.0 {
            PrefixStoreMutBackend::Gas(var) => Ok(var.set(k, v)?),
            PrefixStoreMutBackend::Kv(var) => {
                var.set(k, v);
                Ok(())
            }
        }
    }
}

impl<'a, DB> From<GasPrefixStoreMut<'a, DB>> for PrefixStoreMut<'a, DB> {
    fn from(value: GasPrefixStoreMut<'a, DB>) -> Self {
        Self(PrefixStoreMutBackend::Gas(value))
    }
}

impl<'a, DB> From<MutablePrefixStore<'a, DB>> for PrefixStoreMut<'a, DB> {
    fn from(value: MutablePrefixStore<'a, DB>) -> Self {
        Self(PrefixStoreMutBackend::Kv(value))
    }
}
