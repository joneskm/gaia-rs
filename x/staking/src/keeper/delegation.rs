use super::*;
use gears::{
    store::database::ext::UnwrapCorrupt, types::store::gas::errors::GasStoreErrors,
    x::types::validator::BondStatus,
};

impl<
        SK: StoreKey,
        PSK: ParamsSubspaceKey,
        AK: AuthKeeper<SK, M>,
        BK: BankKeeper<SK, M>,
        KH: KeeperHooks<SK, AK, M>,
        M: Module,
    > Keeper<SK, PSK, AK, BK, KH, M>
{
    /// Delegate performs a delegation, set/update everything necessary within the store.
    /// token_src indicates the bond status of the incoming funds.
    pub fn delegate<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        del_addr: &AccAddress,
        bond_amount: Uint256,
        token_src: BondStatus,
        validator: &mut Validator,
        subtract_account: bool,
    ) -> Result<Decimal256, AppError> {
        // In some situations, the exchange rate becomes invalid, e.g. if
        // Validator loses all tokens due to slashing. In this case,
        // make all future delegations invalid.
        if validator.invalid_ex_rate() {
            return Err(AppError::Custom(
                "invalid delegation_share exchange rate ".into(),
            ));
        }

        // Get or create the delegation object
        let mut delegation = if let Some(delegation) =
            self.delegation(ctx, del_addr, &validator.operator_address)?
        {
            self.before_delegation_shares_modified(ctx, del_addr, &validator.operator_address);
            delegation
        } else {
            self.before_delegation_created(ctx, del_addr, &validator.operator_address);
            Delegation {
                delegator_address: del_addr.clone(),
                validator_address: validator.operator_address.clone(),
                shares: Decimal256::zero(),
            }
        };

        // if subtract_account is true then we are
        // performing a delegation and not a redelegation, thus the source tokens are
        // all non bonded
        if subtract_account {
            if token_src == BondStatus::Bonded {
                return Err(AppError::Custom(
                    "delegation token source cannot be bonded".to_string(),
                ));
            }

            let send_module = match validator.status {
                BondStatus::Bonded => &self.bonded_module,
                BondStatus::Unbonded | BondStatus::Unbonding => &self.not_bonded_module,
            };

            let denom = self
                .staking_params_keeper
                .try_get(ctx)?
                .bond_denom()
                .clone();
            let coins = SendCoins::new(vec![Coin {
                denom,
                amount: bond_amount,
            }])
            .map_err(|e| AppError::Coins(e.to_string()))?;

            self.bank_keeper
                .delegate_coins_from_account_to_module::<DB, CTX>(
                    ctx,
                    delegation.delegator_address.clone(),
                    send_module,
                    coins,
                )?;
        } else {
            // potentially transfer tokens between pools, if

            match (token_src, validator.status == BondStatus::Bonded) {
                (BondStatus::Unbonded | BondStatus::Unbonding, true) => {
                    // transfer pools
                    self.not_bonded_tokens_to_bonded(ctx, bond_amount)?;
                }
                (BondStatus::Bonded, false) => {
                    // transfer pools
                    self.bonded_tokens_to_not_bonded(ctx, bond_amount)?;
                }
                (BondStatus::Bonded, true)
                | (BondStatus::Unbonded | BondStatus::Unbonding, false) => {}
            }
        }

        let new_shares = self
            .add_validator_tokens_and_shares(ctx, validator, bond_amount)
            .map_err(|e| AppError::Custom(e.to_string()))?;
        // Update delegation
        delegation.shares = delegation
            .shares
            .checked_add(new_shares)
            .map_err(|e| AppError::Custom(e.to_string()))?;
        self.set_delegation(ctx, &delegation)?;

        // Call the after-modification hook
        self.after_delegation_modified(
            ctx,
            &delegation.delegator_address,
            &delegation.validator_address,
        );

        Ok(new_shares)
    }

    pub fn delegation<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
        del_addr: &AccAddress,
        val_addr: &ValAddress,
    ) -> Result<Option<Delegation>, GasStoreErrors> {
        let store = QueryableContext::kv_store(ctx, &self.store_key);
        let delegations_store = store.prefix_store(DELEGATION_KEY);
        let mut key = Vec::from(del_addr.clone());
        key.extend_from_slice(&Vec::from(val_addr.clone()));
        Ok(delegations_store
            .get(&key)?
            .map(|bytes| serde_json::from_slice(&bytes).unwrap_or_corrupt()))
    }

    pub fn set_delegation<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        delegation: &Delegation,
    ) -> Result<(), GasStoreErrors> {
        let store = TransactionalContext::kv_store_mut(ctx, &self.store_key);
        let mut delegations_store = store.prefix_store_mut(DELEGATION_KEY);
        let mut key = Vec::from(delegation.delegator_address.clone());
        key.extend_from_slice(&Vec::from(delegation.validator_address.clone()));
        delegations_store.set(
            key,
            serde_json::to_vec(&delegation).expect(SERDE_ENCODING_DOMAIN_TYPE),
        )?;

        Ok(())
    }

    pub fn remove_delegation<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        delegation: &Delegation,
    ) -> Result<Option<Vec<u8>>, GasStoreErrors> {
        let store = ctx.kv_store_mut(&self.store_key);
        let mut delegations_store = store.prefix_store_mut(DELEGATION_KEY);
        let mut key = Vec::from(delegation.delegator_address.clone());
        key.extend_from_slice(&Vec::from(delegation.validator_address.clone()));
        delegations_store.delete(&key)
    }
}
