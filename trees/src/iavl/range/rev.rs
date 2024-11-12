use std::ops::{Bound, RangeBounds};

use database::Database;

use crate::iavl::Node;

use super::*;

impl<DB: Database, R: RangeBounds<RB>, RB: AsRef<[u8]>> Range<'_, DB, RB, R> {
    fn traverse_rev(&mut self) -> Option<(Vec<u8>, Vec<u8>)> {
        let node = self.delayed_nodes_rev.pop()?;

        let after_start = match self.range.start_bound() {
            Bound::Included(l) => node.get_key() >= l.as_ref(),
            Bound::Excluded(l) => node.get_key() > l.as_ref(),
            Bound::Unbounded => true,
        };

        let before_end = match self.range.end_bound() {
            Bound::Included(u) => node.get_key() <= u.as_ref(),
            Bound::Excluded(u) => node.get_key() < u.as_ref(),
            Bound::Unbounded => true,
        };

        match node {
            Node::Leaf(leaf) => {
                if after_start && before_end {
                    return Some((leaf.key, leaf.value));
                }
            }
            Node::Inner(inner) => {
                if after_start {
                    let left_node: Box<Node> = match inner.left_node {
                        Some(left_node) => left_node,
                        None => self
                            .node_db
                            .get_node(&inner.left_hash)
                            .expect("node db should contain all nodes"),
                    };

                    self.delayed_nodes_rev.push(*left_node)
                }

                if before_end {
                    let right_node = match inner.right_node {
                        Some(right_node) => right_node,
                        None => self
                            .node_db
                            .get_node(&inner.right_hash)
                            .expect("node db should contain all nodes"),
                    };

                    self.delayed_nodes_rev.push(*right_node)
                }
            }
        }

        self.traverse_rev()
    }
}

impl<DB: Database, R: RangeBounds<RB>, RB: AsRef<[u8]>> DoubleEndedIterator
    for Range<'_, DB, RB, R>
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.traverse_rev()
    }
}

#[cfg(test)]
mod tests {

    use database::MemDB;
    use extensions::testing::UnwrapTesting;

    use crate::iavl::Tree;

    use super::*;

    #[test]
    fn empty_range() {
        let db = MemDB::new();
        let tree = Tree::new(db, None, 100.try_into().unwrap_test(), None).unwrap_test();

        let empty_range = tree.range::<_, Vec<u8>>(..).collect::<Vec<_>>();

        assert_eq!(Vec::<(Vec<u8>, Vec<u8>)>::new(), empty_range)
    }

    #[test]
    fn simple_full() {
        let db = MemDB::new();
        let mut tree = Tree::new(db, None, 100.try_into().unwrap_test(), None).unwrap_test();

        let expected_array = [
            ([1_u8], [11_u8]),
            ([2], [22]),
            ([3], [33]),
            ([4], [44]),
            ([5], [55]),
            ([6], [66]),
            ([7], [77]),
            ([8], [88]),
            ([9], [99]),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_vec(), value.to_vec()))
        .collect::<Vec<_>>();

        for (key, value) in &expected_array {
            tree.set(key.clone(), value.clone());
        }

        tree.save_version().unwrap_test();

        let full_range = tree.range::<_, Vec<u8>>(..).rev().collect::<Vec<_>>();

        // Revert expected, but not for insert order
        let expected_array = expected_array.into_iter().rev().collect::<Vec<_>>();

