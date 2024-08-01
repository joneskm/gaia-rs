use super::*;
use crate::{
    keys::{
        delegator_starting_info_key, delegator_withdraw_addr_key,
        validator_accumulated_commission_key, validator_current_rewards_key,
        validator_historical_rewards_key, validator_outstanding_rewards_key,
        validator_slash_event_key,
    },
    types::FeePool,
    ByteValue, DelegatorStartingInfo, ValidatorAccumulatedCommission, ValidatorCurrentRewards,
    ValidatorHistoricalRewards, ValidatorOutstandingRewards, ValidatorSlashEvent, FEE_POOL_KEY,
    PROPOSER_KEY,
};
use gears::{
    context::{InfallibleContext, InfallibleContextMut, QueryableContext},
    core::Protobuf,
    store::database::ext::UnwrapCorrupt,
    types::address::{AccAddress, ValAddress},
};

impl<
        SK: StoreKey,
        PSK: ParamsSubspaceKey,
        AK: AuthKeeper<SK, M>,
        BK: BankKeeper<SK, M>,
        DSK: DistributionStakingKeeper<SK, M>,
        M: Module,
    > Keeper<SK, PSK, AK, BK, DSK, M>
{
    /// set the global fee pool distribution info
    pub fn set_fee_pool<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        fee_pool: &FeePool,
    ) -> Result<(), GasStoreErrors> {
        let mut store = ctx.kv_store_mut(&self.store_key);
        store.set(FEE_POOL_KEY, fee_pool.encode_vec())
    }

    /// get the global fee pool distribution info
    pub fn fee_pool<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
    ) -> Result<Option<FeePool>, GasStoreErrors> {
        let store = ctx.kv_store(&self.store_key);
        Ok(store
            .get(&FEE_POOL_KEY)?
            .map(|bytes| FeePool::decode_vec(&bytes).unwrap_or_corrupt()))
    }

    /// get the delegator withdraw address
    pub fn delegator_withdraw_addr<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
        delegator_address: &AccAddress,
    ) -> Result<Option<AccAddress>, GasStoreErrors> {
        let store = ctx.kv_store(&self.store_key);
        Ok(store
            .get(&delegator_withdraw_addr_key(delegator_address.clone()))?
            .map(|bytes| AccAddress::try_from(bytes).unwrap_or_corrupt()))
    }

    /// set the delegator withdraw address
    pub fn set_delegator_withdraw_addr<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        delegator_address: &AccAddress,
        withdraw_address: &AccAddress,
    ) -> Result<(), GasStoreErrors> {
        let mut store = ctx.kv_store_mut(&self.store_key);
        store.set(
            delegator_withdraw_addr_key(delegator_address.clone()),
            Vec::from(withdraw_address.clone()),
        )
    }

    /// previous_proposer_cons_addr returns the proposer consensus address for the
    /// current block.
    pub fn previous_proposer_cons_addr<DB: Database, CTX: InfallibleContext<DB, SK>>(
        &self,
        ctx: &CTX,
    ) -> Option<ConsAddress> {
        let store = ctx.infallible_store(&self.store_key);
        store.get(&PROPOSER_KEY).map(|bytes| {
            ConsAddress::try_from(ByteValue::decode_vec(&bytes).unwrap_or_corrupt().value)
                .unwrap_or_corrupt()
        })
    }

    /// set the proposer public key for this block
    pub fn set_previous_proposer_cons_addr<DB: Database, CTX: InfallibleContextMut<DB, SK>>(
        &self,
        ctx: &mut CTX,
        address: &ConsAddress,
    ) {
        let mut store = ctx.infallible_store_mut(&self.store_key);
        let byte_value = ByteValue {
            value: address.clone().into(),
        };
        store.set(PROPOSER_KEY, byte_value.encode_vec());
    }

    /// get validator outstanding rewards
    pub fn validator_outstanding_rewards<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
        address: &ValAddress,
    ) -> Result<Option<ValidatorOutstandingRewards>, GasStoreErrors> {
        let store = ctx.kv_store(&self.store_key);
        Ok(store
            .get(&validator_outstanding_rewards_key(address.clone()))?
            .map(|bytes| ValidatorOutstandingRewards::decode_vec(&bytes).unwrap_or_corrupt()))
    }

    /// set validator outstanding rewards
    pub fn set_validator_outstanding_rewards<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        address: &ValAddress,
        outstanding_rewards: &ValidatorOutstandingRewards,
    ) -> Result<(), GasStoreErrors> {
        let mut store = ctx.kv_store_mut(&self.store_key);
        store.set(
            validator_outstanding_rewards_key(address.clone()),
            outstanding_rewards.encode_vec(),
        )
    }

    /// get accumulated commission for a validator
    pub fn validator_accumulated_commission<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
        address: &ValAddress,
    ) -> Result<Option<ValidatorAccumulatedCommission>, GasStoreErrors> {
        let store = ctx.kv_store(&self.store_key);
        Ok(store
            .get(&validator_accumulated_commission_key(address.clone()))?
            .map(|bytes| ValidatorAccumulatedCommission::decode_vec(&bytes).unwrap_or_corrupt()))
    }

    /// set accumulated commission for a validator
    pub fn set_validator_accumulated_commission<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        address: &ValAddress,
        accumulated_commission: &ValidatorAccumulatedCommission,
    ) -> Result<(), GasStoreErrors> {
        let mut store = ctx.kv_store_mut(&self.store_key);
        // TODO: we have only valid struct DecimalCoins without zeros
        //     if commission.Commission.IsZero() {
        //         bz = k.cdc.MustMarshal(&types.ValidatorAccumulatedCommission{})
        //     } else {
        //         bz = k.cdc.MustMarshal(&commission)
        //     }
        store.set(
            validator_accumulated_commission_key(address.clone()),
            accumulated_commission.encode_vec(),
        )
    }

    /// get historical rewards for a particular period
    pub fn validator_historical_rewards<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
        address: &ValAddress,
        power: u64,
    ) -> Result<Option<ValidatorHistoricalRewards>, GasStoreErrors> {
        let store = ctx.kv_store(&self.store_key);
        Ok(store
            .get(&validator_historical_rewards_key(address.clone(), power))?
            .map(|bytes| ValidatorHistoricalRewards::decode_vec(&bytes).unwrap_or_corrupt()))
    }

    /// set historical rewards for a particular period
    pub fn set_validator_historical_rewards<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        address: &ValAddress,
        power: u64,
        rewards: &ValidatorHistoricalRewards,
    ) -> Result<(), GasStoreErrors> {
        let mut store = ctx.kv_store_mut(&self.store_key);
        store.set(
            validator_historical_rewards_key(address.clone(), power),
            rewards.encode_vec(),
        )
    }

    /// delete historical rewards for a particular period
    pub fn delete_validator_historical_rewards<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        address: &ValAddress,
        power: u64,
    ) -> Result<Option<Vec<u8>>, GasStoreErrors> {
        let mut store = ctx.kv_store_mut(&self.store_key);
        store.delete(&validator_historical_rewards_key(address.clone(), power))
    }

    /// get current rewards for a validator
    pub fn validator_current_rewards<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
        address: &ValAddress,
    ) -> Result<Option<ValidatorCurrentRewards>, GasStoreErrors> {
        let store = ctx.kv_store(&self.store_key);
        Ok(store
            .get(&validator_current_rewards_key(address.clone()))?
            .map(|bytes| ValidatorCurrentRewards::decode_vec(&bytes).unwrap_or_corrupt()))
    }

    /// set current rewards for a validator
    pub fn set_validator_current_rewards<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        address: &ValAddress,
        rewards: &ValidatorCurrentRewards,
    ) -> Result<(), GasStoreErrors> {
        let mut store = ctx.kv_store_mut(&self.store_key);
        store.set(
            validator_current_rewards_key(address.clone()),
            rewards.encode_vec(),
        )
    }

    /// get the starting info associated with a delegator
    pub fn delegator_starting_info<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
        validator_address: &ValAddress,
        delegator_address: &AccAddress,
    ) -> Result<Option<DelegatorStartingInfo>, GasStoreErrors> {
        let store = ctx.kv_store(&self.store_key);
        Ok(store
            .get(&delegator_starting_info_key(
                validator_address.clone(),
                delegator_address.clone(),
            ))?
            .map(|bytes| DelegatorStartingInfo::decode_vec(&bytes).unwrap_or_corrupt()))
    }

    /// check existence of the starting info associated with a delegator
    pub fn has_delegator_starting_info<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
        validator_address: &ValAddress,
        delegator_address: &AccAddress,
    ) -> Result<bool, GasStoreErrors> {
        let store = ctx.kv_store(&self.store_key);
        Ok(store
            .get(&delegator_starting_info_key(
                validator_address.clone(),
                delegator_address.clone(),
            ))?
            .is_some())
    }

    /// set the starting info associated with a delegator
    pub fn set_delegator_starting_info<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        validator_address: &ValAddress,
        delegator_address: &AccAddress,
        info: &DelegatorStartingInfo,
    ) -> Result<(), GasStoreErrors> {
        let mut store = ctx.kv_store_mut(&self.store_key);
        store.set(
            delegator_starting_info_key(validator_address.clone(), delegator_address.clone()),
            info.encode_vec(),
        )
    }

    /// delete the starting info associated with a delegator
    pub fn delete_delegator_starting_info<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        validator_address: &ValAddress,
        delegator_address: &AccAddress,
    ) -> Result<Option<Vec<u8>>, GasStoreErrors> {
        let mut store = ctx.kv_store_mut(&self.store_key);
        store.delete(&delegator_starting_info_key(
            validator_address.clone(),
            delegator_address.clone(),
        ))
    }

    /// set slash event for height
    pub fn set_validator_slash_event<DB: Database, CTX: InfallibleContextMut<DB, SK>>(
        &self,
        ctx: &mut CTX,
        validator_address: &ValAddress,
        height: u64,
        period: u64,
        slash_event: &ValidatorSlashEvent,
    ) {
        let mut store = ctx.infallible_store_mut(&self.store_key);
        store.set(
            validator_slash_event_key(validator_address.clone(), height, period),
            slash_event.encode_vec(),
        )
    }
}
