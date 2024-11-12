use database::Database;
use kv_store::bank::multi::ApplicationMultiBank;

use crate::application::handlers::node::ABCIHandler;

use gas::metering::{basic_meter::BasicGasMeter, infinite_meter::InfiniteGasMeter, Gas, GasMeter};

use super::mode::{check::CheckTxMode, deliver::DeliverTxMode};

/// Structure to hide state logic from application
/// and sync state between different modes during blocks
#[derive(Debug)]
pub struct ApplicationState<DB, AH: ABCIHandler> {
    pub(super) check_mode: CheckTxMode<DB, AH>,
    pub(super) deliver_mode: DeliverTxMode<DB, AH>,
    pub head_hash: [u8; 32],
    pub last_height: u32,
}

impl<DB: Database, AH: ABCIHandler> ApplicationState<DB, AH> {
    pub fn new(max_gas: Gas, multi_store: &ApplicationMultiBank<DB, AH::StoreKey>) -> Self {
        Self {
            check_mode: CheckTxMode::new(max_gas, multi_store.to_tx_kind()),
            deliver_mode: DeliverTxMode::new(max_gas, multi_store.to_tx_kind()),
            head_hash: multi_store.head_commit_hash(),
            last_height: multi_store.head_version(),
        }
    }

    /// Update gas meter for block metering. This method should be called in each `begin_block`
    /// to update meter according to [crate::baseapp::params::BlockParams]
    pub fn replace_meter(&mut self, max_gas: Gas) {
        match max_gas {
            Gas::Infinite => {
                self.check_mode.block_gas_meter = GasMeter::new(Box::<InfiniteGasMeter>::default());
                self.deliver_mode.block_gas_meter =
                    GasMeter::new(Box::<InfiniteGasMeter>::default());
            }
            Gas::Finite(max_gas) => {
                self.check_mode.block_gas_meter =
                    GasMeter::new(Box::new(BasicGasMeter::new(max_gas)));
                self.deliver_mode.block_gas_meter =
                    GasMeter::new(Box::new(BasicGasMeter::new(max_gas)));
            }
        }
    }

    pub fn append_block_cache(&mut self, multi_store: &mut ApplicationMultiBank<DB, AH::StoreKey>) {
        self.check_mode.multi_store.append_block_cache(multi_store);
        self.deliver_mode
            .multi_store
            .append_block_cache(multi_store);
    }

    pub fn take_block_cache(&mut self, multi_store: &mut ApplicationMultiBank<DB, AH::StoreKey>) {
        let list = self.deliver_mode.multi_store.take_block_cache();

        for (key, (insert_list, delete_list)) in list {
            let kv_store = multi_store.kv_store_mut(&key);

            delete_list.into_iter().for_each(|this| {
                kv_store.delete(&this);
            });
            insert_list
                .into_iter()
                .for_each(|(key, value)| kv_store.set(key, value));
        }
    }

    /// Commit changes from state store to application and persist changes to disk.
    /// Returns application state hash.
    ///
    /// **Note**: changes from `check_tx` state is discarded and instead `deliver_tx` state used.
    pub fn commit(&mut self, multi_store: &mut ApplicationMultiBank<DB, AH::StoreKey>) -> [u8; 32] {
        self.check_mode.multi_store.tx_cache_clear();
        self.check_mode.multi_store.block_cache_clear();

        self.deliver_mode.multi_store.tx_cache_clear();
        multi_store.consume_block_cache(&mut self.deliver_mode.multi_store);

        let hash = multi_store.commit();

        self.head_hash = hash;
        self.last_height = multi_store.head_version();

        hash
    }
}
