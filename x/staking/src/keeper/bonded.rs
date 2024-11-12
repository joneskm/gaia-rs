use super::*;
use gears::types::base::coins::UnsignedCoins;

impl<
        SK: StoreKey,
        PSK: ParamsSubspaceKey,
        AK: AuthKeeper<SK, M>,
        BK: StakingBankKeeper<SK, M>,
        KH: KeeperHooks<SK, AK, M>,
        M: Module,
    > Keeper<SK, PSK, AK, BK, KH, M>
{
    pub fn bonded_tokens_to_not_bonded<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        amount: Uint256,
    ) -> Result<(), GasStoreErrors> {
        let params = self.staking_params_keeper.try_get(ctx)?;

        // original routine is infallible, it means that the amount should be a valid number.
        // All errors in sdk panics in this method
        let coins = UnsignedCoins::new(vec![UnsignedCoin {
            denom: params.bond_denom().clone(),
            amount,
        }])
        .expect("shouldn't fail");

        self.bank_keeper
            .send_coins_from_module_to_module::<DB, CTX>(
                ctx,
                &self.bonded_module,
                &self.not_bonded_module,
                coins,
            )
            .unwrap();

        Ok(())
    }

    pub fn bonded_to_unbonding<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        validator: &mut Validator,
    ) -> anyhow::Result<()> {
        if validator.status != BondStatus::Bonded {
            return Err(anyhow::anyhow!(
                "bad state transition bonded to unbonding, validator: {}",
                validator.operator_address
            ));
        }
        self.begin_unbonding_validator(ctx, validator)
    }

    pub fn bond_validator<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        validator: &mut Validator,
    ) -> Result<(), GasStoreErrors> {
        // delete the validator by power index, as the key will change
        self.delete_validator_by_power_index(ctx, validator)?;

        validator.update_status(BondStatus::Bonded);
        // save the now bonded validator record to the two referenced stores
        self.set_validator(ctx, validator)?;
        self.set_validator_by_power_index(ctx, validator)?;

        // delete from queue if present
        self.delete_unbonding_validators_queue(ctx, validator)?;
        // trigger hook
        self.after_validator_bonded(ctx, validator);

        Ok(())
    }
}
