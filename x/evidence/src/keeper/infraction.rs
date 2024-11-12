use super::*;
use crate::{
    errors::EquivocationEvidenceError,
    types::{Equivocation, RawEquivocation, DOUBLE_SIGN_JAIL_END_TIME},
};
use gears::{
    context::block::BlockContext,
    error::{MathOperation, NumericError},
    x::{
        keepers::staking::VALIDATOR_UPDATE_DELAY,
        types::validator::{BondStatus, StakingValidator},
    },
};

impl<
        SK: StoreKey,
        StkK: SlashingStakingKeeper<SK, M>,
        SlsK: EvidenceSlashingKeeper<SK, M>,
        E: Evidence + Default,
        M: Module,
    > Keeper<SK, StkK, SlsK, E, M>
where
    <E as std::convert::TryFrom<Any>>::Error: std::fmt::Debug,
{
    /// HandleEquivocationEvidence implements an equivocation evidence handler. Assuming the
    /// evidence is valid, the validator committing the misbehavior will be slashed,
    /// jailed and tombstoned. Once tombstoned, the validator will not be able to
    /// recover. Note, the evidence contains the block time and height at the time of
    /// the equivocation.
    ///
    /// The evidence is considered invalid if:
    /// - the evidence is too old
    /// - the validator is unbonded or does not exist
    /// - the signing info does not exist (will panic)
    /// - is already tombstoned

    // TODO: Some of the invalid constraints listed above may need to be reconsidered
    // in the case of a lunatic attack.
    pub fn handle_equivocation_evidence<DB: Database>(
        &self,
        ctx: &mut BlockContext<'_, DB, SK>,
        evidence: &Equivocation,
        // TODO: error type
    ) -> anyhow::Result<()> {
        let cons_address = evidence.consensus_address.clone();
        if self
            .slashing_keeper
            .pubkey(ctx, &cons_address)
            .unwrap_gas()
            .is_none()
        {
            // Ignore evidence that cannot be handled.
            //
            // NOTE: We used to panic with:
            // `panic!(format!("Validator consensus-address {} not found", cons_address))`,
            // but this couples the expectations of the app to both Tendermint.  Both are
            // expected to provide the full range of allowable but none of the disallowed
            // evidence types.  Instead of getting this coordination right, it is easier to
            // relax the constraints and ignore evidence that cannot be handled.
            return Ok(());
        }

        // calculate the age of the evidence
        let infraction_height = evidence.height;
        let infraction_time = evidence.time;
        let age_duration = ctx
            .get_time()
            .checked_sub(&infraction_time)
            .ok_or(NumericError::Overflow(MathOperation::Sub))?;
        let age_blocks = ctx
            .height()
            .checked_sub(infraction_height.try_into()?)
            .ok_or(NumericError::Overflow(MathOperation::Sub))?;

        // Reject evidence if the double-sign is too old. Evidence is considered stale
        // if the difference in time and number of blocks is greater than the allowed
        // parameters defined.
        let cons_params = ctx.consensus_params();

        if let Some(max_age_duration) = cons_params.evidence.max_age_duration {
            if age_duration > max_age_duration
                && age_blocks > cons_params.evidence.max_age_num_blocks.try_into()?
            {
                tracing::info!(
                    name: "ignored equivocation; evidence too old",
                    target: "module::evidence",
                    validator = cons_address.to_string(),
                    ?infraction_height,
                    max_age_num_blocks = cons_params.evidence.max_age_num_blocks,
                    infraction_time = infraction_time.format_string_rounded(),
                    // TODO: what is better option to show duration?
                    max_age_duration = i128::from(max_age_duration.duration_nanoseconds()),
                );
                return Ok(());
            }
        }

        let validator = if let Some(validator) = self
            .staking_keeper
            .validator_by_cons_addr(ctx, &cons_address)
            .unwrap_gas()
        {
            if validator.status() == BondStatus::Unbonded {
                // Defensive: Simulation doesn't take unbonding periods into account, and
                // Tendermint might break this assumption at some point.
                return Ok(());
            }
            validator
        } else {
            return Ok(());
        };

        if !self
            .slashing_keeper
            .has_validator_signing_info(ctx, &cons_address)
            .unwrap_gas()
        {
            return Err(EquivocationEvidenceError::SigningInfoNotExists(cons_address).into());
        }

        // ignore if the validator is already tombstoned
        if self
            .slashing_keeper
            .is_tombstoned(ctx, &cons_address)
            .unwrap_gas()
        {
            tracing::info!(
                name: "ignored equivocation; validator already tombstoned",
                target: "module::evidence",
                validator = cons_address.to_string(),
                ?infraction_height,
                infraction_time = infraction_time.format_string_rounded(),
            );
            return Ok(());
        }
        tracing::info!(
            name: "confirmed equivocation",
            target: "module::evidence",
            validator = cons_address.to_string(),
            ?infraction_height,
            infraction_time = infraction_time.format_string_rounded(),
        );
        // We need to retrieve the stake distribution which signed the block, so we
        // subtract ValidatorUpdateDelay from the evidence height.
        // Note, that this *can* result in a negative "distributionHeight", up to
        // -ValidatorUpdateDelay, i.e. at the end of the
        // pre-genesis block (none) = at the beginning of the genesis block.
        // That's fine since this is just used to filter unbonding delegations & redelegations.
        let distribution_height = infraction_height
            .checked_sub(VALIDATOR_UPDATE_DELAY as i64)
            .ok_or(NumericError::Overflow(MathOperation::Sub))?;

        // Slash validator. The `power` is the int64 power of the validator as provided
        // to/by Tendermint. This value is validator.Tokens as sent to Tendermint via
        // ABCI, and now received as evidence. The fraction is passed in to separately
        // to slash unbonding and rebonding delegations.
        self.slashing_keeper
            .slash(
                ctx,
                &cons_address,
                self.slashing_keeper
                    .slash_fraction_double_sign(ctx)
                    .unwrap_gas(),
                evidence.power.into(),
                distribution_height,
            )
            .unwrap_gas();

        // Jail the validator if not already jailed. This will begin unbonding the
        // validator if not already unbonding (tombstoned).
        if !validator.is_jailed() {
            self.slashing_keeper.jail(ctx, &cons_address).unwrap_gas();
        }

        self.slashing_keeper
            .jail_until(ctx, &cons_address, DOUBLE_SIGN_JAIL_END_TIME)
            .unwrap_gas();
        self.slashing_keeper
            .tombstone(ctx, &cons_address)
            .unwrap_gas();
        self.set_evidence(ctx, &RawEquivocation::from(evidence.clone()))
            .unwrap_gas();

        Ok(())
    }
}
