use std::{borrow::Cow, ops::RangeBounds};

use database::Database;
use gas::store::errors::GasStoreErrors;
use kv_store::{range::Range, store::prefix::range::PrefixRange};

#[derive(Debug)]
enum InfallibleRangeBackend<'a, DB, RB, R> {
    Gas(gas::store::range::infallible::RangeIter<'a, DB, RB, R>),
    Kv(Range<'a, DB, RB, R>),
    Prefix(PrefixRange<'a, DB, RB, R>),
}

#[derive(Debug)]
pub struct RangeIter<'a, DB, RB, R>(InfallibleRangeBackend<'a, DB, RB, R>);

impl<DB: Database, RB: AsRef<[u8]>, R: RangeBounds<RB>> RangeIter<'_, DB, RB, R> {
    pub fn rev_iter(self) -> Self {
        match self.0 {
            InfallibleRangeBackend::Gas(range) => range.rev_iter().into(),
            InfallibleRangeBackend::Kv(range) => range.rev_iter().into(),
            InfallibleRangeBackend::Prefix(range) => range.rev_iter().into(),
        }
    }

    pub fn error(&self) -> Option<GasStoreErrors> {
        match &self.0 {
            InfallibleRangeBackend::Gas(var) => var.error().cloned(),
            InfallibleRangeBackend::Kv(_) => None,
            InfallibleRangeBackend::Prefix(_) => None,
        }
    }
}

impl<'a, DB: Database, RB: AsRef<[u8]>, R: RangeBounds<RB>> Iterator for RangeIter<'a, DB, RB, R> {
    type Item = (Cow<'a, Vec<u8>>, Cow<'a, Vec<u8>>);

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            InfallibleRangeBackend::Gas(var) => var.next(),
            InfallibleRangeBackend::Kv(var) => var.next(),
            InfallibleRangeBackend::Prefix(var) => var.next(),
        }
    }
}

impl<'a, DB, RB, R> From<gas::store::range::infallible::RangeIter<'a, DB, RB, R>>
    for RangeIter<'a, DB, RB, R>
{
    fn from(value: gas::store::range::infallible::RangeIter<'a, DB, RB, R>) -> Self {
        Self(InfallibleRangeBackend::Gas(value))
    }
}

impl<'a, DB, RB, R> From<Range<'a, DB, RB, R>> for RangeIter<'a, DB, RB, R> {
    fn from(value: Range<'a, DB, RB, R>) -> Self {
        Self(InfallibleRangeBackend::Kv(value))
    }
}

impl<'a, DB, RB, R> From<PrefixRange<'a, DB, RB, R>> for RangeIter<'a, DB, RB, R> {
    fn from(value: PrefixRange<'a, DB, RB, R>) -> Self {
        Self(InfallibleRangeBackend::Prefix(value))
    }
}
