use std::{
    collections::BTreeMap,
    ops::Bound,
    sync::{Arc, RwLock},
};

use crate::Database;

/// Database which stores data in memory
#[derive(Debug, Clone)]
pub struct MemDB {
    store: Arc<RwLock<BTreeMap<Vec<u8>, Vec<u8>>>>,
}

impl Default for MemDB {
    fn default() -> Self {
        Self::new()
    }
}

impl MemDB {
    /// Create new `Self`
    pub fn new() -> MemDB {
        MemDB {
            store: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }
}

impl Database for MemDB {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.store.read().expect("poisoned lock").get(key).cloned()
    }

    fn put(&self, key: Vec<u8>, value: Vec<u8>) {
        self.store
            .write()
            .expect("poisoned lock")
            .insert(key, value);
    }

    fn iterator(&self) -> impl Iterator<Item = (Box<[u8]>, Box<[u8]>)> + '_ {
        self.store
            .read()
            .expect("poisoned lock")
            .clone()
            .into_iter()
            .map(|(key, value)| (key.into_boxed_slice(), value.into_boxed_slice()))
    }

    fn prefix_iterator(
        &self,
        prefix: Vec<u8>,
    ) -> impl Iterator<Item = (Box<[u8]>, Box<[u8]>)> + '_ {
        let start = Bound::Included(prefix.clone());
        let end = prefix_end_bound(prefix);

        let mut pairs = Vec::new();

        for (k, v) in self
            .store
            .read()
            .expect("poisoned lock")
            .range((start, end))
        {
            //println!("Found: {}: {}", k, v);
            let pair = (k.clone().into_boxed_slice(), v.clone().into_boxed_slice());
            pairs.push(pair)
        }

        pairs.into_iter()
    }
}

/// Returns the Bound on a range query for a given prefix
///
/// That is the smallest x such that, prefix + y < x for all y. If
/// no such x exists (i.e. prefix = vec![255; N]; for some N) it returns Bound::Unbounded
fn prefix_end_bound(mut prefix: Vec<u8>) -> Bound<Vec<u8>> {
    loop {
        let last = prefix.last_mut();

        match last {
            None => return Bound::Unbounded,
            Some(last) => {
                if *last != 255 {
                    *last += 1;
                    return Bound::Excluded(prefix);
                }
                prefix.pop();
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn iterator_works() {
        let db = MemDB::new();
        db.put(vec![1], vec![1]);
        db.put(vec![2], vec![2]);
        let got_pairs: Vec<(Box<[u8]>, Box<[u8]>)> = db.iterator().collect();

        let expected_pairs: Vec<(Box<[u8]>, Box<[u8]>)> = vec![
            (vec![1].into_boxed_slice(), vec![1].into_boxed_slice()),
            (vec![2].into_boxed_slice(), vec![2].into_boxed_slice()),
        ];

        assert_eq!(expected_pairs.len(), got_pairs.len());
        assert!(got_pairs.iter().all(|e| { expected_pairs.contains(e) }));
    }

    #[test]
    fn prefix_iterator_works() {
        let db = MemDB::new();
        db.put(vec![1, 1], vec![1]);
        db.put(vec![2, 1], vec![2]);
        db.put(vec![3, 1], vec![3]);
        db.put(vec![4, 1], vec![4]);

        let got_pairs: Vec<(Box<[u8]>, Box<[u8]>)> = db.prefix_iterator(vec![2]).collect();

        println!("got pairs: {:?}", got_pairs);

        let expected_pairs: Vec<(Box<[u8]>, Box<[u8]>)> =
            vec![(vec![2, 1].into_boxed_slice(), vec![2].into_boxed_slice())];

        assert_eq!(expected_pairs.len(), got_pairs.len());
        assert!(got_pairs.iter().all(|e| { expected_pairs.contains(e) }));
    }
}
