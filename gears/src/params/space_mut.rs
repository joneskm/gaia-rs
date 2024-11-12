use database::Database;
use kv_store::store::prefix::mutable::MutablePrefixStore;

use super::{parsed::Params, space::ParamsSpace, ParamKind, ParamsDeserialize, ParamsSerialize};

#[derive(Debug)]
pub struct ParamsSpaceMut<'a, DB> {
    pub(super) inner: MutablePrefixStore<'a, DB>,
}

impl<DB> ParamsSpaceMut<'_, DB> {
    pub fn to_immutable(&self) -> ParamsSpace<'_, DB> {
        ParamsSpace {
            inner: self.inner.to_immutable(),
        }
    }
}

impl<DB: Database> ParamsSpaceMut<'_, DB> {
    /// Return whole serialized structure.
    pub fn params<T: ParamsDeserialize>(&self) -> Option<T> {
        self.to_immutable().params()
    }

    /// Return only field from structure.
    pub fn params_field(&self, path: &str, kind: ParamKind) -> Option<Params> {
        self.to_immutable().params_field(path, kind)
    }

    pub fn params_set<T: ParamsSerialize>(&mut self, params: &T) {
        let params = params.to_raw();

        for (key, value) in params {
            self.inner.set(key.as_bytes().iter().cloned(), value)
        }
    }

    // TODO: dangerous fn as it may break consistency
    pub fn raw_key_set(
        &mut self,
        key: impl IntoIterator<Item = u8>,
        value: impl IntoIterator<Item = u8>,
    ) {
        self.inner.set(key, value)
    }
}
