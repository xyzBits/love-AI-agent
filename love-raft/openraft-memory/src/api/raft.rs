use std::sync::Arc;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use openraft::raft::AppendEntriesResponse;
use tonic::{Request as TonicRequest, Response as TonicResponse, Status};

use crate::model::pb::raft_service_server::RaftService;
use crate::model::pb::{
    AppendEntriesRequest as PbAppendEntriesRequest, AppendEntriesResponse as PbAppendEntriesResponse,
    InstallSnapshotRequest as PbInstallSnapshotRequest, InstallSnapshotResponse as PbInstallSnapshotResponse,
    VoteRequest as PbVoteRequest, VoteResponse as PbVoteResponse,
};
use crate::model::TypeConfig;
use crate::api::AppState;

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
        let req: openraft::raft::AppendEntriesRequest<TypeConfig> = serde_json::from_str(&req_data.data)
            .map_err(|e| Status::internal(e.to_string()))?;

        let res = self.raft.append_entries(req).await.map_err(|e| Status::internal(e.to_string()))?;

        let success = match res {
            AppendEntriesResponse::Success { .. } => true,
            _ => false,
        };

        Ok(TonicResponse::new(PbAppendEntriesResponse {
            success,
            data: serde_json::to_string(&res).unwrap(),
        }))
    }

    async fn vote(&self, request: TonicRequest<PbVoteRequest>) -> Result<TonicResponse<PbVoteResponse>, Status> {
        let req_data = request.into_inner();
        let req: openraft::raft::VoteRequest<u64> = serde_json::from_str(&req_data.data)
            .map_err(|e| Status::internal(e.to_string()))?;

        let res = self.raft.vote(req).await.map_err(|e| Status::internal(e.to_string()))?;

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
        let req: openraft::raft::InstallSnapshotRequest<TypeConfig> = serde_json::from_str(&req_data.data)
            .map_err(|e| Status::internal(e.to_string()))?;

        let res = self.raft.install_snapshot(req).await.map_err(|e| Status::internal(e.to_string()))?;

        Ok(TonicResponse::new(PbInstallSnapshotResponse {
            success: true,
            data: serde_json::to_string(&res).unwrap(),
        }))
    }
}

/// HTTP Raft 路由处理函数
pub async fn raft_append_entries(
    State(state): State<AppState>,
    Json(req): Json<openraft::raft::AppendEntriesRequest<TypeConfig>>,
) -> impl IntoResponse {
    tracing::debug!("收到 HTTP AppendEntries 请求: {:?}", req);
    let res = state.raft.append_entries(req).await.unwrap();
    Json(res)
}

pub async fn raft_vote(
    State(state): State<AppState>,
    Json(req): Json<openraft::raft::VoteRequest<u64>>,
) -> impl IntoResponse {
    tracing::debug!("收到 HTTP Vote 请求: {:?}", req);
    let res = state.raft.vote(req).await.unwrap();
    Json(res)
}

pub async fn raft_install_snapshot(
    State(state): State<AppState>,
    Json(req): Json<openraft::raft::InstallSnapshotRequest<TypeConfig>>,
) -> impl IntoResponse {
    tracing::debug!("收到 HTTP InstallSnapshot 请求: {:?}", req);
    let res = state.raft.install_snapshot(req).await.unwrap();
    Json(res)
}

/// 获取集群状态
pub async fn get_cluster_info(State(state): State<AppState>) -> impl IntoResponse {
    let metrics = state.raft.metrics().borrow().clone();
    Json(metrics)
}
