use std::{
    collections::HashMap,
    marker::PhantomData,
    ops::{Add, Div, Mul},
};

use anyhow::anyhow;
use chrono::DateTime;
use gears::{
    application::keepers::params::ParamsKeeper,
    context::{
        block::BlockContext, init::InitContext, tx::TxContext, QueryableContext,
        TransactionalContext,
    },
    error::AppError,
    params::ParamsSubspaceKey,
    store::{database::Database, StoreKey},
    tendermint::types::proto::event::{Event, EventAttribute},
    types::{
        address::{AccAddress, ValAddress},
        decimal256::Decimal256,
        store::gas::{errors::GasStoreErrors, ext::GasResultExt},
    },
    x::{
        keepers::{bank::BankKeeper, staking::GovStakingKeeper},
        module::Module,
        types::{delegation::StakingDelegation, validator::StakingValidator},
    },
};
use strum::IntoEnumIterator;

use crate::{
    errors::SERDE_JSON_CONVERSION,
    genesis::GovGenesisState,
    msg::{
        deposit::MsgDeposit,
        proposal::MsgSubmitProposal,
        vote::VoteOption,
        weighted_vote::{MsgVoteWeighted, VoteOptionWeighted},
    },
    params::GovParamsKeeper,
    types::{
        deposit_iter::DepositIterator,
        proposal::{
            active_iter::ActiveProposalIterator, inactive_iter::InactiveProposalIterator, Proposal,
            ProposalStatus, TallyResult,
        },
        validator::ValidatorGovInfo,
        vote_iters::WeightedVoteIterator,
    },
};

const PROPOSAL_ID_KEY: [u8; 1] = [0x03];
pub(crate) const KEY_PROPOSAL_PREFIX: [u8; 1] = [0x00];

#[derive(Debug, Clone)]
pub struct GovKeeper<
    SK: StoreKey,
    PSK: ParamsSubspaceKey,
    M: Module,
    BK: BankKeeper<SK, M>,
    STK: GovStakingKeeper<SK, M>,
> {
    store_key: SK,
    gov_params_keeper: GovParamsKeeper<PSK>,
    gov_mod: M,
    bank_keeper: BK,
    staking_keeper: STK,
    _bank_marker: PhantomData<M>,
}

