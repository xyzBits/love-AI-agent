use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use openraft::Raft;
use openraft::raft::AppendEntriesResponse;
use tonic::{Request as TonicRequest, Response as TonicResponse, Status};

use crate::model::pb::raft_service_server::RaftService;
use crate::model::pb::{
    self, AppendEntriesRequest as PbAppendEntriesRequest,
    AppendEntriesResponse as PbAppendEntriesResponse, CreateStudentRequest, DeleteStudentRequest,
    GetStudentRequest, InstallSnapshotRequest as PbInstallSnapshotRequest,
    InstallSnapshotResponse as PbInstallSnapshotResponse, StudentResponse, UpdateStudentRequest,
    VoteRequest as PbVoteRequest, VoteResponse as PbVoteResponse,
};
use crate::model::{Request, Student, TypeConfig};
use crate::store::Store;

/// RaftGrpcServer (Raft 内部 gRPC 服务实现)
/// 负责接收来自其他 Raft 节点的 RPC 请求。
pub struct RaftGrpcServer {
    pub raft: Arc<Raft<TypeConfig>>,
}

#[tonic::async_trait]
impl RaftService for RaftGrpcServer {
    /// 处理 AppendEntries 请求
    async fn append_entries(
        &self,
        request: TonicRequest<PbAppendEntriesRequest>,
    ) -> Result<TonicResponse<PbAppendEntriesResponse>, Status> {
        let req: openraft::raft::AppendEntriesRequest<TypeConfig> =
            serde_json::from_str(&request.into_inner().data)
                .map_err(|e| Status::internal(e.to_string()))?;

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

    /// 处理投票请求
    async fn vote(
        &self,
        request: TonicRequest<PbVoteRequest>,
    ) -> Result<TonicResponse<PbVoteResponse>, Status> {
        let req: openraft::raft::VoteRequest<u64> =
            serde_json::from_str(&request.into_inner().data)
                .map_err(|e| Status::internal(e.to_string()))?;

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
        let req: openraft::raft::InstallSnapshotRequest<TypeConfig> =
            serde_json::from_str(&request.into_inner().data)
                .map_err(|e| Status::internal(e.to_string()))?;

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

    /// gRPC 接口：创建学生
    async fn create_student(
        &self,
        request: TonicRequest<CreateStudentRequest>,
    ) -> Result<TonicResponse<StudentResponse>, Status> {
        let req = request.into_inner();
        let pb_student = req
            .student
            .ok_or_else(|| Status::invalid_argument("缺少学生信息"))?;
        let student = Student {
            id: pb_student.id,
            name: pb_student.name,
            age: pb_student.age,
            gender: pb_student.gender,
            score: pb_student.score,
        };

        // 将请求写入 Raft 集群
        let raft_req = Request::Create(student);
        let res = self
            .raft
            .client_write(raft_req)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(TonicResponse::new(StudentResponse {
            success: res.data.success,
            message: res.data.message,
            student: res.data.data.map(|s| pb::Student {
                id: s.id,
                name: s.name,
                age: s.age,
                gender: s.gender,
                score: s.score,
            }),
        }))
    }

    /// gRPC 接口：更新学生
    async fn update_student(
        &self,
        request: TonicRequest<UpdateStudentRequest>,
    ) -> Result<TonicResponse<StudentResponse>, Status> {
        let req = request.into_inner();
        let pb_student = req
            .student
            .ok_or_else(|| Status::invalid_argument("缺少学生信息"))?;
        let student = Student {
            id: pb_student.id,
            name: pb_student.name,
            age: pb_student.age,
            gender: pb_student.gender,
            score: pb_student.score,
        };

        let raft_req = Request::Update(student);
        let res = self
            .raft
            .client_write(raft_req)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(TonicResponse::new(StudentResponse {
            success: res.data.success,
            message: res.data.message,
            student: res.data.data.map(|s| pb::Student {
                id: s.id,
                name: s.name,
                age: s.age,
                gender: s.gender,
                score: s.score,
            }),
        }))
    }

    /// gRPC 接口：删除学生
    async fn delete_student(
        &self,
        request: TonicRequest<DeleteStudentRequest>,
    ) -> Result<TonicResponse<StudentResponse>, Status> {
        let req = request.into_inner();
        let raft_req = Request::Delete(req.id);
        let res = self
            .raft
            .client_write(raft_req)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(TonicResponse::new(StudentResponse {
            success: res.data.success,
            message: res.data.message,
            student: res.data.data.map(|s| pb::Student {
                id: s.id,
                name: s.name,
                age: s.age,
                gender: s.gender,
                score: s.score,
            }),
        }))
    }

    /// gRPC 接口：获取学生（示例中未通过 Raft 读实现，推荐在生产中使用 read_index）
    async fn get_student(
        &self,
        request: TonicRequest<GetStudentRequest>,
    ) -> Result<TonicResponse<StudentResponse>, Status> {
        let _req = request.into_inner();
        Ok(TonicResponse::new(StudentResponse {
            success: false,
            message: "gRPC Get 接口暂未通过 Raft 流程实现，请使用 HTTP 接口直接查询".to_string(),
            student: None,
        }))
    }
}

/// AppState (REST 服务共享状态)
#[derive(Clone)]
pub struct AppState {
    pub raft: Arc<Raft<TypeConfig>>,
    pub store: Arc<Store>,
}

/// REST 接口：写入（创建）学生
pub async fn write_student(
    State(state): State<AppState>,
    Json(student): Json<Student>,
) -> impl IntoResponse {
    let req = Request::Create(student);
    let res = state.raft.client_write(req).await;
    match res {
        Ok(resp) => Json(resp.data).into_response(),
        Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// REST 接口：基于 ID 查询学生 (直接读取本地状态机，可能有延迟，但满足线性读建议使用 read_index)
pub async fn get_student_rest(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let sm = state.store.state_machine.read().await;
    match sm.data.get(&id) {
        Some(s) => Json(s.clone()).into_response(),
        None => (axum::http::StatusCode::NOT_FOUND, "未找到该学生").into_response(),
    }
}

/// REST 接口：删除学生
pub async fn delete_student_rest(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let req = Request::Delete(id);
    let res = state.raft.client_write(req).await;
    match res {
        Ok(resp) => Json(resp.data).into_response(),
        Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
