#![allow(clippy::type_complexity)]

use std::{borrow::Cow, collections::VecDeque};

/// Favours a over b if keys are equal (so make a the cache)
#[derive(Debug, Clone)]
pub struct MergedRange<'a> {
    a: VecDeque<(Cow<'a, Vec<u8>>, Cow<'a, Vec<u8>>)>,
    b: VecDeque<(Cow<'a, Vec<u8>>, Cow<'a, Vec<u8>>)>,
}

impl<'a> MergedRange<'a> {
    pub fn merge<A, B>(a: A, b: B) -> MergedRange<'a>
    where
        A: Iterator<Item = (Cow<'a, Vec<u8>>, Cow<'a, Vec<u8>>)>,
        B: Iterator<Item = (Cow<'a, Vec<u8>>, Cow<'a, Vec<u8>>)>,
    {
        MergedRange {
            a: a.collect::<VecDeque<_>>(),
            b: b.collect::<VecDeque<_>>(),
        }
    }
}

impl<'a> Iterator for MergedRange<'a> {
    type Item = (Cow<'a, Vec<u8>>, Cow<'a, Vec<u8>>);

    fn next(&mut self) -> Option<Self::Item> {
        let peek_a = self.a.front();
        let peek_b = self.b.front();

        match (peek_a, peek_b) {
            (Some(peek_a), Some(peek_b)) if peek_a.0 < peek_b.0 => self.a.pop_front(),
            (Some(peek_a), Some(peek_b)) if peek_a.0 == peek_b.0 => {
                self.b.pop_front();
                self.a.pop_front()
            }
            (Some(_), Some(_)) => self.b.pop_front(),
            (Some(_), None) => self.a.pop_front(),
            (None, _) => self.b.pop_front(),
        }
    }
}

impl<'a> DoubleEndedIterator for MergedRange<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let peek_a = self.a.back();
        let peek_b = self.b.back();

        match (peek_a, peek_b) {
            (Some(peek_a), Some(peek_b)) if peek_a.0 > peek_b.0 => self.a.pop_back(),
            (Some(peek_a), Some(peek_b)) if peek_a.0 == peek_b.0 => {
                self.b.pop_back();
                self.a.pop_back()
            }
            (Some(_), Some(_)) => self.b.pop_back(),
            (Some(_), None) => self.a.pop_back(),
            (None, _) => self.b.pop_back(),
        }
    }
}

