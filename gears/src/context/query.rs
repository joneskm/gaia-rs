use database::prefix::PrefixDB;
use database::Database;

use kv_store::{
    error::KVStoreError, query::QueryMultiStore, store::kv::immutable::KVStore, StoreKey,
};
use tendermint::types::chain_id::ChainId;

use crate::types::store::kv::Store;

use super::{InfallibleContext, QueryableContext};

#[derive(Debug)]
pub struct QueryContext<DB, SK> {
    multi_store: QueryMultiStore<DB, SK>,
    pub(crate) height: u32,
    pub(crate) chain_id: ChainId,
}

impl<DB: Database, SK: StoreKey> QueryContext<DB, SK> {
    pub fn new(
        multi_store: QueryMultiStore<DB, SK>,
        version: u32,
        // chain_id: ChainId,
    ) -> Result<Self, KVStoreError> {
        Ok(QueryContext {
            multi_store,
            height: version,
            chain_id: ChainId::new("todo-900").expect("default should be valid"),
        })
    }
}

impl<DB: Database, SK: StoreKey> QueryContext<DB, SK> {
    pub fn chain_id(&self) -> &ChainId {
        &self.chain_id
    }

    pub fn kv_store(&self, store_key: &SK) -> KVStore<'_, PrefixDB<DB>> {
        self.multi_store.kv_store(store_key)
    }
}

impl<DB: Database, SK: StoreKey> QueryableContext<DB, SK> for QueryContext<DB, SK> {
    fn height(&self) -> u32 {
        self.height
    }

    fn kv_store(&self, store_key: &SK) -> Store<'_, PrefixDB<DB>> {
        Store::from(self.kv_store(store_key))
    }

    fn chain_id(&self) -> &ChainId {
        &self.chain_id
    }
}

impl<DB: Database, SK: StoreKey> InfallibleContext<DB, SK> for QueryContext<DB, SK> {
    fn infallible_store(&self, store_key: &SK) -> KVStore<'_, PrefixDB<DB>> {
        self.kv_store(store_key)
    }
}
