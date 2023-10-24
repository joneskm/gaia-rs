use crate::types::context::context::Context;
use database::{Database, PrefixDB};
use store_crate::{KVStore, MultiStore, StoreKey};
use tendermint_informal::abci::Event;

pub struct InitContext<'a, T: Database, SK: StoreKey> {
    pub multi_store: &'a mut MultiStore<T, SK>,
    pub height: u64,
    pub events: Vec<Event>,
    pub chain_id: String,
}

impl<'a, T: Database, SK: StoreKey> InitContext<'a, T, SK> {
    pub fn new(multi_store: &'a mut MultiStore<T, SK>, height: u64, chain_id: String) -> Self {
        InitContext {
            multi_store,
            height,
            events: vec![],
            chain_id,
        }
    }

    pub fn as_any<'b>(&'b mut self) -> Context<'b, 'a, T, SK> {
        Context::InitContext(self)
    }

    ///  Fetches an immutable ref to a KVStore from the MultiStore.
    pub fn get_kv_store(&self, store_key: &SK) -> &KVStore<PrefixDB<T>> {
        return self.multi_store.get_kv_store(store_key);
    }

    /// Fetches a mutable ref to a KVStore from the MultiStore.
    pub fn get_mutable_kv_store(&mut self, store_key: &SK) -> &mut KVStore<PrefixDB<T>> {
        return self.multi_store.get_mutable_kv_store(store_key);
    }

    pub fn get_height(&self) -> u64 {
        self.height
    }

    pub fn push_event(&mut self, event: Event) {
        self.events.push(event);
    }

    pub fn append_events(&mut self, mut events: Vec<Event>) {
        self.events.append(&mut events);
    }
}