// TODO: are we assuming a and/or b are sorted? Does IAVL tree order items in range, BTreeMap does?
#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn merge_works_with_different_cow() {
        let a = [
            (vec![1u8], vec![10u8]),
            (vec![3], vec![11]),
            (vec![5], vec![12]),
        ]
        .into_iter()
        .map(|(first, second)| (Cow::Owned(first), Cow::Owned(second)));

        let b = [
            (vec![2_u8], vec![13_u8]),
            (vec![4], vec![14]),
            (vec![5], vec![15]),
        ]
        .into_iter()
        .map(|(first, second)| (Cow::Owned(first), Cow::Owned(second)));

        let got_pairs = MergedRange::merge(a, b).collect::<Vec<_>>();

        let tmp_val_1 = vec![1u8];
        let tmp_val_2 = vec![14u8];
        let expected_pairs = vec![
            (Cow::Borrowed(&tmp_val_1), Cow::Owned(vec![10u8])),
            (Cow::Owned(vec![2u8]), Cow::Owned(vec![13u8])),
            (Cow::Owned(vec![3u8]), Cow::Owned(vec![11u8])),
            (Cow::Owned(vec![4u8]), Cow::Borrowed(&tmp_val_2)),
            (Cow::Owned(vec![5u8]), Cow::Owned(vec![12u8])),
        ];

        assert_eq!(expected_pairs, got_pairs);
    }

    // This differs from the previous test in that iterator b reaches the duplicated value first
    #[test]
    fn merge_works_a_duplicates_b() {
        let a = [(1, 10), (3, 11), (5, 12)]
            .into_iter()
            .map(|(first, second)| (Cow::Owned(vec![first]), Cow::Owned(vec![second])));
        let b = [(vec![2], vec![13]), (vec![5], vec![15])]
            .into_iter()
            .map(|(first, second)| (Cow::Owned(first), Cow::Owned(second)));

        let got_pairs: Vec<_> = MergedRange::merge(a, b).collect();

        let expected_pairs = vec![
            (vec![1], vec![10]),
            (vec![2], vec![13]),
            (vec![3], vec![11]),
            (vec![5], vec![12]),
        ]
        .into_iter()
        .map(|(first, second)| (Cow::Owned(first), Cow::Owned(second)))
        .collect::<Vec<_>>();

        assert_eq!(expected_pairs, got_pairs);
    }

    // This differs from the previous test in that the duplicated value is in the middle of the range
    #[test]
    fn merge_works_mid_range_duplicate() {
        let a = vec![
            (vec![1], vec![10]),
            (vec![3], vec![11]),
            (vec![5], vec![12]),
        ]
        .into_iter()
        .map(|(first, second)| (Cow::Owned(first), Cow::Owned(second)));
        let b = vec![
            (vec![2], vec![13]),
            (vec![3], vec![15]),
            (vec![4], vec![14]),
        ]
        .into_iter()
        .map(|(first, second)| (Cow::Owned(first), Cow::Owned(second)));

        let got_pairs = MergedRange::merge(a, b).collect::<Vec<_>>();

        let expected_pairs = vec![
            (vec![1], vec![10]),
            (vec![2], vec![13]),
            (vec![3], vec![11]),
            (vec![4], vec![14]),
            (vec![5], vec![12]),
        ]
        .into_iter()
        .map(|(first, second)| (Cow::Owned(first), Cow::Owned(second)))
        .collect::<Vec<_>>();

        assert_eq!(expected_pairs, got_pairs);
    }

    // DoubleEnded tests

    #[test]
    fn merge_works_with_different_cow_rev() {
        let a = [
            (vec![1u8], vec![10u8]),
            (vec![3], vec![11]),
            (vec![5], vec![12]),
        ]
        .into_iter()
        .map(|(first, second)| (Cow::Owned(first), Cow::Owned(second)));

        let b = [
            (vec![2_u8], vec![13_u8]),
            (vec![4], vec![14]),
            (vec![5], vec![15]),
        ]
        .into_iter()
        .map(|(first, second)| (Cow::Owned(first), Cow::Owned(second)));

        let got_pairs = MergedRange::merge(a, b).rev().collect::<Vec<_>>();

        let tmp_val_1 = vec![1u8];
        let tmp_val_2 = vec![14u8];
        let expected_pairs = [
            (Cow::Borrowed(&tmp_val_1), Cow::Owned(vec![10u8])),
            (Cow::Owned(vec![2u8]), Cow::Owned(vec![13u8])),
            (Cow::Owned(vec![3u8]), Cow::Owned(vec![11u8])),
            (Cow::Owned(vec![4u8]), Cow::Borrowed(&tmp_val_2)),
            (Cow::Owned(vec![5u8]), Cow::Owned(vec![12u8])),
        ]
        .into_iter()
        .rev()
        .collect::<Vec<_>>();

        assert_eq!(expected_pairs, got_pairs);
    }

    // This differs from the previous test in that iterator b reaches the duplicated value first
    #[test]
    fn merge_works_a_duplicates_b_rev() {
        let a = [(1, 10), (3, 11), (5, 12)]
            .into_iter()
            .map(|(first, second)| (Cow::Owned(vec![first]), Cow::Owned(vec![second])));
        let b = [(vec![2], vec![13]), (vec![5], vec![15])]
            .into_iter()
            .map(|(first, second)| (Cow::Owned(first), Cow::Owned(second)));

        let got_pairs: Vec<_> = MergedRange::merge(a, b).rev().collect();

        let expected_pairs = vec![
            (vec![1], vec![10]),
            (vec![2], vec![13]),
            (vec![3], vec![11]),
            (vec![5], vec![12]),
        ]
        .into_iter()
        .map(|(first, second)| (Cow::Owned(first), Cow::Owned(second)))
        .rev()
        .collect::<Vec<_>>();

        assert_eq!(expected_pairs, got_pairs);
    }

    // This differs from the previous test in that the duplicated value is in the middle of the range
    #[test]
    fn merge_works_mid_range_duplicate_rev() {
        let a = vec![
            (vec![1], vec![10]),
            (vec![3], vec![11]),
            (vec![5], vec![12]),
        ]
        .into_iter()
        .map(|(first, second)| (Cow::Owned(first), Cow::Owned(second)));
        let b = vec![
            (vec![2], vec![13]),
            (vec![3], vec![15]),
            (vec![4], vec![14]),
        ]
        .into_iter()
        .map(|(first, second)| (Cow::Owned(first), Cow::Owned(second)));

        let got_pairs = MergedRange::merge(a, b).rev().collect::<Vec<_>>();

        let expected_pairs = vec![
            (vec![1], vec![10]),
            (vec![2], vec![13]),
            (vec![3], vec![11]),
            (vec![4], vec![14]),
            (vec![5], vec![12]),
        ]
        .into_iter()
        .map(|(first, second)| (Cow::Owned(first), Cow::Owned(second)))
        .rev()
        .collect::<Vec<_>>();

        assert_eq!(expected_pairs, got_pairs);
    }
}
