use ibc_proto::cosmos::base::tendermint::v1beta1::service_server::Service as HealthService;
use ibc_proto::cosmos::base::tendermint::v1beta1::service_server::ServiceServer as HealthServer;
use ibc_proto::cosmos::base::tendermint::v1beta1::{
    AbciQueryRequest, AbciQueryResponse, GetBlockByHeightRequest, GetBlockByHeightResponse,
    GetLatestBlockRequest, GetLatestBlockResponse, GetLatestValidatorSetRequest,
    GetLatestValidatorSetResponse, GetNodeInfoRequest, GetNodeInfoResponse, GetSyncingRequest,
    GetSyncingResponse, GetValidatorSetByHeightRequest, GetValidatorSetByHeightResponse,
};
use tonic::{Request, Response, Status};
use tracing::info;

#[derive(Debug)]
pub struct GearsHealthService;

#[tonic::async_trait]
impl HealthService for GearsHealthService {
    async fn abci_query(
        &self,
        _request: Request<AbciQueryRequest>,
    ) -> Result<Response<AbciQueryResponse>, Status> {
        //TODO: implement
        unimplemented!()
    }

    async fn get_node_info(
        &self,
        _request: Request<GetNodeInfoRequest>,
    ) -> Result<Response<GetNodeInfoResponse>, Status> {
        //TODO: implement
        unimplemented!()
    }

    async fn get_syncing(
        &self,
        _request: Request<GetSyncingRequest>,
    ) -> Result<Response<GetSyncingResponse>, Status> {
        //TODO: query the node for syncing status and return it
        info!("Received a gRPC request health::get_syncing");
        Ok(Response::new(GetSyncingResponse { syncing: false }))
    }

    async fn get_latest_block(
        &self,
        _request: Request<GetLatestBlockRequest>,
    ) -> Result<Response<GetLatestBlockResponse>, Status> {
        //TODO: implement
        unimplemented!()
    }

    async fn get_block_by_height(
        &self,
        _request: Request<GetBlockByHeightRequest>,
    ) -> Result<Response<GetBlockByHeightResponse>, Status> {
        //TODO: implement
        unimplemented!()
    }

    async fn get_latest_validator_set(
        &self,
        _request: Request<GetLatestValidatorSetRequest>,
    ) -> Result<Response<GetLatestValidatorSetResponse>, Status> {
        //TODO: implement
        unimplemented!()
    }

    async fn get_validator_set_by_height(
        &self,
        _request: Request<GetValidatorSetByHeightRequest>,
    ) -> Result<Response<GetValidatorSetByHeightResponse>, Status> {
        //TODO: implement
        unimplemented!()
    }
}

pub fn health_server() -> HealthServer<GearsHealthService> {
    HealthServer::new(GearsHealthService)
}
