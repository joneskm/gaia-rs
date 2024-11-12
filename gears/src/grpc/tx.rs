use ibc_proto::cosmos::tx::v1beta1::service_server::Service;
use ibc_proto::cosmos::tx::v1beta1::service_server::ServiceServer as TxServer;
use ibc_proto::cosmos::tx::v1beta1::{
    BroadcastTxRequest, BroadcastTxResponse, GetBlockWithTxsRequest, GetBlockWithTxsResponse,
    GetTxRequest, GetTxResponse, GetTxsEventRequest, GetTxsEventResponse, SimulateRequest,
    SimulateResponse,
};
use tonic::{Request, Response, Status};
use tracing::info;

#[derive(Debug)]
pub struct TxService;

#[tonic::async_trait]
impl Service for TxService {
    async fn simulate(
        &self,
        _request: Request<SimulateRequest>,
    ) -> Result<Response<SimulateResponse>, Status> {
        info!("Received a gRPC request tx::simulate");
        // TODO: run simulation once implemented
        Ok(Response::new(SimulateResponse {
            gas_info: None,
            result: None,
        }))
    }

    async fn get_tx(
        &self,
        _request: Request<GetTxRequest>,
    ) -> Result<Response<GetTxResponse>, Status> {
        //TODO: implement
        unimplemented!()
    }

    async fn broadcast_tx(
        &self,
        _request: Request<BroadcastTxRequest>,
    ) -> Result<Response<BroadcastTxResponse>, Status> {
        //TODO: implement
        unimplemented!()
    }

    async fn get_txs_event(
        &self,
        _request: Request<GetTxsEventRequest>,
    ) -> Result<Response<GetTxsEventResponse>, Status> {
        //TODO: implement
        unimplemented!()
    }

    async fn get_block_with_txs(
        &self,
        _request: Request<GetBlockWithTxsRequest>,
    ) -> Result<Response<GetBlockWithTxsResponse>, Status> {
        //TODO: implement
        unimplemented!()
    }
}

pub fn tx_server() -> TxServer<TxService> {
    TxServer::new(TxService)
}
