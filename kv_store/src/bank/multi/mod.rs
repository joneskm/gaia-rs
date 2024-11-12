//! Implementation of multi bank structures which stores all stores and ensure that no overlap with all keys

use std::{collections::HashMap, marker::PhantomData};

use application::ApplicationStore;
use database::Database;
use transaction::TransactionStore;

use crate::{error::KEY_EXISTS_MSG, StoreKey};

pub mod application;
pub mod transaction;

/// Backend for multi stores
pub trait MultiBankBackend<DB, SK> {
    /// Type of bank to return
    type Bank;

    /// Return all stores
    fn stores(&self) -> &HashMap<SK, Self::Bank>;
    /// Return all stores
    fn stores_mut(&mut self) -> &mut HashMap<SK, Self::Bank>;
}

/// Multistore for application layer
pub type ApplicationMultiBank<DB, SK> = MultiBank<DB, SK, ApplicationStore<DB, SK>>;
/// Multistore for transaction layer
pub type TransactionMultiBank<DB, SK> = MultiBank<DB, SK, TransactionStore<DB, SK>>;

/// Bank which stores all KVBanks
#[derive(Debug)]
pub struct MultiBank<DB, SK, SB> {
    pub(crate) head_version: u32,
    pub(crate) head_commit_hash: [u8; 32],
    pub(crate) backend: SB,
    _marker: PhantomData<(DB, SK)>,
}

impl<DB: Database, SK: StoreKey, SB: MultiBankBackend<DB, SK>> MultiBank<DB, SK, SB> {
    /// Return kv store for this store key
    pub fn kv_store(&self, store_key: &SK) -> &SB::Bank {
        self.backend.stores().get(store_key).expect(KEY_EXISTS_MSG)
    }

    /// Return mutable kv store for this store key
    pub fn kv_store_mut(&mut self, store_key: &SK) -> &mut SB::Bank {
        self.backend
            .stores_mut()
            .get_mut(store_key)
            .expect(KEY_EXISTS_MSG)
    }

    /// Return version of head
    pub fn head_version(&self) -> u32 {
        self.head_version
    }

    /// Return hash of head
    pub fn head_commit_hash(&self) -> [u8; 32] {
        self.head_commit_hash
    }
}
