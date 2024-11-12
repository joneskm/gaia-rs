use crate::{
    types::query::{
        QueryAllBalancesRequest, QueryBalanceRequest, QueryDenomMetadataRequest,
        QueryParamsRequest, QuerySupplyOfRequest, QueryTotalSupplyRequest,
    },
    BankNodeQueryRequest, BankNodeQueryResponse,
};
use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use gears::{
    baseapp::{NodeQueryHandler, QueryRequest, QueryResponse},
    rest::{error::HTTPError, Pagination, RestState},
    types::{address::AccAddress, denom::Denom, pagination::request::PaginationRequest},
};
use serde::Deserialize;

/// Gets the total supply of every denom
pub async fn supply<
    QReq: QueryRequest + From<BankNodeQueryRequest>,
    QRes: QueryResponse,
    App: NodeQueryHandler<QReq, QRes>,
>(
    pagination: Query<Pagination>,
    State(rest_state): State<RestState<QReq, QRes, App>>,
) -> Result<Json<QRes>, HTTPError> {
    let req = BankNodeQueryRequest::TotalSupply(QueryTotalSupplyRequest {
        pagination: Some(PaginationRequest::from(pagination.0)),
    });

    let res = rest_state.app.typed_query(req)?;
    Ok(Json(res))
}

/// Gets the total supply of every denom
pub async fn supply_by_denom<
    QReq: QueryRequest + From<BankNodeQueryRequest>,
    QRes: QueryResponse,
    App: NodeQueryHandler<QReq, QRes>,
>(
    query: Query<QueryData>,
    State(rest_state): State<RestState<QReq, QRes, App>>,
) -> Result<Json<QRes>, HTTPError> {
    let req = BankNodeQueryRequest::SupplyOf(QuerySupplyOfRequest {
        denom: query.0.denom,
    });
    let res = rest_state.app.typed_query(req)?;
    Ok(Json(res))
}

/// Gets the total supply of every denom
pub async fn supply_by_denom_path<
    QReq: QueryRequest + From<BankNodeQueryRequest>,
    QRes: QueryResponse,
    App: NodeQueryHandler<QReq, QRes>,
>(
    Path(denom): Path<Denom>,
    State(rest_state): State<RestState<QReq, QRes, App>>,
) -> Result<Json<QRes>, HTTPError> {
    let req = BankNodeQueryRequest::SupplyOf(QuerySupplyOfRequest { denom });
    let res = rest_state.app.typed_query(req)?;
    Ok(Json(res))
}

/// Get all balances for a given address
pub async fn get_balances<
    QReq: QueryRequest + From<BankNodeQueryRequest>,
    QRes: QueryResponse,
    App: NodeQueryHandler<QReq, QRes>,
>(
    Path(address): Path<AccAddress>,
    pagination: Query<Pagination>,
    State(rest_state): State<RestState<QReq, QRes, App>>,
) -> Result<Json<QRes>, HTTPError> {
    let req = BankNodeQueryRequest::AllBalances(QueryAllBalancesRequest {
        address,
        pagination: Some(pagination.0.into()),
    });

    let res = rest_state.app.typed_query(req)?;

    Ok(Json(res))
}

#[derive(Debug, Deserialize)]
pub struct QueryData {
    denom: Denom,
}

// TODO: returns {"balance":null} if balance is zero, is this expected?
/// Get balance for a given address and denom
pub async fn get_balances_by_denom<
    QReq: QueryRequest + From<BankNodeQueryRequest>,
    QRes: QueryResponse + TryInto<BankNodeQueryResponse>,
    App: NodeQueryHandler<QReq, QRes>,
>(
    Path(address): Path<AccAddress>,
    query: Query<QueryData>,
    State(rest_state): State<RestState<QReq, QRes, App>>,
) -> Result<Json<QRes>, HTTPError> {
    let req = BankNodeQueryRequest::Balance(QueryBalanceRequest {
        address,
        denom: query.0.denom,
    });

    let res = rest_state.app.typed_query(req)?;

    Ok(Json(res))
}

/// get_denom_metadata queries the client metadata for all registered coin denominations.
pub async fn get_denom_metadata<
    QReq: QueryRequest + From<BankNodeQueryRequest>,
    QRes: QueryResponse + TryInto<BankNodeQueryResponse>,
    App: NodeQueryHandler<QReq, QRes>,
>(
    Path(denom): Path<Denom>,
    State(rest_state): State<RestState<QReq, QRes, App>>,
) -> Result<Json<QRes>, HTTPError> {
    let req = BankNodeQueryRequest::DenomMetadata(QueryDenomMetadataRequest { denom });
    let res = rest_state.app.typed_query(req)?;
    Ok(Json(res))
}

/// params queries the module parameters.
pub async fn params<
    QReq: QueryRequest + From<BankNodeQueryRequest>,
    QRes: QueryResponse + TryInto<BankNodeQueryResponse>,
    App: NodeQueryHandler<QReq, QRes>,
>(
    State(rest_state): State<RestState<QReq, QRes, App>>,
) -> Result<Json<QRes>, HTTPError> {
    let req = BankNodeQueryRequest::Params(QueryParamsRequest {});
    let res = rest_state.app.typed_query(req)?;
    Ok(Json(res))
}

pub fn get_router<
    QReq: QueryRequest + From<BankNodeQueryRequest>,
    QRes: QueryResponse + TryInto<BankNodeQueryResponse>,
    App: NodeQueryHandler<QReq, QRes>,
>() -> Router<RestState<QReq, QRes, App>> {
    Router::new()
        .route("/v1beta1/supply", get(supply))
        .route("/v1beta1/supply/by_denom", get(supply_by_denom))
        .route("/v1beta1/supply/:denom", get(supply_by_denom_path))
        .route("/v1beta1/balances/:address", get(get_balances))
        .route(
            "/v1beta1/balances/:address/by_denom",
            get(get_balances_by_denom),
        )
        .route("/v1beta1/denoms_metadata/:denom", get(get_denom_metadata))
        .route("/v1beta1/params", get(params))
}
