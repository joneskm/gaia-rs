use database::{prefix::PrefixDB, Database};
use store_crate::{
    types::{
        kv::{immutable::KVStore, mutable::KVStoreMut},
        multi::{immutable::MultiStore, mutable::MultiStoreMut, MultiBank},
    },
    CommitKind, StoreKey,
};
use tendermint::types::{chain_id::ChainId, proto::event::Event};

use crate::types::header::Header;

use super::{QueryableContext, TransactionalContext};

#[derive(Debug)]
pub struct BlockContext<'a, DB, SK> {
    multi_store: &'a mut MultiBank<DB, SK, CommitKind>,
    pub(crate) height: u64,
    pub(crate) chain_id: ChainId,
    pub header: Header, // TODO:NOW pub or pub(crate)?
    pub events: Vec<Event>,
}

impl<'a, DB, SK> BlockContext<'a, DB, SK> {
    pub fn new(
        multi_store: &'a mut MultiBank<DB, SK, CommitKind>,
        height: u64,
        chain_id: ChainId,
        header: Header,
    ) -> Self {
        BlockContext {
            multi_store,
            height,
            events: Vec::new(),
            chain_id,
            header,
        }
    }
}

impl<DB: Database, SK: StoreKey> QueryableContext<DB, SK> for BlockContext<'_, DB, SK> {
    fn kv_store(&self, store_key: &SK) -> KVStore<'_, PrefixDB<DB>> {
        self.multi_store.kv_store(store_key).into()
    }

    fn multi_store(&self) -> MultiStore<'_, DB, SK> {
        MultiStore::from(&*self.multi_store)
    }

    fn height(&self) -> u64 {
        self.height
    }

    fn chain_id(&self) -> &ChainId {
        &self.chain_id
    }
}

impl<DB: Database, SK: StoreKey> TransactionalContext<DB, SK> for BlockContext<'_, DB, SK> {
    fn multi_store_mut(&mut self) -> MultiStoreMut<'_, DB, SK> {
        MultiStoreMut::from(&mut *self.multi_store)
    }

    fn kv_store_mut(&mut self, store_key: &SK) -> KVStoreMut<'_, PrefixDB<DB>> {
        self.multi_store.kv_store_mut(store_key).into()
    }

    fn push_event(&mut self, event: Event) {
        self.events.push(event);
    }

    fn append_events(&mut self, mut events: Vec<Event>) {
        self.events.append(&mut events);
    }

    fn events_drain(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }
}