impl<
        SK: StoreKey,
        PSK: ParamsSubspaceKey,
        M: Module,
        BK: BankKeeper<SK, M>,
        STK: GovStakingKeeper<SK, M>,
    > GovKeeper<SK, PSK, M, BK, STK>
{
    pub fn new(
        store_key: SK,
        params_subspace_key: PSK,
        gov_mod: M,
        bank_keeper: BK,
        staking_keeper: STK,
    ) -> Self {
        Self {
            store_key,
            gov_params_keeper: GovParamsKeeper {
                params_subspace_key,
            },
            gov_mod,
            bank_keeper,
            staking_keeper,
            _bank_marker: PhantomData,
        }
    }

    pub fn init_genesis<DB: Database>(
        &self,
        ctx: &mut InitContext<'_, DB, SK>,
        GovGenesisState {
            starting_proposal_id,
            deposits,
            votes,
            proposals,
            params,
        }: GovGenesisState,
    ) {
        {
            let mut store = ctx.kv_store_mut(&self.store_key);
            store.set(PROPOSAL_ID_KEY, starting_proposal_id.to_be_bytes())
        }
        self.gov_params_keeper.set(ctx, params);

        let total_deposits = {
            let mut store_mut = ctx.kv_store_mut(&self.store_key);

            let total_deposits = {
                let mut total_deposits = Vec::with_capacity(deposits.len());
                for deposit in deposits {
                    store_mut.set(
                        MsgDeposit::key(deposit.proposal_id, &deposit.depositor),
                        serde_json::to_vec(&deposit).expect(SERDE_JSON_CONVERSION),
                    ); // TODO:NOW IS THIS CORRECT SERIALIZATION?
                    total_deposits.push(deposit.amount);
                }

                total_deposits.into_iter().flatten().collect::<Vec<_>>()
            };

            for vote in votes {
                store_mut.set(
                    MsgVoteWeighted::key(vote.proposal_id, &vote.voter),
                    serde_json::to_vec(&vote).expect(SERDE_JSON_CONVERSION),
                )
            }

            for proposal in proposals {
                match proposal.status {
                    ProposalStatus::DepositPeriod => {
                        store_mut.set(
                            Proposal::inactive_queue_key(
                                proposal.proposal_id,
                                &proposal.deposit_end_time,
                            ),
                            proposal.proposal_id.to_be_bytes(),
                        );
                    }
                    ProposalStatus::VotingPeriod => store_mut.set(
                        Proposal::active_queue_key(
                            proposal.proposal_id,
                            &proposal.deposit_end_time,
                        ),
                        proposal.proposal_id.to_be_bytes(),
                    ),
                    _ => (),
                }

                store_mut.set(
                    proposal.key(),
                    serde_json::to_vec(&proposal).expect(SERDE_JSON_CONVERSION),
                );
            }

            total_deposits
        };

        let balance = self
            .bank_keeper
            .balance_all(ctx, &self.gov_mod.get_address())
            .unwrap_gas();
        /*
           Okay. I think that in our implementation there is no need to create account if it.

           So I should omit this lines...
           if balance.is_empty() || balance.iter().any(|this| this.amount.is_zero()) {
               https://github.com/cosmos/cosmos-sdk/blob/d3f09c222243bb3da3464969f0366330dcb977a8/x/gov/genesis.go#L47
           }
        */

        if !(balance == total_deposits) {
            panic!(
                "expected module account was {:?} but we got {:?}",
                balance, total_deposits
            )
        }
    }

    pub fn deposit_add<DB: Database>(
        &self,
        ctx: &mut TxContext<'_, DB, SK>,
        MsgDeposit {
            proposal_id,
            depositor,
            amount,
        }: MsgDeposit,
    ) -> anyhow::Result<bool> {
        let mut proposal = proposal_get(ctx, &self.store_key, proposal_id)?
            .ok_or(anyhow!("unknown proposal {proposal_id}"))?;

        match proposal.status {
            ProposalStatus::DepositPeriod | ProposalStatus::VotingPeriod => Ok(()),
            _ => Err(anyhow!("inactive proposal {proposal_id}")),
        }?;

        self.bank_keeper.send_coins_from_account_to_module(
            ctx,
            depositor.clone(),
            &self.gov_mod,
            amount.clone(),
        )?;

        proposal.total_deposit = proposal.total_deposit.checked_add(amount.clone())?;
        proposal_set(ctx, &self.store_key, &proposal)?;

        let deposit_params = self.gov_params_keeper.try_get(ctx)?.deposit;
        let activated_voting_period = match proposal.status {
            ProposalStatus::DepositPeriod
                if proposal
                    .total_deposit
                    .is_all_gte(&deposit_params.min_deposit) =>
            {
                true
            }
            _ => false,
        };

        let deposit = match deposit_get(ctx, &self.store_key, proposal_id, &depositor)? {
            Some(mut deposit) => {
                deposit.amount = deposit.amount.checked_add(amount)?;
                deposit
            }
            None => MsgDeposit {
                proposal_id,
                depositor,
                amount,
            },
        };

        // TODO: ADD HOOK https://github.com/cosmos/cosmos-sdk/blob/d3f09c222243bb3da3464969f0366330dcb977a8/x/gov/keeper/deposit.go#L149

        ctx.push_event(Event::new(
            "proposal_deposit",
            vec![
                EventAttribute::new(
                    "amount".into(),
                    format!("{:?}", deposit.amount).into(),
                    false,
                ),
                EventAttribute::new(
                    "proposal_id".into(),
                    format!("{}", deposit.proposal_id).into(),
                    false,
                ),
            ],
        ));

        deposit_set(ctx, &self.store_key, &deposit)?;

        Ok(activated_voting_period)
    }

    pub fn vote_add<DB: Database>(
        &self,
        ctx: &mut TxContext<'_, DB, SK>,
        vote: MsgVoteWeighted,
    ) -> anyhow::Result<()> {
        let proposal = proposal_get(ctx, &self.store_key, vote.proposal_id)?
            .ok_or(anyhow!("unknown proposal {}", vote.proposal_id))?;

        match proposal.status {
            ProposalStatus::VotingPeriod => Ok(()),
            _ => Err(anyhow!("inactive proposal {}", vote.proposal_id)),
        }?;

        vote_set(ctx, &self.store_key, &vote)?;

        // TODO:NOW HOOK https://github.com/cosmos/cosmos-sdk/blob/d3f09c222243bb3da3464969f0366330dcb977a8/x/gov/keeper/vote.go#L31

        ctx.push_event(Event::new(
            "proposal_vote",
            vec![
                EventAttribute::new("option".into(), format!("{:?}", vote.options).into(), false),
                EventAttribute::new(
                    "proposal_id".into(),
                    format!("{}", vote.proposal_id).into(),
                    false,
                ),
            ],
        ));

        Ok(())
    }

    pub fn submit_proposal<DB: Database>(
        &self,
        ctx: &mut TxContext<'_, DB, SK>,
        MsgSubmitProposal {
            content,
            initial_deposit,
            proposer: _proposer,
        }: MsgSubmitProposal,
    ) -> anyhow::Result<u64> {
        /*
           in go they perform check is it possible to handle
           proposal somehow, but not sure we need it and instead
           handle manually. at least this is such concept at moment

           https://github.com/cosmos/cosmos-sdk/blob/d3f09c222243bb3da3464969f0366330dcb977a8/x/gov/keeper/proposal.go#L14-L16
        */

        let proposal_id = proposal_id_get(ctx, &self.store_key)?;
        let submit_time = ctx.header().time.clone();
        let deposit_period = self
            .gov_params_keeper
            .try_get(ctx)?
            .deposit
            .max_deposit_period;

        let submit_date =
            DateTime::from_timestamp(submit_time.seconds, submit_time.nanos as u32).unwrap(); // TODO

        let proposal = Proposal {
            proposal_id,
            content,
            status: ProposalStatus::DepositPeriod,
            final_tally_result: Default::default(),
            submit_time: submit_date,
            deposit_end_time: submit_date.add(deposit_period),
            total_deposit: initial_deposit,
            voting_start_time: None,
            voting_end_time: None,
        };

        proposal_set(ctx, &self.store_key, &proposal)?;
        let mut store = ctx.kv_store_mut(&self.store_key);

        store.set(
            Proposal::inactive_queue_key(proposal.proposal_id, &proposal.deposit_end_time),
            proposal.proposal_id.to_be_bytes(),
        )?;

        store.set(PROPOSAL_ID_KEY, (proposal_id + 1).to_be_bytes())?;

        // TODO:NOW HOOK https://github.com/cosmos/cosmos-sdk/blob/d3f09c222243bb3da3464969f0366330dcb977a8/x/gov/keeper/proposal.go#L45

        ctx.push_event(Event::new(
            "submit_proposal",
            vec![EventAttribute::new(
                "proposal_id".into(),
                proposal_id.to_string().into(),
                false,
            )],
        ));

        Ok(proposal.proposal_id)
    }

    pub fn end_block<DB: Database>(&self, ctx: &mut BlockContext<'_, DB, SK>) -> Vec<Event> {
        let mut events = Vec::new();

        let time = DateTime::from_timestamp(ctx.header.time.seconds, ctx.header.time.nanos as u32)
            .unwrap(); // TODO

        {
            let inactive_iter = {
                let store = ctx.kv_store(&self.store_key);
                InactiveProposalIterator::new(store.into(), &time)
                    .map(|this| this.map(|((proposal_id, _), _)| proposal_id))
                    .collect::<Vec<_>>()
            };

            for var in inactive_iter {
                let proposal_id = var.unwrap_gas();
                proposal_del(ctx, &self.store_key, proposal_id).unwrap_gas();
                deposit_del(ctx, self, proposal_id).unwrap_gas();

                // TODO: HOOK https://github.com/cosmos/cosmos-sdk/blob/d3f09c222243bb3da3464969f0366330dcb977a8/x/gov/abci.go#L24-L25

                events.push(Event::new(
                    "inactive_proposal",
                    vec![
                        EventAttribute::new(
                            "proposal_id".into(),
                            proposal_id.to_string().into(),
                            false,
                        ),
                        EventAttribute::new(
                            "proposal_result".into(),
                            "proposal_dropped".into(),
                            false,
                        ),
                    ],
                ))
            }
        }

        {
            let active_iter = {
                let store = ctx.kv_store(&self.store_key).into();
                ActiveProposalIterator::new(store, &time)
                    .map(|this| this.map(|((proposal_id, _), _)| proposal_id))
                    .collect::<Vec<_>>()
            };

            for proposal_id in active_iter {
                let proposal_id = proposal_id.unwrap_gas();

                let (passes, burn_deposit, tally_result) =
                    self.tally(ctx, proposal_id).unwrap_gas();

                if burn_deposit {
                    deposit_del(ctx, self, proposal_id).unwrap_gas();
                } else {
                }
            }
        }

        events
    }

    fn tally<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        proposal_id: u64,
    ) -> Result<(bool, bool, TallyResult), GasStoreErrors> {
        let mut curr_validators = HashMap::<ValAddress, ValidatorGovInfo>::new();

        for validator in self.staking_keeper.bonded_validators_by_power_iter(ctx)? {
            let validator = validator?;

            curr_validators.insert(
                validator.operator().clone(),
                ValidatorGovInfo {
                    address: validator.operator().clone(),
                    bounded_tokens: validator.bonded_tokens().clone(),
                    delegator_shares: validator.delegator_shares().clone(),
                    delegator_deduction: Decimal256::zero(),
                    vote: Vec::new(),
                },
            );
        }

        let mut tally_results = TallyResultMap::new();
        let mut total_voting_power = Decimal256::zero();

        for vote in WeightedVoteIterator::new(ctx.kv_store(&self.store_key), proposal_id)
            .map(|this| this.map(|(_, value)| value))
            .collect::<Vec<_>>()
        {
            let MsgVoteWeighted {
                proposal_id: _,
                voter,
                options: vote_options,
            } = vote?;

            let val_addr = ValAddress::from(voter.clone());
            if let Some(validator) = curr_validators.get_mut(&val_addr) {
                validator.vote = vote_options.clone();
            }

            for delegation in self
                .staking_keeper
                .delegations_iter(ctx, &voter)
                .collect::<Vec<_>>()
            {
                let delegation = delegation?;

                if let Some(validator) = curr_validators.get_mut(delegation.validator()) {
                    // from cosmos: https://github.com/cosmos/cosmos-sdk/blob/d3f09c222243bb3da3464969f0366330dcb977a8/x/gov/keeper/tally.go#L51
                    // There is no need to handle the special case that validator address equal to voter address.
                    // Because voter's voting power will tally again even if there will deduct voter's voting power from validator.

                    validator.delegator_deduction += delegation.shares(); // TODO: handle overflow?

                    // delegation shares * bonded / total shares
                    let voting_power = delegation
                        .shares()
                        .mul(Decimal256::new(validator.bounded_tokens))
                        .div(validator.delegator_shares);

                    for VoteOptionWeighted { option, weight } in &vote_options {
                        let result_option = tally_results.get_mut(option);

                        *result_option += voting_power * Decimal256::from(weight.clone());
                    }

                    total_voting_power += voting_power;
                }

                vote_del(ctx, &self.store_key, proposal_id, &voter)?;
            }
        }

        for (
            _,
            ValidatorGovInfo {
                address: _,
                bounded_tokens,
                delegator_shares,
                delegator_deduction,
                vote,
            },
        ) in &curr_validators
        {
            if vote.is_empty() {
                continue;
            }

            let voting_power = (delegator_shares - delegator_deduction)
                * Decimal256::new(bounded_tokens.clone())
                / delegator_shares;

            for VoteOptionWeighted { option, weight } in vote {
                let result = tally_results.get_mut(option);
                *result += voting_power * Decimal256::from(weight.clone());
            }

            total_voting_power += voting_power;
        }

        let tally_params = self.gov_params_keeper.try_get(ctx)?.tally;

        let total_bonded_tokens = self.staking_keeper.total_bonded_tokens(ctx)?;

        // If there is no staked coins, the proposal fails
        if total_bonded_tokens.amount.is_zero() {
            return Ok((false, false, tally_results.result()));
        }

        // If there is not enough quorum of votes, the proposal fails
        let percent_voting = total_voting_power / Decimal256::new(total_bonded_tokens.amount);
        if percent_voting < tally_params.quorum {
            return Ok((false, true, tally_results.result()));
        }

        // If no one votes (everyone abstains), proposal fails
        // Why they sub and check to is_zero in cosmos?
        if total_voting_power == *tally_results.get_mut(&VoteOption::Abstain) {
            return Ok((false, false, tally_results.result()));
        }

        // If more than 1/3 of voters veto, proposal fails
        if *tally_results.get_mut(&VoteOption::NoWithVeto) / total_voting_power
            > tally_params.veto_threshold
        {
            return Ok((false, true, tally_results.result()));
        }

        // If more than 1/2 of non-abstaining voters vote Yes, proposal passes
        if *tally_results.get_mut(&VoteOption::Yes)
            / (total_voting_power - *tally_results.get_mut(&VoteOption::Abstain))
            > tally_params.threshold
        {
            return Ok((true, false, tally_results.result()));
        }

        // If more than 1/2 of non-abstaining voters vote No, proposal fails
        Ok((false, false, tally_results.result()))
    }
}

