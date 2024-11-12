pub mod infallible;

use std::{
    borrow::Cow,
    ops::{Bound, RangeBounds},
};

use database::Database;
use infallible::RangeIter;
use kv_store::{range::Range, store::prefix::range::PrefixRange};

use gas::store::{errors::GasStoreErrors, range::GasRange};

pub type VectoredStoreRange<'a, DB> = StoreRange<'a, DB, Vec<u8>, (Bound<Vec<u8>>, Bound<Vec<u8>>)>;

#[derive(Debug)]
enum StoreRangeBackend<'a, DB, RB, R> {
    Gas(GasRange<'a, DB, RB, R>),
    Kv(Range<'a, DB, RB, R>),
    Prefix(PrefixRange<'a, DB, RB, R>),
}

#[derive(Debug)]
pub struct StoreRange<'a, DB, RB, R>(StoreRangeBackend<'a, DB, RB, R>);

impl<'a, DB: Database, RB: AsRef<[u8]>, R: RangeBounds<RB>> StoreRange<'a, DB, RB, R> {
    pub fn rev_iter(self) -> Self {
        match self.0 {
            StoreRangeBackend::Gas(range) => range.rev_iter().into(),
            StoreRangeBackend::Kv(range) => range.rev_iter().into(),
            StoreRangeBackend::Prefix(range) => range.rev_iter().into(),
        }
    }

    pub fn to_infallible_iter(self) -> RangeIter<'a, DB, RB, R> {
        match self.0 {
            StoreRangeBackend::Gas(var) => var.to_infallible_iter().into(),
            StoreRangeBackend::Kv(var) => var.into(),
            StoreRangeBackend::Prefix(var) => var.into(),
        }
    }
}

impl<'a, DB: Database, RB: AsRef<[u8]>, R: RangeBounds<RB>> Iterator for StoreRange<'a, DB, RB, R> {
    type Item = Result<(Cow<'a, Vec<u8>>, Cow<'a, Vec<u8>>), GasStoreErrors>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            StoreRangeBackend::Gas(var) => var.next(),
            StoreRangeBackend::Kv(var) => var.next().map(Ok),
            StoreRangeBackend::Prefix(var) => var.next().map(Ok),
        }
    }
}

impl<'a, DB, RB, R> From<GasRange<'a, DB, RB, R>> for StoreRange<'a, DB, RB, R> {
    fn from(value: GasRange<'a, DB, RB, R>) -> Self {
        Self(StoreRangeBackend::Gas(value))
    }
}

impl<'a, DB, RB, R> From<Range<'a, DB, RB, R>> for StoreRange<'a, DB, RB, R> {
    fn from(value: Range<'a, DB, RB, R>) -> Self {
        Self(StoreRangeBackend::Kv(value))
    }
}

impl<'a, DB, RB, R> From<PrefixRange<'a, DB, RB, R>> for StoreRange<'a, DB, RB, R> {
    fn from(value: PrefixRange<'a, DB, RB, R>) -> Self {
        Self(StoreRangeBackend::Prefix(value))
    }
}
