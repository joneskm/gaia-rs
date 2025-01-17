use std::marker::PhantomData;

use gears::baseapp::QueryResponse;
use gears::extensions::gas::GasResultExt;
use gears::tendermint::request::RequestEndBlock;
use gears::x::keepers::auth::AuthKeeper;
use gears::{
    application::handlers::node::{ABCIHandler, ModuleInfo, TxError},
    baseapp::errors::QueryError,
    context::{
        block::BlockContext, init::InitContext, query::QueryContext, tx::TxContext,
        TransactionalContext,
    },
    core::Protobuf,
    params::ParamsSubspaceKey,
    store::{database::Database, StoreKey},
    tendermint::types::{
        proto::{
            event::{Event, EventAttribute},
            validator::ValidatorUpdate,
        },
        request::query::RequestQuery,
    },
    types::tx::raw::TxWithRaw,
    x::{
        keepers::{gov::GovernanceBankKeeper, staking::GovStakingKeeper},
        module::Module,
    },
};
use serde::de::DeserializeOwned;

use crate::proposal::{Proposal, ProposalHandler};
use crate::{
    errors::GovTxError,
    genesis::GovGenesisState,
    keeper::GovKeeper,
    msg::{deposit::Deposit, GovMsg},
    query::{
        request::{
            QueryAllParamsRequest, QueryDepositRequest, QueryDepositsRequest, QueryParamsRequest,
            QueryProposalRequest, QueryProposalsRequest, QueryProposerRequest,
            QueryTallyResultRequest, QueryVoteRequest, QueryVotesRequest,
        },
        GovQuery, GovQueryResponse,
    },
};

#[derive(Debug, Clone)]
pub struct GovAbciHandler<
    SK: StoreKey,
    PSK: ParamsSubspaceKey,
    M: Module,
    BK: GovernanceBankKeeper<SK, M>,
    AK: AuthKeeper<SK, M>,
    STK: GovStakingKeeper<SK, M>,
    P,
    PH: ProposalHandler<P, SK>,
    MI,
> {
    keeper: GovKeeper<SK, PSK, M, BK, AK, STK, P, PH>,
    _marker: PhantomData<MI>,
}

impl<
        SK: StoreKey,
        PSK: ParamsSubspaceKey,
        M: Module,
        BK: GovernanceBankKeeper<SK, M>,
        AK: AuthKeeper<SK, M>,
        STK: GovStakingKeeper<SK, M>,
        P,
        PH: ProposalHandler<P, SK>,
        MI: ModuleInfo,
    > GovAbciHandler<SK, PSK, M, BK, AK, STK, P, PH, MI>
{
    pub fn new(keeper: GovKeeper<SK, PSK, M, BK, AK, STK, P, PH>) -> Self {
        Self {
            keeper,
            _marker: PhantomData,
        }
    }
}

