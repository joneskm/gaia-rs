use crate::{
    errors::DistributionError, params::DistributionParamsKeeper, GenesisState,
    ValidatorAccumulatedCommission, ValidatorOutstandingRewards,
};
use anyhow::anyhow;
use gears::{extensions::gas::GasResultExt, gas::store::errors::GasStoreErrors};
pub use gears::{
    context::init::InitContext,
    params::ParamsSubspaceKey,
    store::{database::Database, StoreKey},
    x::{
        keepers::{
            auth::AuthKeeper, staking::SlashingStakingKeeper,
            staking::StakingBankKeeper as BankKeeper,
        },
        module::Module,
        types::validator::StakingValidator,
    },
};
use gears::{
    context::{tx::TxContext, TransactionalContext},
    tendermint::types::proto::event::{Event, EventAttribute},
    types::{
        address::{AccAddress, ConsAddress, ValAddress},
        base::coins::{DecimalCoins, UnsignedCoins},
    },
    x::keepers::staking::DistributionStakingKeeper,
};
use std::collections::HashMap;

mod allocation;
mod delegation;
mod query;
mod store;
mod tx;
mod validator;

/// Keeper of the slashing store
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Keeper<
    SK: StoreKey,
    PSK: ParamsSubspaceKey,
    AK: AuthKeeper<SK, M>,
    BK: BankKeeper<SK, M>,
    DSK: DistributionStakingKeeper<SK, M>,
    M: Module,
> {
    store_key: SK,
    auth_keeper: AK,
    bank_keeper: BK,
    params_keeper: DistributionParamsKeeper<PSK>,
    staking_keeper: DSK,
    fee_collector_module: M,
    distribution_module: M,
    blocked_addrs: HashMap<String, bool>,
}

