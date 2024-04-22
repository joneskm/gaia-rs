#![warn(rust_2018_idioms)]

use strum::IntoEnumIterator;
use trees::iavl::Range;
use types::prefix::{immutable::ImmutablePrefixStore, mutable::MutablePrefixStore};

pub mod error;
mod hash;
pub mod types;
mod utils;

pub mod database {
    pub use database::*;
}

use std::{hash::Hash, ops::RangeBounds};

pub(crate) const TREE_CACHE_SIZE: usize = 100_000;

pub trait StoreKey: Hash + Eq + IntoEnumIterator + Clone + Send + Sync + 'static {
    fn name(&self) -> &'static str;
}

pub trait ReadPrefixStore {
    fn get<T: AsRef<[u8]> + ?Sized>(&self, k: &T) -> Option<Vec<u8>>;
}

pub trait WritePrefixStore: ReadPrefixStore {
    fn set<KI: IntoIterator<Item = u8>, VI: IntoIterator<Item = u8>>(&mut self, k: KI, v: VI);
}

pub trait QueryableKVStore<DB> {
    fn get<R: AsRef<[u8]> + ?Sized>(&self, k: &R) -> Option<Vec<u8>>;
    fn prefix_store<I: IntoIterator<Item = u8>>(&self, prefix: I) -> ImmutablePrefixStore<'_, DB>;
    fn range<R: RangeBounds<Vec<u8>> + Clone>(&self, range: R) -> Range<'_, R, DB>;
    // fn get_keys(&self, key_prefix: &(impl AsRef<[u8]> + ?Sized)) -> Vec<Vec<u8>>;
}

pub trait TransactionalKVStore<DB>: QueryableKVStore<DB> {
    fn prefix_store_mut(
        &mut self,
        prefix: impl IntoIterator<Item = u8>,
    ) -> MutablePrefixStore<'_, DB>;
    fn set<KI: IntoIterator<Item = u8>, VI: IntoIterator<Item = u8>>(&mut self, key: KI, value: VI);
    fn commit(&mut self) -> [u8; 32];
}

pub trait ReadMultiKVStore<DB, SK> {
    type KvStore: QueryableKVStore<DB>;

    fn kv_store(&self, store_key: &SK) -> &Self::KvStore;
    fn head_version(&self) -> u32;
    fn head_commit_hash(&self) -> [u8; 32];
}

pub trait WriteMultiKVStore<DB, SK> {
    type KvStoreMut: TransactionalKVStore<DB>;

    fn kv_store_mut(&mut self, store_key: &SK) -> &mut Self::KvStoreMut;
    fn commit(&mut self) -> [u8; 32];
    /// Writes then clears each store's tx cache to the store's block cache then clears the tx caches
    fn tx_caches_write_then_clear(&mut self);
    /// Clears the tx caches
    fn tx_caches_clear(&mut self);
}
