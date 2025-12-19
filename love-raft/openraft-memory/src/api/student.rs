use std::sync::Arc;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::Json;
use tonic::{Request as TonicRequest, Response as TonicResponse, Status};
use tracing::{info, error};

use crate::model::pb::student_service_server::StudentService;
use crate::model::pb::{
    self, CreateStudentRequest, GetStudentRequest, UpdateStudentRequest, DeleteStudentRequest, StudentResponse,
};
use crate::model::{Request, Student, TypeConfig};
use crate::store::Store;
use crate::api::AppState;

/// StudentGrpcServer (业务 gRPC 服务实现)
pub struct StudentGrpcServer {
    pub raft: Arc<openraft::Raft<TypeConfig>>,
    pub store: Arc<Store>,
}

#[tonic::async_trait]
impl StudentService for StudentGrpcServer {
    /// gRPC 接口：创建学生
    async fn create_student(&self, request: TonicRequest<CreateStudentRequest>) -> Result<TonicResponse<StudentResponse>, Status> {
        let req = request.into_inner();
        info!(">>> 收到 gRPC CreateStudent 请求: {:?}", req);
        
        let pb_student = req.student.ok_or_else(|| {
            let s = Status::invalid_argument("缺少学生信息");
            error!("!!! gRPC CreateStudent 失败: {}", s);
            s
        })?;
        let student = Student {
            id: pb_student.id,
            name: pb_student.name,
            age: pb_student.age,
            gender: pb_student.gender,
            score: pb_student.score,
        };

        let raft_req = Request::Create(student);
        let res = self.raft.client_write(raft_req).await.map_err(|e| {
            error!("!!! gRPC CreateStudent 写入 Raft 失败: {}", e);
            Status::internal(e.to_string())
        })?;

        let resp = StudentResponse {
            success: res.data.success,
            message: res.data.message,
            student: res.data.data.map(|s| pb::Student {
                id: s.id,
                name: s.name,
                age: s.age,
                gender: s.gender,
                score: s.score,
            }),
        };
        info!("<<< gRPC CreateStudent 返回: {:?}", resp);
        Ok(TonicResponse::new(resp))
    }

    /// gRPC 接口：更新学生
    async fn update_student(&self, request: TonicRequest<UpdateStudentRequest>) -> Result<TonicResponse<StudentResponse>, Status> {
        let req = request.into_inner();
        info!(">>> 收到 gRPC UpdateStudent 请求: {:?}", req);

        let pb_student = req.student.ok_or_else(|| {
            let s = Status::invalid_argument("缺少学生信息");
            error!("!!! gRPC UpdateStudent 失败: {}", s);
            s
        })?;
        let student = Student {
            id: pb_student.id,
            name: pb_student.name,
            age: pb_student.age,
            gender: pb_student.gender,
            score: pb_student.score,
        };

        let raft_req = Request::Update(student);
        let res = self.raft.client_write(raft_req).await.map_err(|e| {
            error!("!!! gRPC UpdateStudent 写入 Raft 失败: {}", e);
            Status::internal(e.to_string())
        })?;

        let resp = StudentResponse {
            success: res.data.success,
            message: res.data.message,
            student: res.data.data.map(|s| pb::Student {
                id: s.id,
                name: s.name,
                age: s.age,
                gender: s.gender,
                score: s.score,
            }),
        };
        info!("<<< gRPC UpdateStudent 返回: {:?}", resp);
        Ok(TonicResponse::new(resp))
    }

    /// gRPC 接口：删除学生
    async fn delete_student(&self, request: TonicRequest<DeleteStudentRequest>) -> Result<TonicResponse<StudentResponse>, Status> {
        let req = request.into_inner();
        info!(">>> 收到 gRPC DeleteStudent 请求: {:?}", req);

        let raft_req = Request::Delete(req.id);
        let res = self.raft.client_write(raft_req).await.map_err(|e| {
            error!("!!! gRPC DeleteStudent 写入 Raft 失败: {}", e);
            Status::internal(e.to_string())
        })?;

        let resp = StudentResponse {
            success: res.data.success,
            message: res.data.message,
            student: res.data.data.map(|s| pb::Student {
                id: s.id,
                name: s.name,
                age: s.age,
                gender: s.gender,
                score: s.score,
            }),
        };
        info!("<<< gRPC DeleteStudent 返回: {:?}", resp);
        Ok(TonicResponse::new(resp))
    }

    /// gRPC 接口：获取学生
    async fn get_student(&self, request: TonicRequest<GetStudentRequest>) -> Result<TonicResponse<StudentResponse>, Status> {
        let req = request.into_inner();
        info!(">>> 收到 gRPC GetStudent 请求: {:?}", req);

        let sm = self.store.state_machine.read().await;
        let resp = match sm.data.get(&req.id) {
            Some(s) => StudentResponse {
                success: true,
                message: "查询成功".to_string(),
                student: Some(pb::Student {
                    id: s.id,
                    name: s.name.clone(),
                    age: s.age,
                    gender: s.gender.clone(),
                    score: s.score,
                }),
            },
            None => StudentResponse {
                success: false,
                message: "未找到该学生".to_string(),
                student: None,
            },
        };

        info!("<<< gRPC GetStudent 返回: {:?}", resp);
        Ok(TonicResponse::new(resp))
    }
}

/// REST 接口：写入（创建）学生
pub async fn write_student(State(state): State<AppState>, Json(student): Json<Student>) -> impl IntoResponse {
    info!(">>> 收到 HTTP CreateStudent 请求: {:?}", student);
    let req = Request::Create(student);
    let res = state.raft.client_write(req).await;
    let resp = match res {
        Ok(resp) => {
            info!("<<< HTTP CreateStudent 成功: {:?}", resp.data);
            Json(resp.data).into_response()
        },
        Err(e) => {
            error!("!!! HTTP CreateStudent 写入 Raft 失败: {}", e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    };
    resp
}

/// REST 接口：基于 ID 查询学生
pub async fn get_student_rest(State(state): State<AppState>, Path(id): Path<i64>) -> impl IntoResponse {
    info!(">>> 收到 HTTP GetStudent 请求, ID: {}", id);
    let sm = state.store.state_machine.read().await;
    let resp = match sm.data.get(&id) {
        Some(s) => {
            info!("<<< HTTP GetStudent 成功: {:?}", s);
            Json(s.clone()).into_response()
        },
        None => {
            error!("!!! HTTP GetStudent 失败: 未找到 ID 为 {} 的学生", id);
            (axum::http::StatusCode::NOT_FOUND, "未找到该学生").into_response()
        }
    };
    resp
}

/// REST 接口：删除学生
pub async fn delete_student_rest(State(state): State<AppState>, Path(id): Path<i64>) -> impl IntoResponse {
    info!(">>> 收到 HTTP DeleteStudent 请求, ID: {}", id);
    let req = Request::Delete(id);
    let res = state.raft.client_write(req).await;
    let resp = match res {
        Ok(resp) => {
            info!("<<< HTTP DeleteStudent 成功: {:?}", resp.data);
            Json(resp.data).into_response()
        },
        Err(e) => {
            error!("!!! HTTP DeleteStudent 写入 Raft 失败: {}", e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    };
    resp
}
