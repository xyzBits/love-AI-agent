use std::sync::Arc;
use crate::model::TypeConfig;
use crate::store::Store;

pub mod raft;
pub mod student;

// 重新导出常用组件，方便 main.rs 调用
pub use raft::{RaftGrpcServer, raft_append_entries, raft_vote, raft_install_snapshot, get_cluster_info};
pub use student::{StudentGrpcServer, write_student, get_student_rest, delete_student_rest};

/// AppState (REST 服务共享状态)
#[derive(Clone)]
pub struct AppState {
    pub raft: Arc<openraft::Raft<TypeConfig>>,
    pub store: Arc<Store>,
}