impl<
        SK: StoreKey,
        PSK: ParamsSubspaceKey,
        AK: AuthKeeper<SK, M>,
        BK: BankKeeper<SK, M>,
        DSK: DistributionStakingKeeper<SK, M>,
        M: Module,
    > Keeper<SK, PSK, AK, BK, DSK, M>
{
    pub fn new(
        store_key: SK,
        params_subspace_key: PSK,
        auth_keeper: AK,
        bank_keeper: BK,
        staking_keeper: DSK,
        fee_collector_module: M,
        distribution_module: M,
        blocked_addrs: HashMap<String, bool>,
    ) -> Self {
        Self {
            store_key,
            params_keeper: DistributionParamsKeeper {
                params_subspace_key,
            },
            auth_keeper,
            bank_keeper,
            staking_keeper,
            fee_collector_module,
            distribution_module,
            blocked_addrs,
        }
    }

    pub fn init_genesis<DB: Database>(
        &self,
        ctx: &mut InitContext<'_, DB, SK>,
        genesis: GenesisState,
    ) -> anyhow::Result<()> {
        self.set_fee_pool(ctx, &genesis.fee_pool).unwrap_gas();
        self.params_keeper.set(ctx, genesis.params);

        genesis.delegator_withdraw_infos.iter().for_each(|dwi| {
            self.set_delegator_withdraw_addr(ctx, &dwi.delegator_address, &dwi.withdraw_address)
                .unwrap_gas()
        });

        let previous_proposer = if !genesis.previous_proposer.is_empty() {
            ConsAddress::from_bech32(&genesis.previous_proposer)?
        } else {
            todo!("the sdk doesn't have this branch. It may use a default value");
        };

        self.set_previous_proposer_cons_addr(ctx, &previous_proposer);

        let mut module_holdings = vec![];
        for rew in genesis.outstanding_rewards {
            self.set_validator_outstanding_rewards(
                ctx,
                &rew.validator_address,
                &rew.outstanding_rewards,
            )
            .unwrap_gas();
            module_holdings.push(rew.outstanding_rewards.rewards);
        }
        let start = module_holdings[0].clone();
        let module_holdings = module_holdings
            .into_iter()
            .take(1)
            .try_fold(start, |acc, holdings| acc.checked_add(&holdings))?;

        genesis
            .validator_accumulated_commissions
            .iter()
            .for_each(|vac| {
                self.set_validator_accumulated_commission(
                    ctx,
                    &vac.validator_address,
                    &vac.accumulated,
                )
                .unwrap_gas()
            });

        genesis.validator_historical_rewards.iter().for_each(|vhr| {
            self.set_validator_historical_rewards(
                ctx,
                &vhr.validator_address,
                vhr.period,
                &vhr.rewards,
            )
            .unwrap_gas()
        });

        genesis.validator_current_rewards.iter().for_each(|vcr| {
            self.set_validator_current_rewards(ctx, &vcr.validator_address, &vcr.rewards)
                .unwrap_gas()
        });

        genesis.delegator_starting_infos.iter().for_each(|dsi| {
            self.set_delegator_starting_info(
                ctx,
                &dsi.validator_address,
                &dsi.delegator_address,
                &dsi.starting_info,
            )
            .unwrap_gas()
        });

        genesis.validator_slash_events.iter().for_each(|vse| {
            self.set_validator_slash_event(
                ctx,
                &vse.validator_address,
                vse.height,
                vse.period,
                &vse.validator_slash_event,
            )
        });

        let module_holdings = module_holdings.checked_add(&genesis.fee_pool.community_pool)?;
        let (module_holdings_int, _) = module_holdings.truncate_decimal();

        // check if the module account exists

        self.check_set_distribution_account(ctx).unwrap_gas();
        let balances = self
            .bank_keeper
            .balance_all(ctx, self.distribution_module.address(), None)
            .unwrap_gas()
            .1;

        if module_holdings_int != Some(UnsignedCoins::new(balances)?) {
            return Err(anyhow!(
                "distribution module balance does not match the module holdings".to_string(),
            ));
        }

        Ok(())
    }

    /// check_set_distribution_account creates module account for current module
    pub fn check_set_distribution_account<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
    ) -> Result<(), GasStoreErrors> {
        // TODO: here is fallible call with module as self
        self.auth_keeper
            .check_create_new_module_account(ctx, &self.distribution_module)
    }

    /// withdraw rewards from a delegation
    pub fn withdraw_delegation_rewards<DB: Database>(
        &self,
        ctx: &mut TxContext<'_, DB, SK>,
        delegator_address: &AccAddress,
        validator_address: &ValAddress,
    ) -> Result<Option<UnsignedCoins>, DistributionError> {
        let validator = if let Some(val) = self.staking_keeper.validator(ctx, validator_address)? {
            val
        } else {
            return Err(DistributionError::AccountNotFound(
                validator_address.clone().into(),
            ));
        };
        let delegation = if let Some(del) =
            self.staking_keeper
                .delegation(ctx, delegator_address, validator_address)?
        {
            del
        } else {
            return Err(DistributionError::DelegationNotFound(
                delegator_address.clone(),
                validator_address.clone(),
            ));
        };

        // withdraw rewards
        let rewards = self.delegation_withdraw_rewards(ctx, validator, delegation)?;

        // reinitialize the delegation
        self.initialize_delegation(ctx, validator_address, delegator_address)?;
        Ok(rewards)
    }

    /// withdraw validator commission
    pub fn withdraw_validator_commission<DB: Database>(
        &self,
        ctx: &mut TxContext<'_, DB, SK>,
        validator_address: &ValAddress,
    ) -> Result<Option<UnsignedCoins>, DistributionError> {
        // fetch validator accumulated commission
        let accumulated_commission = self
            .validator_accumulated_commission(ctx, validator_address)?
            .ok_or(DistributionError::ValidatorAccumulatedCommissionNotFound(
                validator_address.clone(),
            ))?;
        if accumulated_commission.commission.is_empty() {
            return Err(DistributionError::ValidatorAccumulatedCommissionNotSet(
                validator_address.clone(),
            ));
        }

        let (commission, reminder) = accumulated_commission.commission.truncate_decimal();

        if let Some(rem) = reminder {
            // leave remainder to withdraw later
            self.set_validator_accumulated_commission(
                ctx,
                validator_address,
                &ValidatorAccumulatedCommission { commission: rem },
            )?
        }

        // update outstanding
        let outstanding = self
            .validator_outstanding_rewards(ctx, validator_address)?
            .ok_or(DistributionError::ValidatorOutstandingRewardsNotFound(
                validator_address.clone(),
            ))?;
        let rewards = if let Some(commission) = &commission {
            outstanding
                .rewards
                .checked_sub(&DecimalCoins::try_from(commission.inner().clone())?)?
        } else {
            outstanding.rewards
        };
        self.set_validator_outstanding_rewards(
            ctx,
            validator_address,
            &ValidatorOutstandingRewards { rewards },
        )?;

        if let Some(commission) = &commission {
            let acc_address = AccAddress::from(validator_address.clone());
            let withdraw_address = self
                .delegator_withdraw_addr(ctx, &acc_address)?
                .unwrap_or(acc_address);
            self.bank_keeper.send_coins_from_module_to_account(
                ctx,
                &withdraw_address,
                &self.distribution_module,
                commission.clone(),
            )?;
        }

        ctx.push_event(Event {
            r#type: "withdraw_commission".to_string(),
            attributes: vec![EventAttribute {
                key: "amount".into(),
                // TODO: stringify coins structs
                value: serde_json::to_string(&commission)
                    .expect("serde can't fail")
                    .into(),
                index: false,
            }],
        });

        Ok(commission)
    }

    /// fund_community_pool allows an account to directly fund the community fund pool.
    /// The amount is first added to the distribution module account and then directly
    /// added to the pool. An error is returned if the amount cannot be sent to the
    /// module account.
    pub fn fund_community_pool<DB: Database>(
        &self,
        ctx: &mut TxContext<'_, DB, SK>,
        amount: UnsignedCoins,
        sender: &AccAddress,
    ) -> Result<(), DistributionError> {
        self.bank_keeper.send_coins_from_account_to_module(
            ctx,
            sender.clone(),
            &self.distribution_module,
            amount.clone(),
        )?;
        let mut fee_pool = self.fee_pool(ctx)?.ok_or(DistributionError::FeePoolNone)?;
        fee_pool.community_pool = fee_pool
            .community_pool
            .checked_add(&DecimalCoins::try_from(amount.into_inner())?)?;
        self.set_fee_pool(ctx, &fee_pool)?;
        Ok(())
    }
}