#[derive(Debug, Clone)]
struct TallyResultMap(HashMap<VoteOption, Decimal256>);

impl TallyResultMap {
    const EXISTS_MSG: &'static str = "guarated to exists";

    pub fn new() -> Self {
        let mut hashmap = HashMap::with_capacity(VoteOption::iter().count());

        for variant in VoteOption::iter() {
            hashmap.insert(variant, Decimal256::zero());
        }

        Self(hashmap)
    }

    pub fn get_mut(&mut self, k: &VoteOption) -> &mut Decimal256 {
        self.0.get_mut(k).expect(Self::EXISTS_MSG)
    }

    pub fn result(mut self) -> TallyResult {
        TallyResult {
            // TODO: is it correct?
            yes: self
                .0
                .remove(&VoteOption::Yes)
                .expect(Self::EXISTS_MSG)
                .to_uint_floor(),
            abstain: self
                .0
                .remove(&VoteOption::Abstain)
                .expect(Self::EXISTS_MSG)
                .to_uint_floor(),
            no: self
                .0
                .remove(&VoteOption::No)
                .expect(Self::EXISTS_MSG)
                .to_uint_floor(),
            no_with_veto: self
                .0
                .remove(&VoteOption::NoWithVeto)
                .expect(Self::EXISTS_MSG)
                .to_uint_floor(),
        }
    }
}

