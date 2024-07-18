use bytes::Bytes;
use gears::{
    application::handlers::node::{ABCIHandler, TxError},
    baseapp::errors::QueryError,
    context::{
        block::BlockContext, init::InitContext, query::QueryContext, tx::TxContext,
        TransactionalContext,
    },
    params::ParamsSubspaceKey,
    store::{database::Database, StoreKey},
    tendermint::types::{
        proto::{
            event::{Event, EventAttribute},
            validator::ValidatorUpdate,
            Protobuf,
        },
        request::{end_block::RequestEndBlock, query::RequestQuery},
    },
    types::{store::gas::ext::GasResultExt, tx::raw::TxWithRaw},
    x::{
        keepers::{gov::GovernanceBankKeeper, staking::GovStakingKeeper},
        module::Module,
    },
};

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
    types::proposal::Proposal,
    ProposalHandler,
};

#[derive(Debug, Clone)]
pub struct GovAbciHandler<
    SK: StoreKey,
    PSK: ParamsSubspaceKey,
    M: Module,
    BK: GovernanceBankKeeper<SK, M>,
    STK: GovStakingKeeper<SK, M>,
    PH: ProposalHandler<PSK, Proposal>,
> {
    keeper: GovKeeper<SK, PSK, M, BK, STK, PH>,
}

impl<
        SK: StoreKey,
        PSK: ParamsSubspaceKey,
        M: Module,
        BK: GovernanceBankKeeper<SK, M>,
        STK: GovStakingKeeper<SK, M>,
        PH: ProposalHandler<PSK, Proposal>,
    > GovAbciHandler<SK, PSK, M, BK, STK, PH>
{
    pub fn new(keeper: GovKeeper<SK, PSK, M, BK, STK, PH>) -> Self {
        Self { keeper }
    }
}

impl<
        SK: StoreKey,
        PSK: ParamsSubspaceKey,
        M: Module,
        BK: GovernanceBankKeeper<SK, M>,
        STK: GovStakingKeeper<SK, M>,
        PH: ProposalHandler<PSK, Proposal> + Clone + Send + Sync + 'static,
    > ABCIHandler for GovAbciHandler<SK, PSK, M, BK, STK, PH>
{
    type Message = GovMsg;

    type Genesis = GovGenesisState;

    type StoreKey = SK;

    type QReq = GovQuery;

    type QRes = GovQueryResponse;

    fn typed_query<DB: Database>(
        &self,
        _ctx: &QueryContext<DB, Self::StoreKey>,
        _query: Self::QReq,
    ) -> Self::QRes {
        todo!()
    }

    fn run_ante_checks<DB: Database>(
        &self,
        _ctx: &mut TxContext<'_, DB, Self::StoreKey>,
        _tx: &TxWithRaw<Self::Message>,
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
                let is_voting_started = self
                    .keeper
                    .deposit_add(ctx, msg.clone())
                    .map_err(|e| Into::<GovTxError>::into(e))?;

                match is_voting_started {
                    true => (
                        msg.depositor.to_string(),
                        EmitEvent::Deposit(msg.proposal_id),
                    ),
                    false => (msg.depositor.to_string(), EmitEvent::Regular),
                }
            }
            GovMsg::Vote(msg) => {
                self.keeper
                    .vote_add(ctx, msg.clone().into())
                    .map_err(|e| Into::<GovTxError>::into(e))?;

                (msg.voter.to_string(), EmitEvent::Regular)
            }
            GovMsg::Weighted(msg) => {
                self.keeper
                    .vote_add(ctx, msg.clone())
                    .map_err(|e| Into::<GovTxError>::into(e))?;

                (msg.voter.to_string(), EmitEvent::Regular)
            }
            GovMsg::Proposal(msg) => {
                let proposal_id = self
                    .keeper
                    .submit_proposal(ctx, msg.clone())
                    .map_err(|e| Into::<GovTxError>::into(e))?;

                let is_voting_started = self
                    .keeper
                    .deposit_add(
                        ctx,
                        Deposit {
                            proposal_id,
                            depositor: msg.proposer.clone(),
                            amount: msg.initial_deposit.clone(),
                        },
                    )
                    .map_err(|e| Into::<GovTxError>::into(e))?;

                match is_voting_started {
                    true => (
                        msg.proposer.to_string(),
                        EmitEvent::Proposal((msg.content.type_url.clone(), Some(proposal_id))),
                    ),
                    false => (
                        msg.proposer.to_string(),
                        EmitEvent::Proposal((msg.content.type_url.clone(), None)),
                    ),
                }
            }
        };

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
    ) {
        self.keeper.init_genesis(ctx, genesis)
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
    ) -> Result<Bytes, QueryError> {
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

        Ok(result.encode_to_vec())
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