        assert_eq!(expected_array, full_range)
    }

    #[test]
    fn simple_lower_included_1_upper_excluded_10_full() {
        let db = MemDB::new();
        let mut tree = Tree::new(db, None, 100.try_into().unwrap_test(), None).unwrap_test();

        let expected_array = [
            ([1_u8], [11_u8]),
            ([2], [22]),
            ([3], [33]),
            ([4], [44]),
            ([5], [55]),
            ([6], [66]),
            ([7], [77]),
            ([8], [88]),
            ([9], [99]),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_vec(), value.to_vec()))
        .collect::<Vec<_>>();

        for (key, value) in &expected_array {
            tree.set(key.clone(), value.clone());
        }

        tree.save_version().unwrap_test();

        let full_range = tree
            .range((Bound::Included(vec![1_u8]), Bound::Excluded(vec![10])))
            .rev();

        let full_range = full_range.into_iter().collect::<Vec<_>>();

        // Revert expected, but not for insert order
        let expected_array = expected_array.into_iter().rev().collect::<Vec<_>>();

        assert_eq!(expected_array, full_range)
    }

    #[test]
    fn simple_lower_included_1_upper_excluded_5() {
        let db = MemDB::new();
        let mut tree = Tree::new(db, None, 100.try_into().unwrap_test(), None).unwrap_test();

        let expected_array = [
            ([1_u8], [11_u8]),
            ([2], [22]),
            ([3], [33]),
            ([4], [44]),
            ([5], [55]),
            ([6], [66]),
            ([7], [77]),
            ([8], [88]),
            ([9], [99]),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_vec(), value.to_vec()))
        .collect::<Vec<_>>();

        for (key, value) in &expected_array {
            tree.set(key.clone(), value.clone());
        }

        tree.save_version().unwrap_test();

        let result_range = tree
            .range((Bound::Included(vec![1_u8]), Bound::Excluded(vec![5])))
            .rev()
            .collect::<Vec<_>>();

        let expected_range = expected_array
            .into_iter()
            .rev()
            .skip(5)
            .take(4)
            .collect::<Vec<_>>();

        assert_eq!(expected_range, result_range)
    }

    #[test]
    fn simple_lower_included_1_upper_included_5() {
        let db = MemDB::new();
        let mut tree = Tree::new(db, None, 100.try_into().unwrap_test(), None).unwrap_test();

        let expected_array = [
            ([1_u8], [11_u8]),
            ([2], [22]),
            ([3], [33]),
            ([4], [44]),
            ([5], [55]),
            ([6], [66]),
            ([7], [77]),
            ([8], [88]),
            ([9], [99]),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_vec(), value.to_vec()))
        .collect::<Vec<_>>();

        for (key, value) in &expected_array {
            tree.set(key.clone(), value.clone());
        }

        tree.save_version().unwrap_test();

        let result_range = tree
            .range((Bound::Included(vec![1_u8]), Bound::Included(vec![5])))
            .rev()
            .collect::<Vec<_>>();

        let expected_range = expected_array.into_iter().take(5).rev().collect::<Vec<_>>();

        assert_eq!(expected_range, result_range)
    }

    #[test]
    fn simple_lower_excluded_0_upper_excluded_10_full() {
        let db = MemDB::new();
        let mut tree = Tree::new(db, None, 100.try_into().unwrap_test(), None).unwrap_test();

        let expected_array = [
            ([1_u8], [11_u8]),
            ([2], [22]),
            ([3], [33]),
            ([4], [44]),
            ([5], [55]),
            ([6], [66]),
            ([7], [77]),
            ([8], [88]),
            ([9], [99]),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_vec(), value.to_vec()))
        .collect::<Vec<_>>();

        for (key, value) in &expected_array {
            tree.set(key.clone(), value.clone());
        }

        tree.save_version().unwrap_test();

        let result_range = tree
            .range((Bound::Excluded(vec![0_u8]), Bound::Excluded(vec![10])))
            .rev()
            .collect::<Vec<_>>();

        let expected_range = expected_array.into_iter().rev().collect::<Vec<_>>();

        assert_eq!(expected_range, result_range)
    }

    #[test]
    fn simple_lower_included_5_upper_excluded_9() {
        let db = MemDB::new();
        let mut tree = Tree::new(db, None, 100.try_into().unwrap_test(), None).unwrap_test();

        let expected_array = [
            ([1_u8], [11_u8]),
            ([2], [22]),
            ([3], [33]),
            ([4], [44]),
            ([5], [55]),
            ([6], [66]),
            ([7], [77]),
            ([8], [88]),
            ([9], [99]),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_vec(), value.to_vec()))
        .collect::<Vec<_>>();

        for (key, value) in &expected_array {
            tree.set(key.clone(), value.clone());
        }

        tree.save_version().unwrap_test();

        let result_range = tree
            .range((Bound::Included(vec![5_u8]), Bound::Excluded(vec![9])))
            .rev()
            .collect::<Vec<_>>();

        let expected_range = expected_array
            .into_iter()
            .skip(4)
            .take(4)
            .rev()
            .collect::<Vec<_>>();

        assert_eq!(expected_range, result_range)
    }

    #[test]
    fn simple_lower_excluded_5_upper_included_9() {
        let db = MemDB::new();
        let mut tree = Tree::new(db, None, 100.try_into().unwrap_test(), None).unwrap_test();

        let expected_array = [
            ([1_u8], [11_u8]),
            ([2], [22]),
            ([3], [33]),
            ([4], [44]),
            ([5], [55]),
            ([6], [66]),
            ([7], [77]),
            ([8], [88]),
            ([9], [99]),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_vec(), value.to_vec()))
        .collect::<Vec<_>>();

        for (key, value) in &expected_array {
            tree.set(key.clone(), value.clone());
        }

        tree.save_version().unwrap_test();

        let result_range = tree
            .range((Bound::Excluded(vec![5_u8]), Bound::Included(vec![9])))
            .rev()
            .collect::<Vec<_>>();

        let expected_range = expected_array
            .into_iter()
            .skip(5)
            .take(4)
            .rev()
            .collect::<Vec<_>>();

        assert_eq!(expected_range, result_range)
    }

    #[test]
    fn simple_lower_excluded_3_upper_included_7() {
        let db = MemDB::new();
        let mut tree = Tree::new(db, None, 100.try_into().unwrap_test(), None).unwrap_test();

        let expected_array = [
            ([1_u8], [11_u8]),
            ([2], [22]),
            ([3], [33]),
            ([4], [44]),
            ([5], [55]),
            ([6], [66]),
            ([7], [77]),
            ([8], [88]),
            ([9], [99]),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_vec(), value.to_vec()))
        .collect::<Vec<_>>();

        for (key, value) in &expected_array {
            tree.set(key.clone(), value.clone());
        }

        tree.save_version().unwrap_test();

        let result_range = tree
            .range((Bound::Excluded(vec![3_u8]), Bound::Included(vec![7])))
            .rev()
            .collect::<Vec<_>>();

        let expected_range = expected_array
            .into_iter()
            .skip(3)
            .take(4)
            .rev()
            .collect::<Vec<_>>();

        assert_eq!(expected_range, result_range)
    }
}