fn proposal_id_get<DB: Database, SK: StoreKey, CTX: QueryableContext<DB, SK>>(
    ctx: &CTX,
    store_key: &SK,
) -> Result<u64, GasStoreErrors> {
    let store = ctx.kv_store(store_key);

    let bytes = store
        .get(PROPOSAL_ID_KEY.as_slice())?
        .expect("Invalid genesis, initial proposal ID hasn't been set");

    Ok(u64::from_be_bytes(
        bytes.try_into().expect("we know it serialized correctly"),
    ))
}

fn proposal_get<DB: Database, SK: StoreKey, CTX: QueryableContext<DB, SK>>(
    ctx: &CTX,
    store_key: &SK,
    proposal_id: u64,
) -> Result<Option<Proposal>, GasStoreErrors> {
    let key = [KEY_PROPOSAL_PREFIX.as_slice(), &proposal_id.to_be_bytes()].concat();

    let store = ctx.kv_store(store_key);

    let bytes = store.get(&key)?;
    match bytes {
        Some(var) => Ok(Some(
            serde_json::from_slice(&var).expect(SERDE_JSON_CONVERSION),
        )),
        None => Ok(None),
    }
}

fn proposal_set<DB: Database, SK: StoreKey, CTX: TransactionalContext<DB, SK>>(
    ctx: &mut CTX,
    store_key: &SK,
    proposal: &Proposal,
) -> Result<(), GasStoreErrors> {
    let mut store = ctx.kv_store_mut(store_key);

    store.set(
        proposal.key(),
        serde_json::to_vec(proposal).expect(SERDE_JSON_CONVERSION),
    )
}

