use crate::model::TypeConfig;
use openraft::raft::AppendEntriesResponse;
use std::sync::Arc;
use tonic::{Request as TonicRequest, Response as TonicResponse, Status};

use crate::model::pb::raft_service_server::RaftService;
use crate::model::pb::{
    AppendEntriesRequest as PbAppendEntriesRequest,
    AppendEntriesResponse as PbAppendEntriesResponse,
    InstallSnapshotRequest as PbInstallSnapshotRequest,
    InstallSnapshotResponse as PbInstallSnapshotResponse, VoteRequest as PbVoteRequest,
    VoteResponse as PbVoteResponse,
};
// use crate::api::AppState;

/// RaftGrpcServer (Raft 内部 gRPC 服务实现)
pub struct RaftGrpcServer {
    pub raft: Arc<openraft::Raft<TypeConfig>>,
}

#[tonic::async_trait]
impl RaftService for RaftGrpcServer {
    async fn append_entries(
        &self,
        request: TonicRequest<PbAppendEntriesRequest>,
    ) -> Result<TonicResponse<PbAppendEntriesResponse>, Status> {
        let req_data = request.into_inner();
        let req: openraft::raft::AppendEntriesRequest<TypeConfig> =
            serde_json::from_str(&req_data.data).map_err(|e| Status::internal(e.to_string()))?;

        let res = self
            .raft
            .append_entries(req)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let success = match res {
            AppendEntriesResponse::Success { .. } => true,
            _ => false,
        };

        Ok(TonicResponse::new(PbAppendEntriesResponse {
            success,
            data: serde_json::to_string(&res).unwrap(),
        }))
    }

    async fn vote(
        &self,
        request: TonicRequest<PbVoteRequest>,
    ) -> Result<TonicResponse<PbVoteResponse>, Status> {
        let req_data = request.into_inner();
        let req: openraft::raft::VoteRequest<u64> =
            serde_json::from_str(&req_data.data).map_err(|e| Status::internal(e.to_string()))?;

        let res = self
            .raft
            .vote(req)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(TonicResponse::new(PbVoteResponse {
            success: res.vote_granted,
            data: serde_json::to_string(&res).unwrap(),
        }))
    }

    async fn install_snapshot(
        &self,
        request: TonicRequest<PbInstallSnapshotRequest>,
    ) -> Result<TonicResponse<PbInstallSnapshotResponse>, Status> {
        let req_data = request.into_inner();
        let req: openraft::raft::InstallSnapshotRequest<TypeConfig> =
            serde_json::from_str(&req_data.data).map_err(|e| Status::internal(e.to_string()))?;

        let res = self
            .raft
            .install_snapshot(req)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(TonicResponse::new(PbInstallSnapshotResponse {
            success: true,
            data: serde_json::to_string(&res).unwrap(),
        }))
    }
}
