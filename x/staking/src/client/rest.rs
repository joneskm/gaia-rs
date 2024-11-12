use crate::{
    QueryDelegationRequest, QueryDelegatorDelegationsRequest,
    QueryDelegatorUnbondingDelegationsRequest, QueryDelegatorValidatorsRequest, QueryPoolRequest,
    QueryValidatorDelegationsRequest, QueryValidatorRequest,
    QueryValidatorUnbondingDelegationsRequest, QueryValidatorsRequest, StakingNodeQueryRequest,
    StakingNodeQueryResponse,
};
use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use gears::{
    baseapp::{NodeQueryHandler, QueryRequest, QueryResponse},
    rest::{error::HTTPError, Pagination, RestState},
    types::{
        address::{AccAddress, ValAddress},
        pagination::request::PaginationRequest,
    },
    x::types::validator::BondStatus,
};
use serde::{Deserialize, Serialize};

pub async fn validator<
    QReq: QueryRequest + From<StakingNodeQueryRequest>,
    QRes: QueryResponse + TryInto<StakingNodeQueryResponse>,
    App: NodeQueryHandler<QReq, QRes>,
>(
    Path(validator_addr): Path<ValAddress>,
    State(rest_state): State<RestState<QReq, QRes, App>>,
) -> Result<Json<QRes>, HTTPError> {
    let req = StakingNodeQueryRequest::Validator(QueryValidatorRequest { validator_addr });
    let res = rest_state.app.typed_query(req)?;
    Ok(Json(res))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ValidatorsQuery {
    status: Option<BondStatus>,
    // TODO: serde(flatten) doesn't work
    offset: Option<u32>,
    limit: Option<u8>,
}

pub async fn validators<
    QReq: QueryRequest + From<StakingNodeQueryRequest>,
    QRes: QueryResponse + TryInto<StakingNodeQueryResponse>,
    App: NodeQueryHandler<QReq, QRes>,
>(
    Query(ValidatorsQuery {
        status,
        offset,
        limit,
    }): Query<ValidatorsQuery>,
    State(rest_state): State<RestState<QReq, QRes, App>>,
) -> Result<Json<QRes>, HTTPError> {
    let req = StakingNodeQueryRequest::Validators(QueryValidatorsRequest {
        status: status.unwrap_or(BondStatus::Unspecified),
        pagination: Some(PaginationRequest::from(Pagination::new(offset, limit))),
    });
    let res = rest_state.app.typed_query(req)?;
    Ok(Json(res))
}

pub async fn validator_delegations<
    QReq: QueryRequest + From<StakingNodeQueryRequest>,
    QRes: QueryResponse + TryInto<StakingNodeQueryResponse>,
    App: NodeQueryHandler<QReq, QRes>,
>(
    Path(validator_addr): Path<ValAddress>,
    State(rest_state): State<RestState<QReq, QRes, App>>,
    Query(pagination): Query<Pagination>,
) -> Result<Json<QRes>, HTTPError> {
    let req = StakingNodeQueryRequest::ValidatorDelegations(QueryValidatorDelegationsRequest {
        validator_addr,
        pagination: Some(PaginationRequest::from(pagination)),
    });
    let res = rest_state.app.typed_query(req)?;
    Ok(Json(res))
}

pub async fn validator_unbonding_delegations<
    QReq: QueryRequest + From<StakingNodeQueryRequest>,
    QRes: QueryResponse + TryInto<StakingNodeQueryResponse>,
    App: NodeQueryHandler<QReq, QRes>,
>(
    Path((validator_addr, delegator_addr)): Path<(ValAddress, AccAddress)>,
    State(rest_state): State<RestState<QReq, QRes, App>>,
    Query(pagination): Query<Pagination>,
) -> Result<Json<StakingNodeQueryResponse>, HTTPError> {
    let req = StakingNodeQueryRequest::ValidatorUnbondingDelegations(
        QueryValidatorUnbondingDelegationsRequest {
            validator_addr,
            pagination: Some(PaginationRequest::from(pagination)),
        },
    );

    // TODO: consider to add filtering to the method
    if let StakingNodeQueryResponse::ValidatorUnbondingDelegations(mut res) = rest_state
        .app
        .typed_query(req)?
        .try_into()
        .map_err(|_| HTTPError::internal_server_error())?
    {
        res.unbonding_responses
            .retain(|ubd| ubd.delegator_address == delegator_addr);
        Ok(Json(
            StakingNodeQueryResponse::ValidatorUnbondingDelegations(res),
        ))
    } else {
        Err(HTTPError::internal_server_error())
    }
}

pub async fn delegation<
    QReq: QueryRequest + From<StakingNodeQueryRequest>,
    QRes: QueryResponse + TryInto<StakingNodeQueryResponse>,
    App: NodeQueryHandler<QReq, QRes>,
>(
    Path((validator_addr, delegator_addr)): Path<(ValAddress, AccAddress)>,
    State(rest_state): State<RestState<QReq, QRes, App>>,
) -> Result<Json<QRes>, HTTPError> {
    let req = StakingNodeQueryRequest::Delegation(QueryDelegationRequest {
        delegator_addr,
        validator_addr,
    });
    let res = rest_state.app.typed_query(req)?;
    Ok(Json(res))
}

pub async fn delegator_delegations<
    QReq: QueryRequest + From<StakingNodeQueryRequest>,
    QRes: QueryResponse + TryInto<StakingNodeQueryResponse>,
    App: NodeQueryHandler<QReq, QRes>,
>(
    Path(delegator_addr): Path<AccAddress>,
    Query(pagination): Query<Pagination>,
    State(rest_state): State<RestState<QReq, QRes, App>>,
) -> Result<Json<QRes>, HTTPError> {
    let req = StakingNodeQueryRequest::Delegations(QueryDelegatorDelegationsRequest {
        delegator_addr,
        pagination: Some(PaginationRequest::from(pagination)),
    });
    let res = rest_state.app.typed_query(req)?;
    Ok(Json(res))
}

pub async fn delegator_validators<
    QReq: QueryRequest + From<StakingNodeQueryRequest>,
    QRes: QueryResponse + TryInto<StakingNodeQueryResponse>,
    App: NodeQueryHandler<QReq, QRes>,
>(
    Path(delegator_addr): Path<AccAddress>,
    Query(pagination): Query<Pagination>,
    State(rest_state): State<RestState<QReq, QRes, App>>,
) -> Result<Json<QRes>, HTTPError> {
    let req = StakingNodeQueryRequest::DelegatorValidators(QueryDelegatorValidatorsRequest {
        delegator_addr,
        pagination: Some(PaginationRequest::from(pagination)),
    });
    let res = rest_state.app.typed_query(req)?;
    Ok(Json(res))
}

pub async fn unbonding_delegations<
    QReq: QueryRequest + From<StakingNodeQueryRequest>,
    QRes: QueryResponse + TryInto<StakingNodeQueryResponse>,
    App: NodeQueryHandler<QReq, QRes>,
>(
    Path(delegator_addr): Path<AccAddress>,
    Query(pagination): Query<Pagination>,
    State(rest_state): State<RestState<QReq, QRes, App>>,
) -> Result<Json<QRes>, HTTPError> {
    let req =
        StakingNodeQueryRequest::UnbondingDelegations(QueryDelegatorUnbondingDelegationsRequest {
            delegator_addr: delegator_addr.clone(),
            pagination: Some(PaginationRequest::from(pagination)),
        });
    let res = rest_state.app.typed_query(req)?;
    Ok(Json(res))
}

pub async fn pool<
    QReq: QueryRequest + From<StakingNodeQueryRequest>,
    QRes: QueryResponse + TryInto<StakingNodeQueryResponse>,
    App: NodeQueryHandler<QReq, QRes>,
>(
    State(rest_state): State<RestState<QReq, QRes, App>>,
) -> Result<Json<QRes>, HTTPError> {
    let req = StakingNodeQueryRequest::Pool(QueryPoolRequest {});
    let res = rest_state.app.typed_query(req)?;
    Ok(Json(res))
}

pub async fn params<
    QReq: QueryRequest + From<StakingNodeQueryRequest>,
    QRes: QueryResponse + TryInto<StakingNodeQueryResponse>,
    App: NodeQueryHandler<QReq, QRes>,
>(
    State(rest_state): State<RestState<QReq, QRes, App>>,
) -> Result<Json<QRes>, HTTPError> {
    let req = StakingNodeQueryRequest::Params(crate::QueryParamsRequest {});
    let res = rest_state.app.typed_query(req)?;
    Ok(Json(res))
}

pub fn get_router<
    QReq: QueryRequest + From<StakingNodeQueryRequest>,
    QRes: QueryResponse + TryInto<StakingNodeQueryResponse>,
    App: NodeQueryHandler<QReq, QRes>,
>() -> Router<RestState<QReq, QRes, App>> {
    Router::new()
        .route("/v1beta1/validators", get(validators))
        .route("/v1beta1/validators/:validator_addr", get(validator))
        .route(
            "/v1beta1/validators/:validator_addr/delegations",
            get(validator_delegations),
        )
        .route(
            "/v1beta1/validators/:validator_addr/delegations/:delegator_addr/unbonding_delegation",
            get(validator_unbonding_delegations),
        )
        .route(
            "/v1beta1/validators/:validator_addr/delegations/:delegator_addr",
            get(delegation),
        )
        .route(
            "/v1beta1/delegations/:delegator_addr",
            get(delegator_delegations),
        )
        .route(
            "/v1beta1/delegators/:delegator_addr/validators",
            get(delegator_validators),
        )
        .route(
            "/v1beta1/delegators/:delegator_addr/unbonding_delegations",
            get(unbonding_delegations),
        )
        .route("/v1beta1/pool", get(pool))
        .route("/v1beta1/params", get(params))
}
