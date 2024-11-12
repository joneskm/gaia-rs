mod rev;

use std::{
    marker::PhantomData,
    ops::{Bound, RangeBounds},
};

use database::Database;

use super::{node_db::NodeDB, Node};

/// Range from tree. Loads nodes on requirement
#[derive(Debug, Clone)]
pub struct Range<'a, DB, RB, R> {
    range: R,
    delayed_nodes: Vec<Node>,
    delayed_nodes_rev: Vec<Node>,
    node_db: &'a NodeDB<DB>,
    _marker: PhantomData<RB>,
}

impl<'a, DB, RB, R> Range<'a, DB, RB, R> {
    pub(crate) fn new(range: R, root: Option<Box<Node>>, node_db: &'a NodeDB<DB>) -> Self {
        Self {
            range,
            delayed_nodes: root.clone().map(|this| vec![*this]).unwrap_or_default(),
            delayed_nodes_rev: root.map(|this| vec![*this]).unwrap_or_default(),
            node_db,
            _marker: PhantomData,
        }
    }
}

impl<'a, DB: Database, R: RangeBounds<RB>, RB: AsRef<[u8]>> Range<'a, DB, RB, R> {
    fn traverse(&mut self) -> Option<(Vec<u8>, Vec<u8>)> {
        let node = self.delayed_nodes.pop()?;

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
            Node::Inner(inner) => {
                // Traverse through the left subtree, then the right subtree.
                if before_end {
                    match inner.right_node {
                        Some(right_node) => self.delayed_nodes.push(*right_node),
                        None => {
                            let right_node = self
                                .node_db
                                .get_node(&inner.right_hash)
                                .expect("node db should contain all nodes");

                            self.delayed_nodes.push(*right_node);
                        }
                    }
                }

                if after_start {
                    match inner.left_node {
                        Some(left_node) => self.delayed_nodes.push(*left_node),
                        None => {
                            let left_node = self
                                .node_db
                                .get_node(&inner.left_hash)
                                .expect("node db should contain all nodes");

                            //self.cached_nodes.push(left_node);
                            self.delayed_nodes.push(*left_node);
                        }
                    }

                    //self.delayed_nodes.push(inner.get_left_node(self.node_db));
                }
            }
            Node::Leaf(leaf) => {
                if after_start && before_end {
                    // we have a leaf node within the range
                    return Some((leaf.key, leaf.value));
                }
            }
        }

        self.traverse()
    }
}

impl<'a, DB: Database, R: RangeBounds<RB>, RB: AsRef<[u8]>> Iterator for Range<'a, DB, RB, R> {
    type Item = (Vec<u8>, Vec<u8>);

    fn next(&mut self) -> Option<Self::Item> {
        self.traverse()
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

        let full_range = tree.range::<_, Vec<u8>>(..).collect::<Vec<_>>();

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
            .collect::<Vec<_>>();

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
            .collect::<Vec<_>>();

        let expected_range = expected_array.into_iter().take(4).collect::<Vec<_>>();

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
            .collect::<Vec<_>>();

        let expected_range = expected_array.into_iter().take(5).collect::<Vec<_>>();

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
            .collect::<Vec<_>>();

        assert_eq!(expected_array, result_range)
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
            .collect::<Vec<_>>();

        let expected_range = expected_array
            .into_iter()
            .skip(4)
            .take(4)
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
            .collect::<Vec<_>>();

        let expected_range = expected_array
            .into_iter()
            .skip(5)
            .take(4)
            .collect::<Vec<_>>();

        assert_eq!(expected_range, result_range)
    }
}
