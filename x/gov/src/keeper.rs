use std::{collections::HashMap, marker::PhantomData, ops::Mul};

use gears::core::errors::CoreError;
use gears::extensions::gas::GasResultExt;
use gears::gas::store::errors::GasStoreErrors;
use gears::x::keepers::auth::AuthKeeper;
use gears::{
    application::keepers::params::ParamsKeeper,
    context::{
        block::BlockContext, init::InitContext, tx::TxContext, QueryableContext,
        TransactionalContext,
    },
    params::ParamsSubspaceKey,
    store::{database::Database, StoreKey},
    tendermint::types::proto::event::{Event, EventAttribute},
    types::{
        address::{AccAddress, ValAddress},
        decimal256::Decimal256,
    },
    x::{
        keepers::{gov::GovernanceBankKeeper, staking::GovStakingKeeper},
        module::Module,
        types::{delegation::StakingDelegation, validator::StakingValidator},
    },
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use strum::IntoEnumIterator;

use crate::proposal::{Proposal, ProposalHandler};
use crate::{
    errors::{GovKeeperError, TallyError, SERDE_JSON_CONVERSION},
    genesis::GovGenesisState,
    msg::{
        deposit::Deposit,
        proposal::MsgSubmitProposal,
        vote::VoteOption,
        weighted_vote::{MsgVoteWeighted, VoteOptionWeighted},
    },
    params::GovParamsKeeper,
    query::{
        request::{
            ParamsQuery, QueryDepositRequest, QueryDepositsRequest, QueryParamsRequest,
            QueryProposalRequest, QueryProposalsRequest, QueryProposerRequest,
            QueryTallyResultRequest, QueryVoteRequest, QueryVotesRequest,
        },
        response::{
            QueryAllParamsResponse, QueryDepositResponse, QueryDepositsResponse,
            QueryParamsResponse, QueryProposalResponse, QueryProposalsResponse,
            QueryTallyResultResponse, QueryVoteResponse, QueryVotesResponse,
        },
        GovQuery, GovQueryResponse,
    },
    types::{
        deposit_iter::DepositIterator,
        proposal::{
            active_iter::ActiveProposalIterator, inactive_iter::InactiveProposalIterator,
            ProposalModel, ProposalStatus, ProposalsIterator, TallyResult,
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
    BK: GovernanceBankKeeper<SK, M>,
    AK: AuthKeeper<SK, M>,
    STK: GovStakingKeeper<SK, M>,
    P,
    PH: ProposalHandler<P, SK>,
> {
    store_key: SK,
    gov_params_keeper: GovParamsKeeper<PSK>,
    gov_mod: M,
    bank_keeper: BK,
    auth_keeper: AK,
    staking_keeper: STK,
    proposal_handler: PH,

    _marker: PhantomData<(P, BK)>,
}

impl<
        SK: StoreKey,
        PSK: ParamsSubspaceKey,
        M: Module,
        BK: GovernanceBankKeeper<SK, M>,
        AK: AuthKeeper<SK, M>,
        STK: GovStakingKeeper<SK, M>,
        P: Proposal + DeserializeOwned,
        PH: ProposalHandler<P, SK>,
    > GovKeeper<SK, PSK, M, BK, AK, STK, P, PH>
{
    pub fn new(
        store_key: SK,
        params_subspace_key: PSK,
        gov_mod: M,
        bank_keeper: BK,
        auth_keeper: AK,
        staking_keeper: STK,
        proposal_handler: PH,
    ) -> Self {
        Self {
            store_key,
            gov_params_keeper: GovParamsKeeper {
                params_subspace_key,
            },
            gov_mod,
            bank_keeper,
            auth_keeper,
            staking_keeper,
            proposal_handler,
            _marker: PhantomData,
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
        }: GovGenesisState<P>,
    ) {
        {
            let mut store = ctx.kv_store_mut(&self.store_key);
            store.set(PROPOSAL_ID_KEY, starting_proposal_id.to_be_bytes())
        }
        self.gov_params_keeper.set(ctx, params);

        self.auth_keeper
            .check_create_new_module_account(ctx, &self.gov_mod)
            .unwrap_gas();

        let total_deposits = {
            let mut store_mut = ctx.kv_store_mut(&self.store_key);

            let total_deposits = {
                let mut total_deposits = Vec::with_capacity(deposits.len());
                for deposit in deposits {
                    store_mut.set(
                        Deposit::key(deposit.proposal_id, &deposit.depositor),
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
                            ProposalModel::<P>::inactive_queue_key(
                                proposal.proposal_id,
                                &proposal.deposit_end_time,
                            ),
                            proposal.proposal_id.to_be_bytes(),
                        );
                    }
                    ProposalStatus::VotingPeriod => store_mut.set(
                        ProposalModel::<P>::active_queue_key(
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
            .balance_all(ctx, self.gov_mod.address(), None)
            .unwrap_gas()
            .1;
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

    pub fn query<CTX: QueryableContext<DB, SK>, DB: Database>(
        &self,
        ctx: &CTX,
        query: GovQuery,
    ) -> Result<GovQueryResponse<P>, GasStoreErrors> {
        let result = match query {
            GovQuery::Deposit(QueryDepositRequest {
                proposal_id,
                depositor,
            }) => GovQueryResponse::Deposit(QueryDepositResponse {
                deposit: deposit_get(ctx, &self.store_key, proposal_id, &depositor)?,
            }),
            GovQuery::Deposits(QueryDepositsRequest {
                proposal_id,
                pagination: _,
            }) => {
                let deposits = DepositIterator::new(ctx.kv_store(&self.store_key))
                    .map(|this| this.map(|(_key, value)| value))
                    .filter_map(|this| this.ok())
                    .filter(|this| this.proposal_id == proposal_id)
                    .collect::<Vec<_>>();

                GovQueryResponse::Deposits(QueryDepositsResponse {
                    deposits,
                    pagination: None,
                })
            }
            GovQuery::Params(QueryParamsRequest { kind }) => {
                let params = self.gov_params_keeper.try_get(ctx)?;

                let result = match kind {
                    ParamsQuery::Voting => QueryParamsResponse {
                        voting_params: Some(params.voting),
                        deposit_params: None,
                        tally_params: None,
                    },
                    ParamsQuery::Deposit => QueryParamsResponse {
                        voting_params: None,
                        deposit_params: Some(params.deposit),
                        tally_params: None,
                    },
                    ParamsQuery::Tally => QueryParamsResponse {
                        voting_params: None,
                        deposit_params: None,
                        tally_params: Some(params.tally),
                    },
                };

                GovQueryResponse::Params(result)
            }
            GovQuery::AllParams(_) => {
                let params = self.gov_params_keeper.try_get(ctx)?;

                GovQueryResponse::AllParams(QueryAllParamsResponse {
                    voting_params: params.voting,
                    deposit_params: params.deposit,
                    tally_params: params.tally,
                })
            }
            GovQuery::Proposal(QueryProposalRequest { proposal_id }) => {
                GovQueryResponse::Proposal(QueryProposalResponse {
                    proposal: proposal_get(ctx, &self.store_key, proposal_id)?,
                })
            }
            GovQuery::Proposals(QueryProposalsRequest {
                voter,
                depositor,
                proposal_status,
                pagination: _,
            }) => {
                let iterator = ProposalsIterator::new(ctx.kv_store(&self.store_key))
                    .map(|this| this.map(|(_key, value)| value))
                    .filter_map(|this| this.ok());

                let mut proposals = Vec::new();
                for proposal in iterator {
                    if let Some(voter) = &voter {
                        let vote = vote_get(ctx, &self.store_key, proposal.proposal_id, voter)?;
                        if vote.is_none() {
                            continue;
                        }
                    }

                    if let Some(depositor) = &depositor {
                        let deposit =
                            deposit_get(ctx, &self.store_key, proposal.proposal_id, depositor)?;
                        if deposit.is_none() {
                            continue;
                        }
                    }

                    if let Some(proposal_status) = proposal_status {
                        if proposal.status != proposal_status {
                            continue;
                        }
                    }

                    proposals.push(proposal);
                }

                GovQueryResponse::Proposals(QueryProposalsResponse {
                    proposals,
                    pagination: None,
                })
            }
            GovQuery::Tally(QueryTallyResultRequest { proposal_id }) => {
                let proposal = proposal_get::<_, _, _, P>(ctx, &self.store_key, proposal_id)?;

                GovQueryResponse::Tally(QueryTallyResultResponse {
                    tally: proposal.and_then(|this| this.final_tally_result),
                })
            }
            GovQuery::Vote(QueryVoteRequest { proposal_id, voter }) => {
                GovQueryResponse::Vote(QueryVoteResponse {
                    vote: vote_get(ctx, &self.store_key, proposal_id, &voter)?,
                })
            }
            GovQuery::Votes(QueryVotesRequest {
                proposal_id,
                pagination: _,
            }) => {
                let votes = WeightedVoteIterator::new(ctx.kv_store(&self.store_key), proposal_id)
                    .map(|this| this.map(|(_key, value)| value))
                    .filter_map(|this| this.ok())
                    .collect::<Vec<_>>();

                GovQueryResponse::Votes(QueryVotesResponse {
                    votes,
                    pagination: None,
                })
            }
            GovQuery::Proposer(QueryProposerRequest { proposal_id: _ }) => unimplemented!(), // TODO:NOW I couldn't find where this query handles or what method
        };

        Ok(result)
    }

    pub fn deposit_add<DB: Database>(
        &self,
        ctx: &mut TxContext<'_, DB, SK>,
        Deposit {
            proposal_id,
            depositor,
            amount,
        }: Deposit,
    ) -> Result<bool, GovKeeperError> {
        let mut proposal = proposal_get::<_, _, _, P>(ctx, &self.store_key, proposal_id)?
            .ok_or(GovKeeperError::ProposalUnknown(proposal_id))?;

        match proposal.status {
            ProposalStatus::DepositPeriod | ProposalStatus::VotingPeriod => Ok(()),
            _ => Err(GovKeeperError::InactiveProposal(proposal_id)),
        }?;

        self.bank_keeper.send_coins_from_account_to_module(
            ctx,
            depositor.clone(),
            &self.gov_mod,
            amount.clone(),
        )?;

        proposal.total_deposit = proposal.total_deposit.checked_add(&amount)?;
        proposal_set(ctx, &self.store_key, &proposal)?;

        let deposit_params = self.gov_params_keeper.try_get(ctx)?.deposit;

        let activated_voting_period = matches!(proposal.status, ProposalStatus::DepositPeriod if proposal
                   .total_deposit
                   .is_all_gte(Vec::from(deposit_params.min_deposit.clone()).iter()));

        let deposit = match deposit_get(ctx, &self.store_key, proposal_id, &depositor)? {
            Some(mut deposit) => {
                deposit.amount = deposit.amount.checked_add(&amount)?;
                deposit
            }
            None => Deposit {
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
    ) -> Result<(), GovKeeperError> {
        let proposal = proposal_get::<_, _, _, P>(ctx, &self.store_key, vote.proposal_id)?
            .ok_or(GovKeeperError::ProposalUnknown(vote.proposal_id))?;

        match proposal.status {
            ProposalStatus::VotingPeriod => Ok(()),
            _ => Err(GovKeeperError::InactiveProposal(vote.proposal_id)),
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
    ) -> Result<u64, GovKeeperError> {
        let proposal_id = proposal_id_get(ctx, &self.store_key)?;
        let submit_time = ctx.header().time;
        let deposit_period = self
            .gov_params_keeper
            .try_get(ctx)?
            .deposit
            .max_deposit_period;

        let proposal = ProposalModel {
            proposal_id,
            content: content
                .try_into()
                .map_err(|e: CoreError| GovKeeperError::Custom(e.to_string()))?, // TODO: Better way. Generic or smth else
            status: ProposalStatus::DepositPeriod,
            final_tally_result: None,
            submit_time,
            deposit_end_time: submit_time
                .checked_add(deposit_period)
                .ok_or(GovKeeperError::Time("Deposit end time overflow".to_owned()))?,
            total_deposit: initial_deposit,
            voting_start_time: None,
            voting_end_time: None,
        };

        if !PH::check(&proposal.content) {
            return Err(GovKeeperError::NoHandler);
        }

        proposal_set(ctx, &self.store_key, &proposal)?;
        let mut store = ctx.kv_store_mut(&self.store_key);

        store.set(
            ProposalModel::<P>::inactive_queue_key(
                proposal.proposal_id,
                &proposal.deposit_end_time,
            ),
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

        {
            let inactive_iter = {
                let store = ctx.kv_store(&self.store_key);
                InactiveProposalIterator::<'_, _, P>::new(store.into(), &ctx.header.time)
                    .map(|this| this.map(|((proposal_id, _), _)| proposal_id))
                    .collect::<Vec<_>>()
            };

            for var in inactive_iter {
                let proposal_id = var.unwrap_gas();
                proposal_del::<_, _, _, P>(ctx, &self.store_key, proposal_id).unwrap_gas();
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
                ActiveProposalIterator::new(store, &ctx.header.time)
                    .map(|this| this.map(|((_, _), proposal)| proposal))
                    .collect::<Vec<_>>()
            };

            for proposal in active_iter {
                let mut proposal: ProposalModel<P> = proposal.unwrap_gas();

                let (passes, burn_deposit, tally_result) =
                    match self.tally(ctx, proposal.proposal_id) {
                        Ok(var) => var,
                        Err(err) => match err {
                            TallyError::Gas(_) => unreachable!("block ctx doesn't have any gas"),
                            TallyError::Math(e) => panic!("Failed to get tally: {e}"),
                        },
                    };

                if burn_deposit {
                    deposit_del(ctx, self, proposal.proposal_id).unwrap_gas();
                } else {
                    deposit_refund(ctx, self).unwrap_gas();
                }

                match passes {
                    true if self
                        .proposal_handler
                        .handle(proposal.content.clone(), ctx)
                        .is_ok() =>
                    {
                        proposal.status = ProposalStatus::Passed
                    }
                    true => proposal.status = ProposalStatus::Failed,
                    false => proposal.status = ProposalStatus::Rejected,
                }

                proposal.final_tally_result = Some(tally_result);

                proposal_set(ctx, &self.store_key, &proposal).unwrap_gas();
                ctx.kv_store_mut(&self.store_key)
                    .delete(&ProposalModel::<P>::active_queue_key(
                        proposal.proposal_id,
                        &proposal.deposit_end_time,
                    ));

                // TODO: HOOKS https://github.com/cosmos/cosmos-sdk/blob/d3f09c222243bb3da3464969f0366330dcb977a8/x/gov/abci.go#L97

                events.push(Event::new(
                    "active_proposal",
                    vec![
                        EventAttribute::new(
                            "proposal_id".into(),
                            proposal.proposal_id.to_string().into(),
                            false,
                        ),
                        EventAttribute::new("proposal_result".into(), "TODO".into(), false),
                    ],
                ))
            }
        }

        events
    }

    fn tally<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        proposal_id: u64,
    ) -> Result<(bool, bool, TallyResult), TallyError> {
        let mut curr_validators = HashMap::<ValAddress, ValidatorGovInfo>::new();

        for validator in self.staking_keeper.bonded_validators_by_power_iter(ctx)? {
            let validator = validator?;

            curr_validators.insert(
                validator.operator().clone(),
                ValidatorGovInfo {
                    address: validator.operator().clone(),
                    bounded_tokens: validator.bonded_tokens(),
                    delegator_shares: validator.delegator_shares(),
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

                    validator.delegator_deduction = validator
                        .delegator_deduction
                        .checked_add(*delegation.shares())
                        .map_err(|e| {
                            TallyError::Math(format!("Delegator deduction overflow: {e}"))
                        })?;

                    // delegation shares * bonded / total shares
                    let voting_power = delegation
                        .shares()
                        .mul(
                            Decimal256::from_atomics(validator.bounded_tokens, 0).map_err(|e| {
                                TallyError::Math(format!(
                                    "Decimal overflow while calculating voting power: {e}"
                                ))
                            })?,
                        )
                        .checked_div(validator.delegator_shares)
                        .map_err(|e| TallyError::Math(format!("Div on voting power: {e}")))?;

                    for VoteOptionWeighted { option, weight } in &vote_options {
                        let result_option = tally_results.get_mut(option);

                        *result_option += voting_power * Decimal256::from(weight.clone());
                    }

                    total_voting_power += voting_power;
                }

                vote_del(ctx, &self.store_key, proposal_id, &voter)?;
            }
        }

        for ValidatorGovInfo {
            address: _,
            bounded_tokens,
            delegator_shares,
            delegator_deduction,
            vote,
        } in curr_validators.values()
        {
            if vote.is_empty() {
                continue;
            }

            let voting_power = (delegator_shares - delegator_deduction)
                * Decimal256::from_atomics(*bounded_tokens, 0).unwrap() // TODO: HANDLE THIS
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
            return Ok((false, false, tally_results.into_result()));
        }

        // If there is not enough quorum of votes, the proposal fails
        let percent_voting =
            total_voting_power / Decimal256::from_atomics(total_bonded_tokens.amount, 0).unwrap(); // TODO: HANDLE THIS
        if percent_voting < tally_params.quorum {
            return Ok((false, true, tally_results.into_result()));
        }

        // If no one votes (everyone abstains), proposal fails
        // Why they sub and check to is_zero in cosmos?
        if total_voting_power == *tally_results.get_mut(&VoteOption::Abstain) {
            return Ok((false, false, tally_results.into_result()));
        }

        // If more than 1/3 of voters veto, proposal fails
        if *tally_results.get_mut(&VoteOption::NoWithVeto) / total_voting_power
            > tally_params.veto_threshold
        {
            return Ok((false, true, tally_results.into_result()));
        }

        // If more than 1/2 of non-abstaining voters vote Yes, proposal passes
        if *tally_results.get_mut(&VoteOption::Yes)
            / (total_voting_power - *tally_results.get_mut(&VoteOption::Abstain))
            > tally_params.threshold
        {
            return Ok((true, false, tally_results.into_result()));
        }

        // If more than 1/2 of non-abstaining voters vote No, proposal fails
        Ok((false, false, tally_results.into_result()))
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

    pub fn into_result(mut self) -> TallyResult {
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

fn proposal_get<DB: Database, SK: StoreKey, CTX: QueryableContext<DB, SK>, P: DeserializeOwned>(
    ctx: &CTX,
    store_key: &SK,
    proposal_id: u64,
) -> Result<Option<ProposalModel<P>>, GasStoreErrors> {
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

fn proposal_set<
    DB: Database,
    SK: StoreKey,
    CTX: TransactionalContext<DB, SK>,
    P: DeserializeOwned + Serialize,
>(
    ctx: &mut CTX,
    store_key: &SK,
    proposal: &ProposalModel<P>,
) -> Result<(), GasStoreErrors> {
    let mut store = ctx.kv_store_mut(store_key);

    store.set(
        proposal.key(),
        serde_json::to_vec(proposal).expect(SERDE_JSON_CONVERSION),
    )
}

fn proposal_del<
    DB: Database,
    SK: StoreKey,
    CTX: TransactionalContext<DB, SK>,
    P: DeserializeOwned,
>(
    ctx: &mut CTX,
    store_key: &SK,
    proposal_id: u64,
) -> Result<bool, GasStoreErrors> {
    let proposal = proposal_get::<_, _, _, P>(ctx, store_key, proposal_id)?;

    if let Some(proposal) = proposal {
        let mut store = ctx.kv_store_mut(store_key);

        store.delete(&ProposalModel::<P>::inactive_queue_key(
            proposal_id,
            &proposal.deposit_end_time,
        ))?;

        store.delete(&ProposalModel::<P>::active_queue_key(
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
) -> Result<Option<Deposit>, GasStoreErrors> {
    let key = [
        Deposit::KEY_PREFIX.as_slice(),
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
    deposit: &Deposit,
) -> Result<(), GasStoreErrors> {
    let mut store = ctx.kv_store_mut(store_key);

    store.set(
        Deposit::key(deposit.proposal_id, &deposit.depositor),
        serde_json::to_vec(deposit).expect(SERDE_JSON_CONVERSION),
    )
}

fn deposit_del<
    DB: Database,
    SK: StoreKey,
    PSK: ParamsSubspaceKey,
    M: Module,
    BK: GovernanceBankKeeper<SK, M>,
    AK: AuthKeeper<SK, M>,
    STK: GovStakingKeeper<SK, M>,
    CTX: TransactionalContext<DB, SK>,
    P: Proposal,
    PH: ProposalHandler<P, SK>,
>(
    ctx: &mut CTX,
    keeper: &GovKeeper<SK, PSK, M, BK, AK, STK, P, PH>,
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
            .delete(&Deposit::key(proposal_id, &deposit.depositor))?;
    }

    Ok(())
}

fn deposit_refund<
    DB: Database,
    SK: StoreKey,
    PSK: ParamsSubspaceKey,
    M: Module,
    BK: GovernanceBankKeeper<SK, M>,
    AK: AuthKeeper<SK, M>,
    STK: GovStakingKeeper<SK, M>,
    CTX: TransactionalContext<DB, SK>,
    P: Proposal,
    PH: ProposalHandler<P, SK>,
>(
    ctx: &mut CTX,
    keeper: &GovKeeper<SK, PSK, M, BK, AK, STK, P, PH>,
) -> Result<(), GasStoreErrors> {
    for deposit in DepositIterator::new(ctx.kv_store(&keeper.store_key))
        .map(|this| this.map(|(_, val)| val))
        .collect::<Vec<_>>()
    {
        let Deposit {
            proposal_id,
            depositor,
            amount,
        } = deposit?;

        keeper
            .bank_keeper
            .send_coins_from_module_to_account(ctx, &depositor, &keeper.gov_mod, amount)
            .expect("Failed to refund coins"); // TODO: how to do this better?

        ctx.kv_store_mut(&keeper.store_key)
            .delete(&Deposit::key(proposal_id, &depositor))?;
    }

    Ok(())
}

fn vote_get<DB: Database, SK: StoreKey, CTX: QueryableContext<DB, SK>>(
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

    let is_deleted = store.delete(&MsgVoteWeighted::key(proposal_id, voter))?;

    Ok(is_deleted.is_some())
}
