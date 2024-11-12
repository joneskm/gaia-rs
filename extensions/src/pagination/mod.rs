//! Extensions for pagination

mod key;
mod offset;

pub use self::key::*;
pub use self::offset::*;

#[derive(Debug, Clone)]
pub(crate) enum PaginationVariant {
    Offset(PaginationByOffset),
    Key(PaginationByKey),
}

/// Pagination structure
#[derive(Debug, Clone)]
pub struct Pagination(pub(crate) PaginationVariant);

impl From<PaginationByOffset> for Pagination {
    fn from(value: PaginationByOffset) -> Self {
        Self(PaginationVariant::Offset(value))
    }
}

impl From<PaginationByKey> for Pagination {
    fn from(value: PaginationByKey) -> Self {
        Self(PaginationVariant::Key(value))
    }
}

impl From<(vec1::Vec1<u8>, usize)> for Pagination {
    fn from((key, limit): (vec1::Vec1<u8>, usize)) -> Self {
        Self(PaginationVariant::Key(PaginationByKey { key, limit }))
    }
}

impl From<(usize, usize)> for Pagination {
    fn from((offset, limit): (usize, usize)) -> Self {
        Self(PaginationVariant::Offset(PaginationByOffset {
            offset,
            limit,
        }))
    }
}

/// Trait which each item should implement to iterate over items
pub trait IteratorPaginate {
    /// Item in iterator
    type Item;

    /// Paginate iterator
    fn paginate(
        self,
        pagination: impl Into<Pagination>,
    ) -> (PaginationResult, impl Iterator<Item = Self::Item>);

    /// Same as [IteratorPaginate::paginate], but accept optional pagination.
    /// Useful when user could set pagination, but by default it's `None`
    fn maybe_paginate<P: Into<Pagination>>(
        self,
        pagination: Option<P>,
    ) -> (Option<PaginationResult>, impl Iterator<Item = Self::Item>);
}

impl<T: Iterator<Item = U>, U: PaginationKey + Clone> IteratorPaginate for T {
    type Item = U;

    fn paginate(
        self,
        pagination: impl Into<Pagination>,
    ) -> (PaginationResult, impl Iterator<Item = Self::Item>) {
        let Pagination(variant) = pagination.into();
        match variant {
            PaginationVariant::Offset(pagination) => {
                let (PaginationByOffsetResult { total, next_key }, iter) =
                    self.paginate_by_offset(pagination);
                (
                    PaginationResult {
                        total,
                        next_key: next_key.map(|this| this.iterator_key().into_owned()),
                    },
                    TwoIterators::First(iter),
                )
            }
            PaginationVariant::Key(pagination) => {
                let (result, iter) = self.paginate_by_key(pagination);
                (result, TwoIterators::Second(iter))
            }
        }
    }

    fn maybe_paginate<P: Into<Pagination>>(
        self,
        pagination: Option<P>,
    ) -> (Option<PaginationResult>, impl Iterator<Item = Self::Item>) {
        match pagination {
            Some(pagination) => {
                let (result, iter) = self.paginate(pagination);
                (Some(result), TwoIterators::First(iter))
            }
            None => (None, TwoIterators::Second(self)),
        }
    }
}

#[derive(Debug, Clone)]
enum TwoIterators<I, T: Iterator<Item = I>, U: Iterator<Item = I>> {
    First(T),
    Second(U),
}

impl<I, T: Iterator<Item = I>, U: Iterator<Item = I>> Iterator for TwoIterators<I, T, U> {
    type Item = I;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            TwoIterators::First(var) => var.next(),
            TwoIterators::Second(var) => var.next(),
        }
    }
}

/// Extension methods to reduce boilerplate after pagination
pub trait UnwrapPagination<I> {
    /// Drop pagination info and paginated return iterator only
    fn unwrap_pagination(self) -> I;
}

impl<T, I: Iterator<Item = T>> UnwrapPagination<I> for (Option<PaginationResultElement<T>>, I) {
    fn unwrap_pagination(self) -> I {
        let (_, iter) = self;
        iter
    }
}

impl<T, I: Iterator<Item = T>> UnwrapPagination<I> for (PaginationResultElement<T>, I) {
    fn unwrap_pagination(self) -> I {
        let (_, iter) = self;
        iter
    }
}

/// Result of pagination. Always contains next key no matter which sort of pagination used
pub type PaginationResult = PaginationResultElement<Vec<u8>>;

/// Generis pagination result
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaginationResultElement<T> {
    /// Total amount of items
    pub total: usize,
    /// key to begin iteration
    pub next_key: Option<T>,
}

impl<T> PaginationResultElement<T> {
    /// Create new `Self`
    pub fn new(total: usize, next_element: Option<T>) -> Self {
        Self {
            total,
            next_key: next_element,
        }
    }
}
