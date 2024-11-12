use crate::{
    error::StakingTxError, GenesisState, Keeper, Message, QueryDelegationRequest,
    QueryDelegationResponse, QueryDelegatorDelegationsRequest, QueryDelegatorDelegationsResponse,
    QueryDelegatorUnbondingDelegationsRequest, QueryDelegatorUnbondingDelegationsResponse,
    QueryParamsRequest, QueryParamsResponse, QueryPoolRequest, QueryPoolResponse,
    QueryRedelegationsRequest, QueryRedelegationsResponse, QueryUnbondingDelegationRequest,
    QueryUnbondingDelegationResponse, QueryValidatorRequest, QueryValidatorResponse,
    QueryValidatorsRequest, QueryValidatorsResponse, Redelegation, RedelegationEntryResponse,
    RedelegationResponse,
};
use crate::{
    QueryDelegatorValidatorRequest, QueryDelegatorValidatorResponse,
    QueryDelegatorValidatorsRequest, QueryDelegatorValidatorsResponse, QueryHistoricalInfoRequest,
    QueryHistoricalInfoResponse, QueryValidatorDelegationsRequest,
    QueryValidatorDelegationsResponse, QueryValidatorUnbondingDelegationsRequest,
    QueryValidatorUnbondingDelegationsResponse,
};
use gears::{
    application::handlers::node::{ABCIHandler, ModuleInfo, TxError},
    baseapp::{errors::QueryError, QueryRequest, QueryResponse},
    context::{block::BlockContext, init::InitContext, query::QueryContext, tx::TxContext},
    core::Protobuf,
    derive::Query,
    extensions::{
        gas::GasResultExt,
        pagination::{IteratorPaginate, Pagination},
    },
    params::ParamsSubspaceKey,
    store::{database::Database, StoreKey},
    tendermint::{
        request::{RequestBeginBlock, RequestEndBlock},
        types::{proto::validator::ValidatorUpdate, request::query::RequestQuery},
    },
    types::pagination::response::PaginationResponse,
    x::{
        keepers::{
            auth::AuthKeeper,
            staking::{KeeperHooks, StakingBankKeeper},
        },
        module::Module,
    },
};
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct StakingABCIHandler<
    SK: StoreKey,
    PSK: ParamsSubspaceKey,
    AK: AuthKeeper<SK, M>,
    BK: StakingBankKeeper<SK, M>,
    KH: KeeperHooks<SK, AK, M>,
    M: Module,
    MI: ModuleInfo,
> {
    keeper: Keeper<SK, PSK, AK, BK, KH, M>,
    phantom_data: std::marker::PhantomData<MI>,
}

#[derive(Clone, Debug)]
pub enum StakingNodeQueryRequest {
    Validator(QueryValidatorRequest),
    Validators(QueryValidatorsRequest),
    ValidatorDelegations(QueryValidatorDelegationsRequest),
    ValidatorUnbondingDelegations(QueryValidatorUnbondingDelegationsRequest),
    Delegation(QueryDelegationRequest),
    Delegations(QueryDelegatorDelegationsRequest),
    UnbondingDelegation(QueryUnbondingDelegationRequest),
    UnbondingDelegations(QueryDelegatorUnbondingDelegationsRequest),
    DelegatorValidator(QueryDelegatorValidatorRequest),
    Redelegations(QueryRedelegationsRequest),
    DelegatorValidators(QueryDelegatorValidatorsRequest),
    HistoricalInfo(QueryHistoricalInfoRequest),
    Pool(QueryPoolRequest),
    Params(QueryParamsRequest),
}

impl QueryRequest for StakingNodeQueryRequest {
    fn height(&self) -> u32 {
        todo!()
    }
}

#[derive(Clone, Debug, Serialize, Query)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum StakingNodeQueryResponse {
    Validator(QueryValidatorResponse),
    Validators(QueryValidatorsResponse),
    ValidatorDelegations(QueryValidatorDelegationsResponse),
    ValidatorUnbondingDelegations(QueryValidatorUnbondingDelegationsResponse),
    Delegation(QueryDelegationResponse),
    Delegations(QueryDelegatorDelegationsResponse),
    UnbondingDelegation(QueryUnbondingDelegationResponse),
    UnbondingDelegations(QueryDelegatorUnbondingDelegationsResponse),
    DelegatorValidator(QueryDelegatorValidatorResponse),
    Redelegations(QueryRedelegationsResponse),
    DelegatorValidators(QueryDelegatorValidatorsResponse),
    HistoricalInfo(QueryHistoricalInfoResponse),
    Pool(QueryPoolResponse),
    Params(QueryParamsResponse),
}

