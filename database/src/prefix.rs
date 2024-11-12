//! Prefixed database

use std::sync::Arc;

use crate::Database;

/// Struct to automatically add prefix to any key passed to db
#[derive(Debug, Clone)]
pub struct PrefixDB<T> {
    db: Arc<T>,
    prefix: Vec<u8>,
}

impl<T: Database> PrefixDB<T> {
    /// Create new `Self`
    pub fn new(db: Arc<T>, prefix: Vec<u8>) -> Self {
        PrefixDB { db, prefix }
    }
}
impl<T: Database> Database for PrefixDB<T> {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        let key = [&self.prefix, key].concat();
        self.db.get(&key)
    }

    fn put(&self, key: Vec<u8>, value: Vec<u8>) {
        let key = [self.prefix.clone(), key].concat();
        self.db.put(key, value)
    }

    fn iterator(&self) -> impl Iterator<Item = (Box<[u8]>, Box<[u8]>)> + '_ {
        let prefix_length = self.prefix.len();

        self.db
            .prefix_iterator(self.prefix.clone())
            .map(move |(k, v)| {
                let key = k[prefix_length..].to_vec();
                (key.into_boxed_slice(), v)
            })
    }

    fn prefix_iterator(
        &self,
        prefix: Vec<u8>,
    ) -> impl Iterator<Item = (Box<[u8]>, Box<[u8]>)> + '_ {
        let prefix = [self.prefix.clone(), prefix].concat();
        let prefix_length = prefix.len();

        self.db.prefix_iterator(prefix).map(move |(k, v)| {
            let key = k[prefix_length..].to_vec();
            (key.into_boxed_slice(), v)
        })
    }
}

#[cfg(test)]
mod tests {

    use crate::MemDB;

    use super::*;

    #[test]
    fn get_works() {
        let db = MemDB::new();
        db.put(vec![1, 1], vec![1]);
        db.put(vec![2, 1], vec![2]);
        let prefix_db = PrefixDB::new(Arc::new(db), vec![2]);

        assert!(prefix_db.get(&[1, 1]).is_none());
        assert_eq!(prefix_db.get(&[1]), Some(vec![2]));
    }

    #[test]
    fn put_works() {
        let db = MemDB::new();
        let prefix_db = PrefixDB::new(Arc::new(db), vec![2]);
        prefix_db.put(vec![2], vec![1, 2, 3]);

        assert_eq!(prefix_db.get(&[2]), Some(vec![1, 2, 3]));
    }

    #[test]
    fn iterator_works() {
        let db = MemDB::new();
        db.put(vec![1, 1], vec![1]);
        db.put(vec![2, 1], vec![2]);
        db.put(vec![3, 1], vec![3]);
        let prefix_db = PrefixDB::new(Arc::new(db), vec![2]);

        let got_pairs: Vec<(Box<[u8]>, Box<[u8]>)> = prefix_db.iterator().collect();

        let expected_pairs: Vec<(Box<[u8]>, Box<[u8]>)> =
            vec![(vec![1].into_boxed_slice(), vec![2].into_boxed_slice())];

        assert_eq!(expected_pairs.len(), got_pairs.len());
        assert!(got_pairs.iter().all(|e| { expected_pairs.contains(e) }));
    }

    #[test]
    fn prefix_iterator_works() {
        let db = MemDB::new();
        db.put(vec![1, 1], vec![1]);
        db.put(vec![2, 1], vec![2]);
        db.put(vec![2, 2, 3], vec![2]);
        db.put(vec![2, 2, 4], vec![6]);
        db.put(vec![2, 1], vec![2]);
        db.put(vec![3, 1], vec![3]);
        db.put(vec![4, 1], vec![4]);

        let prefix_db = PrefixDB::new(Arc::new(db), vec![2]);

        let got_pairs: Vec<(Box<[u8]>, Box<[u8]>)> = prefix_db.prefix_iterator(vec![2]).collect();

        let expected_pairs: Vec<(Box<[u8]>, Box<[u8]>)> = vec![
            (vec![3].into_boxed_slice(), vec![2].into_boxed_slice()),
            (vec![4].into_boxed_slice(), vec![6].into_boxed_slice()),
        ];

        assert_eq!(expected_pairs.len(), got_pairs.len());
        assert!(got_pairs.iter().all(|e| { expected_pairs.contains(e) }));
    }
}
