use itertools::Itertools;

use super::{PaginationResultElement, TwoIterators};

/// Result of pagination by offset
pub type PaginationByOffsetResult<T> = PaginationResultElement<T>;

/// Struct to paginate over offset
#[derive(Debug, Clone)]
pub struct PaginationByOffset {
    /// offset to start iteration
    pub offset: usize,
    /// max amount of items
    pub limit: usize,
}

impl From<(usize, usize)> for PaginationByOffset {
    fn from((offset, limit): (usize, usize)) -> Self {
        Self { offset, limit }
    }
}

/// Trait which contains methods to paginate by offset
pub trait IteratorPaginateByOffset {
    /// Item in iterator
    type Item;

    /// Paginate by offset and get result of pagination which contains information about next offset
    fn paginate_by_offset(
        self,
        pagination: impl Into<PaginationByOffset>,
    ) -> (
        PaginationByOffsetResult<Self::Item>,
        impl Iterator<Item = Self::Item>,
    );

    /// Same as [IteratorPaginateByOffset::paginate_by_offset], but accept optional pagination.
    /// Useful when user could set pagination, but by default it's `None`
    fn maybe_paginate_by_offset<P: Into<PaginationByOffset>>(
        self,
        pagination: Option<P>,
    ) -> (
        Option<PaginationByOffsetResult<Self::Item>>,
        impl Iterator<Item = Self::Item>,
    );
}

impl<T: Iterator<Item = U>, U: Clone> IteratorPaginateByOffset for T {
    type Item = U;

    fn paginate_by_offset(
        self,
        pagination: impl Into<PaginationByOffset>,
    ) -> (
        PaginationByOffsetResult<Self::Item>,
        impl Iterator<Item = Self::Item>,
    ) {
        let PaginationByOffset { offset, limit } = pagination.into();

        let mut iterator = itertools::peek_nth(self.skip(offset * limit));

        let last = iterator.peek_nth(limit).cloned();
        let count = match iterator.try_len() {
            Ok(len) => len,
            Err((_lower_bound, upper_bound)) => upper_bound.unwrap_or(usize::MAX),
        };

        (
            PaginationResultElement::new(count, last),
            iterator.take(limit),
        )
    }

    fn maybe_paginate_by_offset<P: Into<PaginationByOffset>>(
        self,
        pagination: Option<P>,
    ) -> (
        Option<PaginationByOffsetResult<Self::Item>>,
        impl Iterator<Item = Self::Item>,
    ) {
        match pagination {
            Some(pagination) => {
                let (result, iter) = self.paginate_by_offset(pagination);
                (Some(result), TwoIterators::First(iter))
            }
            None => (None, TwoIterators::Second(self)),
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::pagination::UnwrapPagination;

    use super::*;

    #[test]
    fn all_values() {
        let array = (0..20).collect::<Vec<_>>();

        let expected = (0..20).collect::<Vec<_>>();

        let result = array
            .into_iter()
            .paginate_by_offset((0, 20))
            .unwrap_pagination()
            .collect::<Vec<_>>();

        assert_eq!(expected, result)
    }

    #[test]
    fn first_half_of_values() {
        let array = (0..20).collect::<Vec<_>>();

        let expected = (0..10).collect::<Vec<_>>();

        let result = array
            .into_iter()
            .paginate_by_offset((0, 10))
            .unwrap_pagination()
            .collect::<Vec<_>>();

        assert_eq!(expected, result)
    }

    #[test]
    fn second_half_of_values() {
        let array = (0..20).collect::<Vec<_>>();

        let expected = (10..20).collect::<Vec<_>>();

        let result = array
            .into_iter()
            .paginate_by_offset((1, 10))
            .unwrap_pagination()
            .collect::<Vec<_>>();

        assert_eq!(expected, result)
    }

    #[test]
    fn first_middle_of_values() {
        let array = (0..20).collect::<Vec<_>>();

        let expected = (5..10).collect::<Vec<_>>();

        let result = array
            .into_iter()
            .paginate_by_offset((1, 5))
            .unwrap_pagination()
            .collect::<Vec<_>>();

        assert_eq!(expected, result)
    }

    #[test]
    fn second_middle_of_values() {
        let array = (0..20).collect::<Vec<_>>();

        let expected = (10..15).collect::<Vec<_>>();

        let result = array
            .into_iter()
            .paginate_by_offset((2, 5))
            .unwrap_pagination()
            .collect::<Vec<_>>();

        assert_eq!(expected, result)
    }

    #[test]
    fn p_result_all_values() {
        let array = [vec![1_u8], vec![2], vec![3], vec![4], vec![5], vec![6]];

        let (p_result, _) = array.into_iter().paginate_by_offset((0, 2));

        let expected = PaginationResultElement::new(6, Some(vec![3]));

        assert_eq!(expected, p_result);
    }

    #[test]
    fn p_result_last_value() {
        let array = [vec![1_u8], vec![2], vec![3], vec![4], vec![5], vec![6]];

        let (p_result, _) = array.into_iter().paginate_by_offset((1, 5));

        let expected = PaginationResultElement::new(1, None);

        assert_eq!(expected, p_result);
    }

    #[test]
    fn p_result_not_existed_value() {
        let array = [vec![1_u8], vec![2], vec![3], vec![4], vec![5], vec![6]];

        let (p_result, _) = array.into_iter().paginate_by_offset((10, 10));

        let expected = PaginationResultElement::new(0, None);

        assert_eq!(expected, p_result);
    }
}
