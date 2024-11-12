use database::Database;
use gas::store::errors::GasStoreErrors;

use crate::{params::{parsed::Params, ParamKind, ParamsDeserialize, ParamsSerialize}, types::store::prefix::mutable::PrefixStoreMut};

use super::space::GasParamsSpace;

#[derive(Debug)]
pub struct GasParamsSpaceMut<'a, DB> {
    pub(super) inner: PrefixStoreMut<'a, DB>,
}

impl<DB: Database> GasParamsSpaceMut<'_, DB> {
    pub fn to_immutable(&self) -> GasParamsSpace<'_, DB> {
        GasParamsSpace {
            inner: self.inner.to_immutable(),
        }
    }
}

impl<DB: Database> GasParamsSpaceMut<'_, DB> {
    /// Return whole serialized structure.
    pub fn params<T: ParamsDeserialize>(&self) -> Result<Option<T>, GasStoreErrors> {
        self.to_immutable().params()
    }

    /// Return only field from structure.
    pub fn params_field(
        &self,
        path: &str,
        kind: ParamKind,
    ) -> Result<Option<Params>, GasStoreErrors> {
        self.to_immutable().params_field(path, kind)
    }

    pub fn params_set<T: ParamsSerialize>(&mut self, params: &T) -> Result<(), GasStoreErrors> {
        let params = params.to_raw();

        for (key, value) in params {
            self.inner.set(key.as_bytes().iter().cloned(), value)?;
        }

        Ok(())
    }

    // TODO: dangerous fn as it may break consistency
    pub fn raw_key_set(
        &mut self,
        key: impl IntoIterator<Item = u8>,
        value: impl IntoIterator<Item = u8>,
    ) -> Result<(), GasStoreErrors> {
        self.inner.set(key, value)
    }
}
