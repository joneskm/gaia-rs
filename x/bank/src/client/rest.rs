use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use gears::{
    baseapp::{NodeQueryHandler, QueryRequest, QueryResponse},
    rest::{error::HTTPError, RestState},
    types::denom::Denom,
};
use gears::{
    rest::Pagination,
    types::{address::AccAddress, pagination::request::PaginationRequest},
};
use serde::Deserialize;

use crate::{
    types::query::{QueryAllBalancesRequest, QueryBalanceRequest, QueryTotalSupplyRequest},
    BankNodeQueryRequest, BankNodeQueryResponse,
};

/// Gets the total supply of every denom
pub async fn supply<
    QReq: QueryRequest + From<BankNodeQueryRequest>,
    QRes: QueryResponse,
    App: NodeQueryHandler<QReq, QRes>,
>(
    pagination: Query<Option<Pagination>>,
    State(rest_state): State<RestState<QReq, QRes, App>>,
) -> Result<Json<QRes>, HTTPError> {
    let req = BankNodeQueryRequest::TotalSupply(QueryTotalSupplyRequest {
        pagination: pagination.0.map(PaginationRequest::from),
    });

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
    pagination: Query<Option<PaginationRequest>>,
    State(rest_state): State<RestState<QReq, QRes, App>>,
) -> Result<Json<QRes>, HTTPError> {
    let req = BankNodeQueryRequest::AllBalances(QueryAllBalancesRequest {
        address,
        pagination: pagination.0,
    });

    let res = rest_state.app.typed_query(req)?;

    Ok(Json(res))
}

#[derive(Deserialize)]
pub struct QueryData {
    denom: Denom,
}

// TODO: returns {"balance":null} if balance is zero, is this expected?
/// Get balance for a given address and denom
//#[get("/cosmos/bank/v1beta1/balances/<addr>/by_denom?<denom>")]
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

pub fn get_router<
    QReq: QueryRequest + From<BankNodeQueryRequest>,
    QRes: QueryResponse + TryInto<BankNodeQueryResponse>,
    App: NodeQueryHandler<QReq, QRes>,
>() -> Router<RestState<QReq, QRes, App>> {
    Router::new()
        .route("/v1beta1/supply", get(supply))
        .route("/v1beta1/balances/:address", get(get_balances))
        .route(
            "/v1beta1/balances/:address/by_denom",
            get(get_balances_by_denom::<QReq, QRes, App>),
        )
}