fn proposal_del<DB: Database, SK: StoreKey, CTX: TransactionalContext<DB, SK>>(
    ctx: &mut CTX,
    store_key: &SK,
    proposal_id: u64,
) -> Result<bool, GasStoreErrors> {
    let proposal = proposal_get(ctx, store_key, proposal_id)?;

    if let Some(proposal) = proposal {
        let mut store = ctx.kv_store_mut(store_key);

        store.delete(&Proposal::inactive_queue_key(
            proposal_id,
            &proposal.deposit_end_time,
        ))?;

        store.delete(&Proposal::active_queue_key(
            proposal_id,
            &proposal.deposit_end_time,
        ))?;

        store.delete(&proposal.key())?;

        Ok(true)
    } else {
        Ok(false)
    }
}

fn deposit_get<DB: Database, SK: StoreKey, CTX: QueryableContext<DB, SK>>(
    ctx: &CTX,
    store_key: &SK,
    proposal_id: u64,
    depositor: &AccAddress,
) -> Result<Option<MsgDeposit>, GasStoreErrors> {
    let key = [
        MsgDeposit::KEY_PREFIX.as_slice(),
        &proposal_id.to_be_bytes(),
        &[depositor.len()],
        depositor.as_ref(),
    ]
    .concat();

    let store = ctx.kv_store(store_key);

    let bytes = store.get(&key)?;
    match bytes {
        Some(var) => Ok(Some(
            serde_json::from_slice(&var).expect(SERDE_JSON_CONVERSION),
        )),
        None => Ok(None),
    }
}

