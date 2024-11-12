use std::{
    borrow::Cow,
    ops::{Bound, RangeBounds},
};

use database::Database;
use infallible::RangeIter;
use kv_store::{range::Range, store::prefix::range::PrefixRange};

use super::{errors::GasStoreErrors, guard::GasGuard};

pub mod infallible;

pub type VectoredGasRange<'a, DB> = GasRange<'a, DB, Vec<u8>, (Bound<Vec<u8>>, Bound<Vec<u8>>)>;

#[derive(Debug)]
enum RangeBackend<'a, DB, RB, R> {
    Kv(Range<'a, DB, RB, R>),
    Prefix(PrefixRange<'a, DB, RB, R>),
}

#[derive(Debug)]
pub struct GasRange<'a, DB, RB, R> {
    inner: RangeBackend<'a, DB, RB, R>,
    guard: GasGuard,
}

impl<'a, DB: Database, RB: AsRef<[u8]>, R: RangeBounds<RB>> GasRange<'a, DB, RB, R> {
    pub fn rev_iter(self) -> Self {
        let Self { inner, guard } = self;
        let inner = match inner {
            RangeBackend::Kv(range) => RangeBackend::Kv(range.rev_iter()),
            RangeBackend::Prefix(range) => RangeBackend::Prefix(range.rev_iter()),
        };

        Self { inner, guard }
    }
}

impl<'a, DB, RB, R> GasRange<'a, DB, RB, R> {
    pub(super) fn new_kv(inner: Range<'a, DB, RB, R>, guard: GasGuard) -> Self {
        Self {
            inner: RangeBackend::Kv(inner),
            guard,
        }
    }

    pub(super) fn new_prefix(inner: PrefixRange<'a, DB, RB, R>, guard: GasGuard) -> Self {
        Self {
            inner: RangeBackend::Prefix(inner),
            guard,
        }
    }

    pub fn to_infallible_iter(self) -> RangeIter<'a, DB, RB, R> {
        RangeIter::from(self)
    }
}

impl<'a, DB: Database, RB: AsRef<[u8]>, R: RangeBounds<RB>> Iterator for GasRange<'a, DB, RB, R> {
    type Item = Result<(Cow<'a, Vec<u8>>, Cow<'a, Vec<u8>>), GasStoreErrors>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = match &mut self.inner {
            RangeBackend::Kv(var) => var.next(),
            RangeBackend::Prefix(var) => var.next(),
        };

        let err = self.guard.range(
            next.as_ref()
                .map(|(key, val)| (key.len(), val.len(), &***key)),
        );

        match err {
            Ok(_) => next.map(Ok),
            Err(err) => Some(Err(err)),
        }
    }
}
