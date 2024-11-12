use std::{borrow::Cow, ops::RangeBounds};

use database::Database;

use crate::store::errors::GasStoreErrors;

use super::{GasRange, RangeBackend};

#[derive(Debug)]
pub struct RangeIter<'a, DB, RB, R> {
    range: GasRange<'a, DB, RB, R>,
    err: Option<GasStoreErrors>,
}

impl<DB: Database, RB: AsRef<[u8]>, R: RangeBounds<RB>> RangeIter<'_, DB, RB, R> {
    pub fn rev_iter(mut self) -> Self {
        self.range = self.range.rev_iter();
        self
    }

    pub fn error(&self) -> Option<&GasStoreErrors> {
        self.err.as_ref()
    }
}

impl<'a, DB, RB, R> From<GasRange<'a, DB, RB, R>> for RangeIter<'a, DB, RB, R> {
    fn from(value: GasRange<'a, DB, RB, R>) -> Self {
        Self {
            range: value,
            err: None,
        }
    }
}

impl<'a, DB: Database, RB: AsRef<[u8]>, R: RangeBounds<RB>> Iterator for RangeIter<'a, DB, RB, R> {
    type Item = (Cow<'a, Vec<u8>>, Cow<'a, Vec<u8>>);

    fn next(&mut self) -> Option<Self::Item> {
        let next = match &mut self.range.inner {
            RangeBackend::Kv(var) => var.next(),
            RangeBackend::Prefix(var) => var.next(),
        };

        let err = self.range.guard.range(
            next.as_ref()
                .map(|(key, val)| (key.len(), val.len(), &***key)),
        );

        if let Err(err) = err {
            self.err = Some(err);

            None
        } else {
            next
        }
    }
}
