use std::ops::{Bound, RangeBounds};

use database::Database;

use crate::store::kv::immutable::KVStore;

use super::{
    prefix_end_bound,
    range::{PrefixRange, VectoredPrefixRange},
};

#[derive(Debug, Clone)]
pub struct ImmutablePrefixStore<'a, DB> {
    pub(crate) store: KVStore<'a, DB>,
    pub(crate) prefix: Vec<u8>,
}

impl<'a, DB: Database> ImmutablePrefixStore<'a, DB> {
    pub fn into_range<R: RangeBounds<Vec<u8>> + Clone>(
        self,
        range: R,
    ) -> VectoredPrefixRange<'a, DB> {
        let new_start = match range.start_bound() {
            Bound::Included(b) => Bound::Included([self.prefix.clone(), b.clone()].concat()),
            Bound::Excluded(b) => Bound::Excluded([self.prefix.clone(), b.clone()].concat()),
            Bound::Unbounded => Bound::Included(self.prefix.clone()),
        };

        let new_end = match range.end_bound() {
            Bound::Included(b) => Bound::Included([self.prefix.clone(), b.clone()].concat()),
            Bound::Excluded(b) => Bound::Excluded([self.prefix.clone(), b.clone()].concat()),
            Bound::Unbounded => prefix_end_bound(self.prefix.clone()),
        };

        PrefixRange {
            parent_range: self.store.into_range((new_start, new_end)),
            prefix_length: self.prefix.len(),
        }
    }
}

impl<DB: Database> ImmutablePrefixStore<'_, DB> {
    pub fn get<T: AsRef<[u8]> + ?Sized>(&self, k: &T) -> Option<Vec<u8>> {
        let full_key = [&self.prefix, k.as_ref()].concat();
        self.store.get(&full_key)
    }
}
