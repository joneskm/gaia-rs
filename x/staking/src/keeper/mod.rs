use crate::{
    consts::{error::SERDE_ENCODING_DOMAIN_TYPE, keeper::*},
    error::StakingGenesisError,
    Delegation, DvPair, DvvTriplet, GenesisState, LastValidatorPower, Pool, Redelegation,
    StakingParams, StakingParamsKeeper, UnbondingDelegation, Validator,
};
use anyhow::anyhow;
use gears::{
    application::keepers::params::ParamsKeeper,
    context::{
        block::BlockContext, init::InitContext, query::QueryContext, InfallibleContext,
        QueryableContext, TransactionalContext,
    },
    extensions::gas::GasResultExt,
    gas::store::errors::GasStoreErrors,
    params::ParamsSubspaceKey,
    store::{database::Database, StoreKey},
    tendermint::types::{
        proto::{
            event::{Event, EventAttribute},
            validator::ValidatorUpdate,
        },
        time::timestamp::Timestamp,
    },
    types::{
        address::{AccAddress, ValAddress},
        base::{coin::UnsignedCoin, coins::UnsignedCoins},
        decimal256::Decimal256,
        uint::Uint256,
    },
    x::{
        keepers::{
            auth::AuthKeeper,
            staking::{KeeperHooks, StakingBankKeeper},
        },
        module::Module,
        types::validator::BondStatus,
    },
};
use std::{cmp::Ordering, collections::HashMap};

// Each module contains methods of keeper with logic related to its name. It can be delegation and
// validator types.

mod bonded;
mod delegation;
mod gov;
mod historical_info;
mod hooks;
mod mock_hook_keeper;
mod query;
mod redelegation;
mod tx;
mod unbonded;
mod unbonding;
mod validator;
mod validators_and_total_power;

pub use mock_hook_keeper::*;

#[derive(Debug, Clone)]
pub struct Keeper<
    SK: StoreKey,
    PSK: ParamsSubspaceKey,
    AK: AuthKeeper<SK, M>,
    BK,
    KH: KeeperHooks<SK, AK, M>,
    M: Module,
> {
    store_key: SK,
    auth_keeper: AK,
    bank_keeper: BK,
    staking_params_keeper: StakingParamsKeeper<PSK>,
    hooks_keeper: Option<KH>,
    bonded_module: M,
    not_bonded_module: M,
}

