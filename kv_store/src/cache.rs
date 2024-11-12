use std::collections::{BTreeMap, HashSet};

/// Storage for store cache
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KVCache {
    pub(crate) storage: BTreeMap<Vec<u8>, Vec<u8>>,
    pub(crate) delete: HashSet<Vec<u8>>,
}

pub type KVCacheCollection = (BTreeMap<Vec<u8>, Vec<u8>>, HashSet<Vec<u8>>);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("Key deleted from cache")]
pub struct DeletedError;

impl KVCache {
    /// Take out all cache from storages.
    pub fn take(&mut self) -> KVCacheCollection {
        (
            std::mem::take(&mut self.storage),
            std::mem::take(&mut self.delete),
        )
    }

    /// Get value from cache
    pub fn get<R: AsRef<[u8]> + ?Sized>(&self, k: &R) -> Result<Option<&Vec<u8>>, DeletedError> {
        if self.delete.contains(k.as_ref()) {
            return Err(DeletedError);
        }

        Ok(self.storage.get(k.as_ref()))
    }

    pub fn delete(&mut self, k: &[u8]) -> Option<Vec<u8>> {
        let _ = self.delete.insert(k.to_owned());
        self.storage.remove(k)
    }

    pub fn set<KI: IntoIterator<Item = u8>, VI: IntoIterator<Item = u8>>(
        &mut self,
        key: KI,
        value: VI,
    ) {
        let key: Vec<u8> = key.into_iter().collect();

        let _ = self.delete.remove(&key);
        self.storage.insert(key, value.into_iter().collect());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delete_removes_insert() {
        let mut cache = KVCache::default();

        cache.storage.insert(vec![1], vec![2]);
        cache.storage.insert(vec![3], vec![4]);

        // ---
        cache.delete([1].as_slice());

        // ---
        assert!(!cache.storage.contains_key([1].as_slice()));
        assert!(cache.storage.contains_key([3].as_slice()));
        assert!(cache.delete.contains([1].as_slice()));
    }

    #[test]
    fn set_removes_delete() {
        let mut cache = KVCache::default();

        cache.delete.insert(vec![1]);

        // ---
        cache.set(vec![1], vec![2]);

        // ---
        assert!(cache.storage.contains_key([1].as_slice()));
        assert!(!cache.delete.contains([1].as_slice()));
    }

    #[test]
    fn set_overrides() {
        let mut cache = KVCache::default();

        cache.storage.insert(vec![1], vec![2]);

        // ---
        cache.set(vec![1], vec![3]);

        // ---
        assert!(cache.storage.contains_key([1].as_slice()));
        assert_eq!(Some(&vec![3]), cache.storage.get([1].as_slice()));
    }

    #[test]
    fn deleted_not_gets() {
        let mut cache = KVCache::default();

        cache.storage.insert(vec![1], vec![2]);
        cache.delete.insert(vec![1]);

        // ---

        let result = cache.get(&[1]);

        //
        assert_eq!(Err(DeletedError), result);
    }
}