impl<
        SK: StoreKey,
        PSK: ParamsSubspaceKey,
        AK: AuthKeeper<SK, M> + Clone + Send + Sync + 'static,
        BK: StakingBankKeeper<SK, M>,
        KH: KeeperHooks<SK, AK, M>,
        M: Module,
        MI: ModuleInfo + Clone + Send + Sync + 'static,
    > ABCIHandler for StakingABCIHandler<SK, PSK, AK, BK, KH, M, MI>
{
    type Message = Message;

    type Genesis = GenesisState;

    type StoreKey = SK;

    type QReq = StakingNodeQueryRequest;

    type QRes = StakingNodeQueryResponse;

    fn typed_query<DB: Database>(
        &self,
        ctx: &QueryContext<DB, Self::StoreKey>,
        query: Self::QReq,
    ) -> Self::QRes {
        match query {
            StakingNodeQueryRequest::Validator(req) => {
                StakingNodeQueryResponse::Validator(self.query_validator(ctx, req))
            }
            StakingNodeQueryRequest::Validators(req) => {
                StakingNodeQueryResponse::Validators(self.query_validators(ctx, req))
            }
            StakingNodeQueryRequest::ValidatorDelegations(req) => {
                StakingNodeQueryResponse::ValidatorDelegations(
                    self.query_validator_delegations(ctx, req),
                )
            }
            StakingNodeQueryRequest::ValidatorUnbondingDelegations(req) => {
                StakingNodeQueryResponse::ValidatorUnbondingDelegations(
                    self.query_validator_unbonding_delegations(ctx, req),
                )
            }
            StakingNodeQueryRequest::Delegation(req) => {
                StakingNodeQueryResponse::Delegation(self.query_delegation(ctx, req))
            }
            StakingNodeQueryRequest::Delegations(req) => {
                StakingNodeQueryResponse::Delegations(self.query_delegator_delegations(ctx, req))
            }
            StakingNodeQueryRequest::UnbondingDelegation(req) => {
                StakingNodeQueryResponse::UnbondingDelegation(
                    self.query_unbonding_delegation(ctx, req),
                )
            }
            StakingNodeQueryRequest::UnbondingDelegations(req) => {
                StakingNodeQueryResponse::UnbondingDelegations(
                    self.query_unbonding_delegations(ctx, req).unwrap_or(
                        QueryDelegatorUnbondingDelegationsResponse {
                            unbonding_responses: vec![],
                            pagination: None,
                        },
                    ),
                )
            }
            StakingNodeQueryRequest::DelegatorValidator(req) => {
                StakingNodeQueryResponse::DelegatorValidator(
                    self.query_delegator_validator(ctx, req),
                )
            }
            StakingNodeQueryRequest::Redelegations(req) => {
                StakingNodeQueryResponse::Redelegations(self.query_redelegations(ctx, req))
            }
            StakingNodeQueryRequest::DelegatorValidators(req) => {
                StakingNodeQueryResponse::DelegatorValidators(
                    self.query_delegator_validators(ctx, req),
                )
            }
            StakingNodeQueryRequest::HistoricalInfo(req) => {
                StakingNodeQueryResponse::HistoricalInfo(self.query_historical_info(ctx, req))
            }
            StakingNodeQueryRequest::Pool(_) => {
                StakingNodeQueryResponse::Pool(self.query_pool(ctx))
            }
            StakingNodeQueryRequest::Params(_) => {
                StakingNodeQueryResponse::Params(self.query_params(ctx))
            }
        }
    }

    fn run_ante_checks<DB: Database>(
        &self,
        _: &mut TxContext<'_, DB, Self::StoreKey>,
        _: &gears::types::tx::raw::TxWithRaw<Self::Message>,
        _: bool,
    ) -> Result<(), TxError> {
        Ok(())
    }

    fn msg<DB: Database>(
        &self,
        ctx: &mut TxContext<'_, DB, Self::StoreKey>,
        msg: &Self::Message,
    ) -> Result<(), TxError> {
        let result = match msg {
            Message::CreateValidator(msg) => {
                self.keeper
                    .create_validator(ctx, ctx.consensus_params().validator.clone(), msg)
            }
            Message::EditValidator(msg) => self.keeper.edit_validator(ctx, msg),
            Message::Delegate(msg) => self.keeper.delegate_cmd_handler(ctx, msg),
            Message::Redelegate(msg) => self.keeper.redelegate_cmd_handler(ctx, msg),
            Message::Undelegate(msg) => self.keeper.undelegate_cmd_handler(ctx, msg),
        };

        result.map_err(|e| Into::<StakingTxError>::into(e).into::<MI>())
    }

    fn init_genesis<DB: Database>(
        &self,
        ctx: &mut InitContext<'_, DB, Self::StoreKey>,
        genesis: Self::Genesis,
    ) -> Vec<ValidatorUpdate> {
        self.genesis(ctx, genesis)
    }

    fn query<DB: Database + Send + Sync>(
        &self,
        ctx: &QueryContext<DB, Self::StoreKey>,
        query: RequestQuery,
    ) -> Result<Vec<u8>, QueryError> {
        match query.path.as_str() {
            QueryValidatorRequest::QUERY_URL => {
                let req = QueryValidatorRequest::decode(query.data)?;

                Ok(self.query_validator(ctx, req).into_bytes())
            }
            QueryValidatorsRequest::QUERY_URL => {
                let req = QueryValidatorsRequest::decode(query.data)?;

                Ok(self.query_validators(ctx, req).into_bytes())
            }
            QueryValidatorDelegationsRequest::QUERY_URL => {
                let req = QueryValidatorDelegationsRequest::decode(query.data)?;

                Ok(self.query_validator_delegations(ctx, req).into_bytes())
            }
            QueryValidatorUnbondingDelegationsRequest::QUERY_URL => {
                let req = QueryValidatorUnbondingDelegationsRequest::decode(query.data)?;

                Ok(self
                    .query_validator_unbonding_delegations(ctx, req)
                    .into_bytes())
            }
            QueryDelegationRequest::QUERY_URL => {
                let req = QueryDelegationRequest::decode(query.data)?;

                Ok(self.query_delegation(ctx, req).into_bytes())
            }
            QueryDelegatorDelegationsRequest::QUERY_URL => {
                let req = QueryDelegatorDelegationsRequest::decode(query.data)?;

                Ok(self.query_delegator_delegations(ctx, req).into_bytes())
            }
            QueryUnbondingDelegationRequest::QUERY_URL => {
                let req = QueryUnbondingDelegationRequest::decode(query.data)?;

                Ok(self.query_unbonding_delegation(ctx, req).into_bytes())
            }
            QueryDelegatorUnbondingDelegationsRequest::QUERY_URL => {
                let req = QueryDelegatorUnbondingDelegationsRequest::decode(query.data)?;

                Ok(self.query_unbonding_delegations(ctx, req)?.into_bytes())
            }
            QueryRedelegationsRequest::QUERY_URL => {
                let req = QueryRedelegationsRequest::decode(query.data)?;

                Ok(self.query_redelegations(ctx, req).into_bytes())
            }
            QueryHistoricalInfoRequest::QUERY_URL => {
                let req = QueryHistoricalInfoRequest::decode(query.data)?;

                Ok(self.query_historical_info(ctx, req).into_bytes())
            }
            QueryPoolRequest::QUERY_URL => Ok(self.query_pool(ctx).into_bytes()),
            QueryParamsRequest::QUERY_URL => Ok(self.query_params(ctx).into_bytes()),
            _ => Err(QueryError::PathNotFound),
        }
    }

    fn begin_block<'a, DB: Database>(
        &self,
        ctx: &mut BlockContext<'_, DB, Self::StoreKey>,
        _request: RequestBeginBlock,
    ) {
        self.keeper.track_historical_info(ctx);
    }

    fn end_block<DB: Database>(
        &self,
        ctx: &mut BlockContext<'_, DB, SK>,
        _request: RequestEndBlock,
    ) -> Vec<ValidatorUpdate> {
        self.keeper.block_validator_updates(ctx)
    }
}

