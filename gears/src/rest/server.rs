use crate::{
    application::ApplicationInfo,
    baseapp::{NodeQueryHandler, QueryRequest, QueryResponse},
    rest::handlers::{
        block, block_latest, health, node_info, send_tx, tx, txs, validatorsets,
        validatorsets_latest,
    },
    runtime::runtime,
    types::tx::TxMessage,
};
use axum::{extract::FromRef, http::Method, routing::get, Router};
use std::{marker::PhantomData, net::SocketAddr};
use tendermint::rpc::client::HttpClientUrl;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

pub fn run_rest_server<
    M: TxMessage,
    QReq: QueryRequest,
    QRes: QueryResponse,
    App: NodeQueryHandler<QReq, QRes> + ApplicationInfo,
>(
    app: App,
    listen_addr: SocketAddr,
    router: Router<RestState<QReq, QRes, App>>,
    tendermint_rpc_address: HttpClientUrl,
) {
    std::thread::spawn(move || {
        let result = runtime().block_on(launch::<M, _, _, _>(
            app,
            listen_addr,
            router,
            tendermint_rpc_address,
        ));
        if let Err(err) = result {
            panic!("Failed to run rest server with err: {}", err)
        }
    });
}

#[derive(Clone, Debug)]
pub struct RestState<QReq, QRes, App: NodeQueryHandler<QReq, QRes>> {
    pub app: App,
    pub tendermint_rpc_address: HttpClientUrl,
    phantom: PhantomData<(QReq, QRes)>,
}

impl<QReq, QRes, App: NodeQueryHandler<QReq, QRes>> FromRef<RestState<QReq, QRes, App>>
    for HttpClientUrl
{
    fn from_ref(rest_state: &RestState<QReq, QRes, App>) -> HttpClientUrl {
        rest_state.tendermint_rpc_address.clone()
    }
}

// TODO:
// 1. Replace "accept" header to force rocket to return json errors rather than the default HTML.
// 2. what happens if a route panics?
// 3. No error message unrecognized route - does return a 404 - can use a "fallback" route if necessary
async fn launch<
    M: TxMessage,
    QReq: QueryRequest,
    QRes: QueryResponse,
    App: NodeQueryHandler<QReq, QRes> + ApplicationInfo,
>(
    app: App,
    listen_addr: SocketAddr,
    router: Router<RestState<QReq, QRes, App>>,
    tendermint_rpc_address: HttpClientUrl,
) -> anyhow::Result<()> {
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any);

    let rest_state = RestState {
        app,
        tendermint_rpc_address,
        phantom: PhantomData,
    };

    let app = Router::new()
        .route("/cosmos/base/tendermint/v1beta1/health", get(health))
        .route(
            "/cosmos/base/tendermint/v1beta1/node_info",
            get(node_info::<QReq, QRes, App>),
        )
        .route(
            "/cosmos/base/tendermint/v1beta1/validatorsets/latest",
            get(validatorsets_latest),
        )
        .route(
            "/cosmos/base/tendermint/v1beta1/validatorsets/:height",
            get(validatorsets),
        )
        .route("/cosmos/tx/v1beta1/txs", get(txs::<M>).post(send_tx))
        .route("/cosmos/tx/v1beta1/txs/:hash", get(tx::<M>))
        .route(
            "/cosmos/base/tendermint/v1beta1/blocks/latest",
            get(block_latest),
        )
        .route("/cosmos/base/tendermint/v1beta1/blocks/:height", get(block))
        .merge(router)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(rest_state);

    let listener = tokio::net::TcpListener::bind(listen_addr).await?;

    tracing::info!("REST server running at {}", listen_addr);
    axum::serve(listener, app).await?;

    Ok(())
}