impl<
        SK: StoreKey,
        PSK: ParamsSubspaceKey,
        AK: AuthKeeper<SK, M>,
        BK: StakingBankKeeper<SK, M>,
        KH: KeeperHooks<SK, AK, M>,
        M: Module,
    > Keeper<SK, PSK, AK, BK, KH, M>
{
    pub fn new(
        store_key: SK,
        params_subspace_key: PSK,
        auth_keeper: AK,
        bank_keeper: BK,
        hooks_keeper: Option<KH>,
        bonded_module: M,
        not_bonded_module: M,
    ) -> Self {
        let staking_params_keeper = StakingParamsKeeper {
            params_subspace_key,
        };

        Keeper {
            store_key,
            auth_keeper,
            bank_keeper,
            staking_params_keeper,
            hooks_keeper,
            bonded_module,
            not_bonded_module,
        }
    }

    pub fn init_genesis<DB: Database>(
        &self,
        ctx: &mut InitContext<'_, DB, SK>,
        genesis: GenesisState,
    ) -> Result<Vec<ValidatorUpdate>, StakingGenesisError> {
        let mut bonded_tokens = Uint256::zero();
        let mut not_bonded_tokens = Uint256::zero();

        // TODO
        // ctx = ctx.WithBlockHeight(1 - sdk.ValidatorUpdateDelay)

        self.set_last_total_power(ctx, genesis.last_total_power)
            .unwrap_gas();
        self.staking_params_keeper.set(ctx, genesis.params.clone());

        for validator in genesis.validators {
            self.set_validator(ctx, &validator).unwrap_gas();
            // Manually set indices for the first time
            self.set_validator_by_cons_addr(ctx, &validator)
                .unwrap_gas();
            self.set_validator_by_power_index(ctx, &validator)
                .unwrap_gas();

            if !genesis.exported {
                self.after_validator_created(ctx, &validator);
            }

            if validator.status == BondStatus::Unbonding {
                self.insert_unbonding_validator_queue(ctx, &validator)
                    .unwrap_gas();
            }

            match validator.status {
                BondStatus::Bonded => {
                    bonded_tokens += validator.tokens;
                }
                BondStatus::Unbonding | BondStatus::Unbonded => {
                    not_bonded_tokens += validator.tokens;
                }
                // TODO: maybe move panics to abci handler
                BondStatus::Unspecified => {
                    return Err(StakingGenesisError::InvalidStatus(validator.status))
                }
            }
        }

        for delegation in genesis.delegations {
            if !genesis.exported {
                self.before_delegation_created(
                    ctx,
                    &delegation.delegator_address,
                    &delegation.validator_address,
                );
            }

            self.set_delegation(ctx, &delegation).unwrap_gas();

            if !genesis.exported {
                self.after_delegation_modified(
                    ctx,
                    &delegation.delegator_address,
                    &delegation.validator_address,
                );
            }
        }

        for unbonding_delegation in genesis.unbonding_delegations {
            self.set_unbonding_delegation(ctx, &unbonding_delegation)
                .unwrap_gas();
            for entry in unbonding_delegation.entries.as_slice() {
                self.insert_ubd_queue(ctx, &unbonding_delegation, entry.completion_time)
                    .unwrap_gas();
            }
        }

        for redelegation in genesis.redelegations {
            self.set_redelegation(ctx, &redelegation).unwrap_gas();
            for entry in &redelegation.entries {
                self.insert_redelegation_queue(ctx, &redelegation, entry.completion_time)
                    .unwrap_gas();
            }
        }

        let bonded_coins = if !bonded_tokens.is_zero() {
            vec![UnsignedCoin {
                denom: genesis.params.bond_denom().clone(),
                amount: bonded_tokens,
            }]
        } else {
            vec![]
        };
        let not_bonded_coins = if !not_bonded_tokens.is_zero() {
            vec![UnsignedCoin {
                denom: genesis.params.bond_denom().clone(),
                amount: not_bonded_tokens,
            }]
        } else {
            vec![]
        };

        let bonded_balance = self
            .bank_keeper
            .balance_all::<DB, InitContext<'_, DB, SK>>(ctx, self.bonded_module.address(), None)
            .unwrap_gas()
            .1;

        // there's a check in the cosmos SDK to ensure that a new module account is only created if the balance is zero
        // (the logic being that the module account will be set in the genesis file and created by the auth module
        // if the balance is non-zero)
        // see https://github.com/cosmos/cosmos-sdk/blob/2582f0aab7b2cbf66ade066fe570a4622cf0b098/x/staking/genesis.go#L107
        // However the call here in the cosmos SDK https://github.com/cosmos/cosmos-sdk/blob/2582f0aab7b2cbf66ade066fe570a4622cf0b098/x/staking/genesis.go#L101
        // has a side effect of creating a new module account.
        // So whatever the bonded balance a call is made in the staking module to create a new module account.
        self.auth_keeper
            .check_create_new_module_account(ctx, &self.bonded_module)
            .unwrap_gas();

        // if balance is different from bonded coins panic because genesis is most likely malformed
        if bonded_balance != bonded_coins {
            return Err(StakingGenesisError::WrongBondedPoolBalance(
                bonded_balance,
                bonded_coins,
            ));
        }

        let not_bonded_balance = self
            .bank_keeper
            .balance_all::<DB, InitContext<'_, DB, SK>>(ctx, self.not_bonded_module.address(), None)
            .unwrap_gas()
            .1;

        // see comment above for the logic of creating a new module account
        self.auth_keeper
            .check_create_new_module_account(ctx, &self.not_bonded_module)
            .unwrap_gas();

        // if balance is different from non bonded coins panic because genesis is most likely malformed
        if not_bonded_balance != not_bonded_coins {
            return Err(StakingGenesisError::WrongBondedPoolBalance(
                not_bonded_balance,
                not_bonded_coins,
            ));
        }

        let mut res = vec![];
        // don't need to run Tendermint updates if we exported
        if genesis.exported {
            for last_validator in genesis.last_validator_powers {
                self.set_last_validator_power(ctx, &last_validator)
                    .unwrap_gas();
                let validator = self.validator(ctx, &last_validator.address).unwrap_gas();

                let Some(validator) = validator else {
                    return Err(StakingGenesisError::ValidatorNotFound(
                        last_validator.address,
                    ));
                };

                let update = ValidatorUpdate {
                    pub_key: validator.consensus_pubkey,
                    power: (last_validator.power as u64).try_into()?,
                };
                res.push(update);
            }
        } else {
            // TODO: exit in sdk
            res = self.apply_and_return_validator_set_updates(ctx)?;
        }
        Ok(res)
    }

    /// BlockValidatorUpdates calculates the ValidatorUpdates for the current block
    /// Called in each EndBlock
    pub fn block_validator_updates<DB: Database>(
        &self,
        ctx: &mut BlockContext<'_, DB, SK>,
    ) -> Vec<ValidatorUpdate> {
        // Calculate validator set changes.

        // NOTE: ApplyAndReturnValidatorSetUpdates has to come before
        // UnbondAllMatureValidatorQueue.
        // This fixes a bug when the unbonding period is instant (is the case in
        // some of the tests). The test expected the validator to be completely
        // unbonded after the Endblocker (go from Bonded -> Unbonding during
        // ApplyAndReturnValidatorSetUpdates and then Unbonding -> Unbonded during
        // UnbondAllMatureValidatorQueue).
        let validator_updates = match self.apply_and_return_validator_set_updates(ctx) {
            Ok(v) => v,
            Err(e) => panic!("{}", e),
        };

        // unbond all mature validators from the unbonding queue
        self.unbond_all_mature_validators(ctx).unwrap_gas();

        // Remove all mature unbonding delegations from the ubd queue.
        let time = ctx.get_time();
        let mature_unbonds = self.dequeue_all_mature_ubd_queue(ctx, &time);
        for dv_pair in mature_unbonds {
            let val_addr = dv_pair.val_addr;
            let val_addr_str = val_addr.to_string();
            let del_addr = dv_pair.del_addr;
            let del_addr_str = del_addr.to_string();
            let balances = if let Ok(balances) = self.complete_unbonding(ctx, &val_addr, &del_addr)
            {
                balances
            } else {
                continue;
            };

            ctx.push_event(Event {
                r#type: EVENT_TYPE_COMPLETE_UNBONDING.to_string(),
                attributes: vec![
                    EventAttribute {
                        key: ATTRIBUTE_KEY_AMOUNT.into(),
                        value: serde_json::to_string(&balances)
                            .expect(SERDE_ENCODING_DOMAIN_TYPE)
                            .into(),
                        index: false,
                    },
                    EventAttribute {
                        key: ATTRIBUTE_KEY_VALIDATOR.into(),
                        value: val_addr_str.into(),
                        index: false,
                    },
                    EventAttribute {
                        key: ATTRIBUTE_KEY_DELEGATOR.into(),
                        value: del_addr_str.into(),
                        index: false,
                    },
                ],
            });
        }
        // Remove all mature redelegations from the red queue.
        let mature_redelegations = self.dequeue_all_mature_redelegation_queue(ctx, &time);
        for dvv_triplet in mature_redelegations {
            let val_src_addr = dvv_triplet.val_src_addr;
            let val_src_addr_str = val_src_addr.to_string();
            let val_dst_addr = dvv_triplet.val_dst_addr;
            let val_dst_addr_str = val_dst_addr.to_string();
            let del_addr = dvv_triplet.del_addr;
            let del_addr_str = del_addr.to_string();
            let balances = if let Ok(balances) =
                self.complete_redelegation(ctx, del_addr, val_src_addr, val_dst_addr)
            {
                balances
            } else {
                continue;
            };

            ctx.push_event(Event {
                r#type: EVENT_TYPE_COMPLETE_REDELEGATION.to_string(),
                attributes: vec![
                    EventAttribute {
                        key: ATTRIBUTE_KEY_AMOUNT.into(),
                        value: serde_json::to_string(&balances)
                            .expect(SERDE_ENCODING_DOMAIN_TYPE)
                            .into(),
                        index: false,
                    },
                    EventAttribute {
                        key: ATTRIBUTE_KEY_DELEGATOR.into(),
                        value: del_addr_str.into(),
                        index: false,
                    },
                    EventAttribute {
                        key: ATTRIBUTE_KEY_VALIDATOR.into(),
                        value: val_src_addr_str.into(),
                        index: false,
                    },
                    EventAttribute {
                        key: ATTRIBUTE_KEY_VALIDATOR.into(),
                        value: val_dst_addr_str.into(),
                        index: false,
                    },
                ],
            });
        }
        validator_updates
    }

    /// ApplyAndReturnValidatorSetUpdates applies and return accumulated updates to the bonded validator set. Also,
    /// * Updates the active valset as keyed by LastValidatorPowerKey.
    /// * Updates the total power as keyed by LastTotalPowerKey.
    /// * Updates validator status' according to updated powers.
    /// * Updates the fee pool bonded vs not-bonded tokens.
    /// * Updates relevant indices.
    ///
    /// It gets called once after genesis, another time maybe after genesis transactions,
    /// then once at every EndBlock.
    ///
    /// CONTRACT: Only validators with non-zero power or zero-power that were bonded
    /// at the previous block height or were removed from the validator set entirely
    /// are returned to Tendermint.
    pub fn apply_and_return_validator_set_updates<
        DB: Database,
        CTX: TransactionalContext<DB, SK> + InfallibleContext<DB, SK>,
    >(
        &self,
        ctx: &mut CTX,
    ) -> anyhow::Result<Vec<ValidatorUpdate>> {
        let params = self.staking_params_keeper.get(ctx);
        let max_validators = params.max_validators();
        let power_reduction = self.power_reduction(ctx);
        let mut total_power = 0;
        let mut amt_from_bonded_to_not_bonded = Uint256::zero();
        let mut amt_from_not_bonded_to_bonded = Uint256::zero();

        let mut last = self.last_validators_by_addr(ctx);
        let validators_map = self.validators_power_store_vals_vec(ctx)?;

        let mut updates = vec![];

        for val_addr in validators_map.iter().rev().take(max_validators as usize) {
            // everything that is iterated in this loop is becoming or already a
            // part of the bonded validator set
            let mut validator: Validator = self
                .validator(ctx, val_addr)?
                .expect("validator should be presented in store");

            if validator.jailed {
                return Err(anyhow::anyhow!(
                    "should never retrieve a jailed validator from the power store".to_string(),
                ));
            }
            // if we get to a zero-power validator (which we don't bond),
            // there are no more possible bonded validators
            if validator.tokens_to_consensus_power(self.power_reduction(ctx)) == 0 {
                break;
            }

            // apply the appropriate state change if necessary
            match validator.status {
                BondStatus::Unbonded => {
                    self.unbonded_to_bonded(ctx, &mut validator)?;
                    amt_from_not_bonded_to_bonded += validator.tokens;
                }
                BondStatus::Unbonding => {
                    self.unbonding_to_bonded(ctx, &mut validator)?;
                    amt_from_not_bonded_to_bonded += validator.tokens;
                }
                BondStatus::Bonded => {}
                BondStatus::Unspecified => return Err(anyhow!("unexpected validator status")),
            }

            // fetch the old power
            let old_power = last.get(val_addr);
            let new_power = validator.consensus_power(power_reduction);
            // update the validator set if power has changed

            if old_power.is_none() || old_power != Some(&new_power) {
                // TODO: check unwraps and update types to omit conversion
                updates.push(validator.abci_validator_update(power_reduction).unwrap());

                self.set_last_validator_power(
                    ctx,
                    &LastValidatorPower {
                        address: val_addr.clone(),
                        // TODO: update types to omit conversion
                        power: new_power as i64,
                    },
                )?;
            }

            last.remove(val_addr);

            total_power += new_power;
        }

        let no_longer_bonded = sort_no_longer_bonded(last)?;

        for val_addr in no_longer_bonded {
            let mut validator = self
                .validator(ctx, &val_addr)?
                .expect("validator should be presented in store");
            self.bonded_to_unbonding(ctx, &mut validator)?;
            amt_from_bonded_to_not_bonded += validator.tokens;
            self.delete_last_validator_power(ctx, &validator.operator_address)?;
            updates.push(validator.abci_validator_update_zero());
        }

        // Update the pools based on the recent updates in the validator set:
        // - The tokens from the non-bonded candidates that enter the new validator set need to be transferred
        // to the Bonded pool.
        // - The tokens from the bonded validators that are being kicked out from the validator set
        // need to be transferred to the NotBonded pool.
        // Compare and subtract the respective amounts to only perform one transfer.
        // This is done in order to avoid doing multiple updates inside each iterator/loop.
        match amt_from_not_bonded_to_bonded.cmp(&amt_from_bonded_to_not_bonded) {
            Ordering::Greater => {
                self.not_bonded_tokens_to_bonded(
                    ctx,
                    amt_from_not_bonded_to_bonded - amt_from_bonded_to_not_bonded,
                )?;
            }
            Ordering::Less => {
                self.bonded_tokens_to_not_bonded(
                    ctx,
                    amt_from_bonded_to_not_bonded - amt_from_not_bonded_to_bonded,
                )?;
            }
            Ordering::Equal => {
                // equal amounts of tokens; no update required
            }
        }

        // set total power on lookup index if there are any updates
        if !updates.is_empty() {
            self.set_last_total_power(ctx, Uint256::from_u128(total_power as u128))?;
        }

        Ok(updates)
    }

    pub fn power_reduction<DB: Database, CTX: QueryableContext<DB, SK>>(&self, _ctx: &CTX) -> u64 {
        // TODO: sdk constant in cosmos
        1_000_000
    }

    pub fn not_bonded_tokens_to_bonded<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        amount: Uint256,
    ) -> Result<(), GasStoreErrors> {
        // original routine is infallible, it means that the amount should be a valid number.
        // All errors in sdk panics in this method
        let params = self.staking_params_keeper.try_get(ctx)?;
        let coins = UnsignedCoins::new(vec![UnsignedCoin {
            denom: params.bond_denom().clone(),
            amount,
        }])
        .unwrap();

        self.bank_keeper
            .send_coins_from_module_to_module::<DB, CTX>(
                ctx,
                &self.not_bonded_module,
                &self.bonded_module,
                coins,
            )
            .unwrap();

        Ok(())
    }

    /// begin_info returns the completion time and height of a redelegation, along
    /// with a boolean signaling if the redelegation is complete based on the source
    /// validator.
    pub fn begin_info<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        val_addr: &ValAddress,
    ) -> Result<(Timestamp, u32, bool), GasStoreErrors> {
        // TODO: When would the validator not be found?
        let validator = self.validator(ctx, val_addr)?;
        let validator_status = validator
            .as_ref()
            .map(|v| v.status)
            .unwrap_or(BondStatus::Bonded);

        match validator_status {
            BondStatus::Bonded => {
                // the longest wait - just unbonding period from now
                let params = self.staking_params_keeper.try_get(ctx)?;
                let duration = params.unbonding_time();

                let completion_time = ctx.get_time().checked_add(duration).unwrap();
                let height = ctx.height();
                Ok((completion_time, height, false))
            }
            BondStatus::Unbonded => Ok((Timestamp::UNIX_EPOCH, 0, true)),
            BondStatus::Unbonding => {
                let validator = validator.unwrap();
                Ok((validator.unbonding_time, validator.unbonding_height, false))
            }
            // TODO: maybe change signature and move panic
            BondStatus::Unspecified => panic!("unexpected validator status"),
        }
    }

    pub fn pool<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
    ) -> Result<Pool, GasStoreErrors> {
        let denom = self.staking_params_keeper.try_get(ctx)?.bond_denom;
        let not_bonded_tokens = self
            .bank_keeper
            .balance_all(ctx, self.not_bonded_module.address(), None)?
            .1
            .into_iter()
            .find(|e| e.denom == denom);
        let bonded_tokens = self
            .bank_keeper
            .balance_all(ctx, self.bonded_module.address(), None)?
            .1
            .into_iter()
            .find(|e| e.denom == denom);
        Ok(Pool {
            not_bonded_tokens: not_bonded_tokens.map(|t| t.amount).unwrap_or_default(),
            bonded_tokens: bonded_tokens.map(|t| t.amount).unwrap_or_default(),
        })
    }

    pub fn params<DB: Database>(&self, ctx: &QueryContext<DB, SK>) -> StakingParams {
        self.staking_params_keeper.get(ctx)
    }
}

/// given a map of remaining validators to previous bonded power
/// returns the list of validators to be unbonded, sorted by operator address
fn sort_no_longer_bonded(last: HashMap<ValAddress, u64>) -> anyhow::Result<Vec<ValAddress>> {
    let mut no_longer_bonded = last.into_keys().collect::<Vec<_>>();
    // sorted by address - order doesn't matter
    no_longer_bonded.sort();
    Ok(no_longer_bonded)
}