impl<
        SK: StoreKey,
        PSK: ParamsSubspaceKey,
        AK: AuthKeeper<SK, M>,
        BK: StakingBankKeeper<SK, M>,
        KH: KeeperHooks<SK, AK, M>,
        M: Module,
        MI: ModuleInfo,
    > StakingABCIHandler<SK, PSK, AK, BK, KH, M, MI>
{
    pub fn new(keeper: Keeper<SK, PSK, AK, BK, KH, M>) -> Self {
        StakingABCIHandler {
            keeper,
            phantom_data: std::marker::PhantomData,
        }
    }

    pub fn genesis<DB: Database>(
        &self,
        ctx: &mut InitContext<'_, DB, SK>,
        genesis: GenesisState,
    ) -> Vec<ValidatorUpdate> {
        match self.keeper.init_genesis(ctx, genesis) {
            Ok(updates) => updates,
            Err(err) => panic!("{err}"),
        }
    }

    fn query_redelegations<DB: Database>(
        &self,
        ctx: &QueryContext<DB, SK>,
        QueryRedelegationsRequest {
            delegator_address,
            src_validator_address,
            dst_validator_address,
            pagination,
        }: QueryRedelegationsRequest,
    ) -> QueryRedelegationsResponse {
        let (p_result, redelegations) = self.keeper.redelegations(
            ctx,
            &delegator_address,
            &src_validator_address,
            &dst_validator_address,
            pagination.map(Pagination::from),
        );

        let redelegation_responses = self
            .redelegations_to_redelegations_response(ctx, redelegations)
            .ok()
            .unwrap_or_default();

        QueryRedelegationsResponse {
            redelegation_responses,
            pagination: p_result.map(PaginationResponse::from),
        }
    }

    fn redelegations_to_redelegations_response<DB: Database>(
        &self,
        ctx: &QueryContext<DB, SK>,
        redelegations: Vec<Redelegation>,
    ) -> Result<Vec<RedelegationResponse>, anyhow::Error> {
        let mut resp = Vec::with_capacity(redelegations.len());
        for red in redelegations.into_iter() {
            let validator = self
                .keeper
                .validator(ctx, &red.validator_dst_address)
                .unwrap_gas()
                .ok_or(anyhow::anyhow!("account not found"))?;

            let mut entries = Vec::with_capacity(red.entries.len());
            for entry in red.entries.clone().into_iter() {
                let balance = validator
                    .tokens_from_shares(entry.share_dst)
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?
                    .to_uint_floor();
                entries.push(RedelegationEntryResponse {
                    redelegation_entry: entry,
                    balance,
                });
            }

            resp.push(RedelegationResponse {
                redelegation: red,
                entries,
            });
        }

        Ok(resp)
    }
    pub fn query_validator<DB: Database>(
        &self,
        ctx: &QueryContext<DB, SK>,
        query: QueryValidatorRequest,
    ) -> QueryValidatorResponse {
        let validator = self
            .keeper
            .validator(ctx, &query.validator_addr)
            .unwrap_gas()
            .map(Into::into);
        QueryValidatorResponse { validator }
    }

    pub fn query_validators<DB: Database>(
        &self,
        ctx: &QueryContext<DB, SK>,
        query: QueryValidatorsRequest,
    ) -> QueryValidatorsResponse {
        let (pagination, validators) = self.keeper.validators(ctx, query.status, query.pagination);

        QueryValidatorsResponse {
            validators,
            pagination,
        }
    }

    pub fn query_delegation<DB: Database>(
        &self,
        ctx: &QueryContext<DB, SK>,
        query: QueryDelegationRequest,
    ) -> QueryDelegationResponse {
        if let Some(delegation) = self
            .keeper
            .delegation(ctx, &query.delegator_addr, &query.validator_addr)
            .unwrap_gas()
        {
            let delegation_response = self
                .keeper
                .delegation_to_delegation_response(ctx, delegation)
                .ok();
            QueryDelegationResponse {
                delegation_response,
            }
        } else {
            QueryDelegationResponse {
                delegation_response: None,
            }
        }
    }

    pub fn query_delegator_delegations<DB: Database>(
        &self,
        ctx: &QueryContext<DB, SK>,
        query: QueryDelegatorDelegationsRequest,
    ) -> QueryDelegatorDelegationsResponse {
        let (pagination, delegation_responses) =
            self.keeper
                .delegator_delegations(ctx, &query.delegator_addr, query.pagination);

        QueryDelegatorDelegationsResponse {
            delegation_responses,
            pagination,
        }
    }

    pub fn query_delegator_validator<DB: Database>(
        &self,
        ctx: &QueryContext<DB, SK>,
        QueryDelegatorValidatorRequest {
            delegator_addr,
            validator_addr,
        }: QueryDelegatorValidatorRequest,
    ) -> QueryDelegatorValidatorResponse {
        let delegation = self
            .keeper
            .delegation(ctx, &delegator_addr, &validator_addr)
            .unwrap_gas();
        let validator = self.keeper.validator(ctx, &validator_addr).unwrap_gas();
        if delegation.is_some() {
            QueryDelegatorValidatorResponse { validator }
        } else {
            QueryDelegatorValidatorResponse { validator: None }
        }
    }

    pub fn query_validator_delegations<DB: Database>(
        &self,
        ctx: &QueryContext<DB, SK>,
        query: QueryValidatorDelegationsRequest,
    ) -> QueryValidatorDelegationsResponse {
        let (pagination, delegation_responses) =
            self.keeper
                .validator_delegations(ctx, &query.validator_addr, query.pagination);

        QueryValidatorDelegationsResponse {
            delegation_responses,
            pagination,
        }
    }

    pub fn query_validator_unbonding_delegations<DB: Database>(
        &self,
        ctx: &QueryContext<DB, SK>,
        query: QueryValidatorUnbondingDelegationsRequest,
    ) -> QueryValidatorUnbondingDelegationsResponse {
        let unbonding_delegations = self
            .keeper
            .unbonding_delegations_from_validator(ctx, &query.validator_addr)
            .unwrap_gas();

        let (p_res, iter) = unbonding_delegations
            .into_iter()
            .maybe_paginate(query.pagination.map(Pagination::from));

        let unbonding_responses = iter.collect();

        QueryValidatorUnbondingDelegationsResponse {
            unbonding_responses,
            pagination: p_res.map(PaginationResponse::from),
        }
    }

    pub fn query_unbonding_delegation<DB: Database>(
        &self,
        ctx: &QueryContext<DB, SK>,
        query: QueryUnbondingDelegationRequest,
    ) -> QueryUnbondingDelegationResponse {
        QueryUnbondingDelegationResponse {
            unbond: self
                .keeper
                .unbonding_delegation(ctx, &query.delegator_addr, &query.validator_addr)
                .unwrap_gas(),
        }
    }

    pub fn query_unbonding_delegations<DB: Database>(
        &self,
        ctx: &QueryContext<DB, SK>,
        query: QueryDelegatorUnbondingDelegationsRequest,
    ) -> Result<QueryDelegatorUnbondingDelegationsResponse, QueryError> {
        let res = self
            .keeper
            .unbonding_delegations(ctx, &query.delegator_addr, query.pagination);

        res.map(
            |(pagination, unbonding_responses)| QueryDelegatorUnbondingDelegationsResponse {
                unbonding_responses,
                pagination,
            },
        )
    }

    /// query_delegator_validators queries all validators info for given delegator address
    pub fn query_delegator_validators<DB: Database>(
        &self,
        ctx: &QueryContext<DB, SK>,
        query: QueryDelegatorValidatorsRequest,
    ) -> QueryDelegatorValidatorsResponse {
        let (pagination, validators) =
            self.keeper
                .delegator_validators(ctx, &query.delegator_addr, query.pagination);

        QueryDelegatorValidatorsResponse {
            validators,
            pagination,
        }
    }

    pub fn query_historical_info<DB: Database>(
        &self,
        ctx: &QueryContext<DB, SK>,
        QueryHistoricalInfoRequest { height }: QueryHistoricalInfoRequest,
    ) -> QueryHistoricalInfoResponse {
        let historical_info = self.keeper.historical_info(ctx, height as u32).unwrap_gas();
        QueryHistoricalInfoResponse {
            hist: historical_info,
        }
    }

    pub fn query_pool<DB: Database>(&self, ctx: &QueryContext<DB, SK>) -> QueryPoolResponse {
        let pool = self.keeper.pool(ctx).unwrap_gas();
        QueryPoolResponse { pool: Some(pool) }
    }

    pub fn query_params<DB: Database>(&self, ctx: &QueryContext<DB, SK>) -> QueryParamsResponse {
        let params = self.keeper.params(ctx);
        QueryParamsResponse {
            params: Some(params),
        }
    }
}