impl<
        SK: StoreKey,
        PSK: ParamsSubspaceKey,
        M: Module,
        BK: GovernanceBankKeeper<SK, M>,
        AK: AuthKeeper<SK, M>,
        STK: GovStakingKeeper<SK, M>,
        P: Proposal + DeserializeOwned,
        PH: ProposalHandler<P, SK> + Clone + Send + Sync + 'static,
        MI: ModuleInfo + Clone + Send + Sync + 'static,
    > ABCIHandler for GovAbciHandler<SK, PSK, M, BK, AK, STK, P, PH, MI>
{
    type Message = GovMsg;

    type Genesis = GovGenesisState<P>;

    type StoreKey = SK;

    type QReq = GovQuery;

    type QRes = GovQueryResponse<P>;

    fn typed_query<DB: Database>(
        &self,
        ctx: &QueryContext<DB, Self::StoreKey>,
        query: Self::QReq,
    ) -> Self::QRes {
        self.keeper.query(ctx, query).unwrap_gas()
    }

    fn run_ante_checks<DB: Database>(
        &self,
        _ctx: &mut TxContext<'_, DB, Self::StoreKey>,
        _tx: &TxWithRaw<Self::Message>,
        _: bool,
    ) -> Result<(), TxError> {
        Ok(())
    }

    fn msg<DB: Database>(
        &self,
        ctx: &mut TxContext<'_, DB, Self::StoreKey>,
        msg: &Self::Message,
    ) -> Result<(), TxError> {
        enum EmitEvent {
            Regular,
            Deposit(u64),
            Proposal((String, Option<u64>)),
        }

        let (address_str, proposal) = match msg {
            GovMsg::Deposit(msg) => {
                self.keeper
                    .deposit_add(ctx, msg.clone())
                    .map(|is_voting_started| match is_voting_started {
                        true => (
                            msg.depositor.to_string(),
                            EmitEvent::Deposit(msg.proposal_id),
                        ),
                        false => (msg.depositor.to_string(), EmitEvent::Regular),
                    })
            }
            GovMsg::Vote(msg) => self
                .keeper
                .vote_add(ctx, msg.clone().into())
                .map(|_| (msg.voter.to_string(), EmitEvent::Regular)),
            GovMsg::Weighted(msg) => self
                .keeper
                .vote_add(ctx, msg.clone())
                .map(|_| (msg.voter.to_string(), EmitEvent::Regular)),
            GovMsg::Proposal(msg) => {
                let proposal_id = self
                    .keeper
                    .submit_proposal(ctx, msg.clone())
                    .map_err(GovTxError::from)
                    .map_err(|e| e.into::<MI>())?;

                self.keeper
                    .deposit_add(
                        ctx,
                        Deposit {
                            proposal_id,
                            depositor: msg.proposer.clone(),
                            amount: msg.initial_deposit.clone(),
                        },
                    )
                    .map(|is_voting_started| match is_voting_started {
                        true => (
                            msg.proposer.to_string(),
                            EmitEvent::Proposal((msg.content.type_url.clone(), Some(proposal_id))),
                        ),
                        false => (
                            msg.proposer.to_string(),
                            EmitEvent::Proposal((msg.content.type_url.clone(), None)),
                        ),
                    })
            }
        }
        .map_err(GovTxError::from)
        .map_err(|e| e.into::<MI>())?;

        ctx.push_event(Event::new(
            "message",
            vec![
                EventAttribute::new("module".into(), "governance".into(), false),
                EventAttribute::new("sender".into(), address_str.into(), false),
            ],
        ));

        match proposal {
            EmitEvent::Regular => (),
            EmitEvent::Deposit(proposal) => {
                ctx.push_event(Event::new(
                    "proposal_deposit",
                    vec![EventAttribute::new(
                        "voting_period_start".into(),
                        proposal.to_string().into(),
                        false,
                    )],
                ));
            }
            EmitEvent::Proposal((proposal_type, proposal)) => {
                ctx.push_event(Event::new(
                    "submit_proposal",
                    match proposal {
                        Some(proposal_id) => vec![
                            EventAttribute::new(
                                "proposal_type".into(),
                                proposal_type.into(),
                                false,
                            ),
                            EventAttribute::new(
                                "voting_period_start".into(),
                                proposal_id.to_string().into(),
                                false,
                            ),
                        ],
                        None => vec![EventAttribute::new(
                            "proposal_type".into(),
                            proposal_type.into(),
                            false,
                        )],
                    },
                ));
            }
        }

        Ok(())
    }

    fn init_genesis<DB: Database>(
        &self,
        ctx: &mut InitContext<'_, DB, Self::StoreKey>,
        genesis: Self::Genesis,
    ) -> Vec<ValidatorUpdate> {
        self.keeper.init_genesis(ctx, genesis);
        vec![]
    }

    fn query<DB: Database>(
        &self,
        ctx: &QueryContext<DB, Self::StoreKey>,
        RequestQuery {
            data,
            path,
            height: _,
            prove: _,
        }: RequestQuery,
    ) -> Result<Vec<u8>, QueryError> {
        let query = match path.as_str() {
            QueryDepositRequest::QUERY_URL => GovQuery::Deposit(QueryDepositRequest::decode(data)?),
            QueryDepositsRequest::QUERY_URL => {
                GovQuery::Deposits(QueryDepositsRequest::decode(data)?)
            }
            QueryParamsRequest::QUERY_URL => GovQuery::Params(QueryParamsRequest::decode(data)?),
            QueryAllParamsRequest::QUERY_URL => {
                GovQuery::AllParams(QueryAllParamsRequest::decode(data)?)
            }
            QueryProposalRequest::QUERY_URL => {
                GovQuery::Proposal(QueryProposalRequest::decode(data)?)
            }
            QueryProposalsRequest::QUERY_URL => {
                GovQuery::Proposals(QueryProposalsRequest::decode(data)?)
            }
            QueryTallyResultRequest::QUERY_URL => {
                GovQuery::Tally(QueryTallyResultRequest::decode(data)?)
            }
            QueryVoteRequest::QUERY_URL => GovQuery::Vote(QueryVoteRequest::decode(data)?),
            QueryVotesRequest::QUERY_URL => GovQuery::Votes(QueryVotesRequest::decode(data)?),
            QueryProposerRequest::QUERY_URL => {
                GovQuery::Proposer(QueryProposerRequest::decode(data)?)
            }
            _ => Err(QueryError::PathNotFound)?,
        };

        let result = self.keeper.query(ctx, query).unwrap_gas();

        Ok(result.into_bytes())
    }

    fn end_block<'a, DB: Database>(
        &self,
        ctx: &mut BlockContext<'_, DB, Self::StoreKey>,
        _request: RequestEndBlock,
    ) -> Vec<ValidatorUpdate> {
        let events = self.keeper.end_block(ctx);

        ctx.append_events(events);

        Vec::new()
    }
}
