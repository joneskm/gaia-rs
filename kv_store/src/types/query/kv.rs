use std::ops::RangeBounds;

use database::Database;
use trees::iavl::QueryTree;

use crate::types::prefix::immutable::ImmutablePrefixStore;

#[derive(Debug)]
pub struct QueryKVStore<DB>(QueryTree<DB>);

impl<DB: Database> QueryKVStore<DB> {
    pub fn new(tree: QueryTree<DB>) -> Self {
        Self(tree)
    }
}

impl<DB: Database> QueryKVStore<DB> {
    pub fn range<R: RangeBounds<Vec<u8>> + Clone>(&self, range: R) -> crate::range::Range<'_, DB> {
        self.0.range(range).into()
    }

    pub fn get<R: AsRef<[u8]> + ?Sized>(&self, k: &R) -> Option<Vec<u8>> {
        self.0.get(k.as_ref())
    }

    pub fn prefix_store<I: IntoIterator<Item = u8>>(
        &self,
        prefix: I,
    ) -> ImmutablePrefixStore<'_, DB> {
        ImmutablePrefixStore {
            store: self.into(),
            prefix: prefix.into_iter().collect(),
        }
    }
}
