use std::{
    borrow::Cow,
    ops::{Bound, RangeBounds},
};

use database::Database;

use crate::range::Range;

pub type VectoredPrefixRange<'a, DB> =
    PrefixRange<'a, DB, Vec<u8>, (Bound<Vec<u8>>, Bound<Vec<u8>>)>;

#[derive(Debug, Clone)]
pub struct PrefixRange<'a, DB, RB, R> {
    pub(crate) parent_range: Range<'a, DB, RB, R>,
    pub(crate) prefix_length: usize,
}

impl<DB: Database, R: RangeBounds<RB>, RB: AsRef<[u8]>> PrefixRange<'_, DB, RB, R> {
    pub fn rev_iter(mut self) -> Self {
        self.parent_range = self.parent_range.rev_iter();
        self
    }
}

impl<'a, DB: Database, R: RangeBounds<RB>, RB: AsRef<[u8]>> Iterator
    for PrefixRange<'a, DB, RB, R>
{
    type Item = (Cow<'a, Vec<u8>>, Cow<'a, Vec<u8>>);

    fn next(&mut self) -> Option<Self::Item> {
        let (first, second) = self.parent_range.next()?;

        // Remove the prefix from the key - this is safe since all returned keys will include the prefix.
        // TODO: what if the key now has zero length, is this safe given the check on KVStore set.
        let truncated_key = first[self.prefix_length..].to_vec();

        Some((Cow::Owned(truncated_key), second))
    }
}