fn deposit_set<DB: Database, SK: StoreKey, CTX: TransactionalContext<DB, SK>>(
    ctx: &mut CTX,
    store_key: &SK,
    deposit: &MsgDeposit,
) -> Result<(), GasStoreErrors> {
    let mut store = ctx.kv_store_mut(store_key);

    store.set(
        MsgDeposit::key(deposit.proposal_id, &deposit.depositor),
        serde_json::to_vec(deposit).expect(SERDE_JSON_CONVERSION),
    )
}

fn deposit_del<
    DB: Database,
    SK: StoreKey,
    PSK: ParamsSubspaceKey,
    M: Module,
    BK: BankKeeper<SK, M>,
    STK: GovStakingKeeper<SK, M>,
    CTX: TransactionalContext<DB, SK>,
>(
    ctx: &mut CTX,
    keeper: &GovKeeper<SK, PSK, M, BK, STK>,
    proposal_id: u64,
) -> Result<(), GasStoreErrors> {
    let deposits = DepositIterator::new(ctx.kv_store(&keeper.store_key))
        .map(|this| this.map(|(_, value)| value))
        .collect::<Vec<_>>();

    for deposit in deposits {
        let deposit = deposit?;

        keeper
            .bank_keeper
            .coins_burn(ctx, &keeper.gov_mod, &deposit.amount)
            .expect("Failed to burn coins for gov xmod"); // TODO: how to do this better?

        ctx.kv_store_mut(&keeper.store_key)
            .delete(&MsgDeposit::key(proposal_id, &deposit.depositor))?;
    }

    Ok(())
}

fn deposit_refund<
    DB: Database,
    SK: StoreKey,
    PSK: ParamsSubspaceKey,
    M: Module,
    BK: BankKeeper<SK, M>,
    STK: GovStakingKeeper<SK, M>,
    CTX: TransactionalContext<DB, SK>,
>(
    ctx: &mut CTX,
    keeper: &GovKeeper<SK, PSK, M, BK, STK>,
) -> Result<(), GasStoreErrors> {
    for deposit in
        DepositIterator::new(ctx.kv_store(&keeper.store_key)).map(|this| this.map(|(_, val)| val))
    {
        let deposit = deposit?;

        
    }

    Ok(())
}

fn _vote_get<DB: Database, SK: StoreKey, CTX: QueryableContext<DB, SK>>(
    ctx: &CTX,
    store_key: &SK,
    proposal_id: u64,
    voter: &AccAddress,
) -> Result<Option<MsgVoteWeighted>, GasStoreErrors> {
    let key = [
        MsgVoteWeighted::KEY_PREFIX.as_slice(),
        &proposal_id.to_be_bytes(),
        &[voter.len()],
        voter.as_ref(),
    ]
    .concat();

    let store = ctx.kv_store(store_key);

    let bytes = store.get(&key)?;
    match bytes {
        Some(var) => Ok(Some(
            serde_json::from_slice(&var).expect(SERDE_JSON_CONVERSION),
        )),
        None => Ok(None),
    }
}

fn vote_set<DB: Database, SK: StoreKey, CTX: TransactionalContext<DB, SK>>(
    ctx: &mut CTX,
    store_key: &SK,
    vote: &MsgVoteWeighted,
) -> Result<(), GasStoreErrors> {
    let mut store = ctx.kv_store_mut(store_key);

    store.set(
        MsgVoteWeighted::key(vote.proposal_id, &vote.voter),
        serde_json::to_vec(vote).expect(SERDE_JSON_CONVERSION),
    )
}

fn vote_del<DB: Database, SK: StoreKey, CTX: TransactionalContext<DB, SK>>(
    ctx: &mut CTX,
    store_key: &SK,
    proposal_id: u64,
    voter: &AccAddress,
) -> Result<bool, GasStoreErrors> {
    let mut store = ctx.kv_store_mut(store_key);

    let is_deleted = store.delete(&MsgVoteWeighted::key(proposal_id, &voter))?;

    Ok(is_deleted.is_some())
}
