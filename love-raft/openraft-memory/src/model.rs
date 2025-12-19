use serde::{Deserialize, Serialize};
use openraft::RaftTypeConfig;

pub mod pb {
    tonic::include_proto!("raft_service");
}

pub use pb::Student;

/// Raft 节点的 ID 类型
pub type NodeId = u64;

/// 状态机中的写操作（提案）
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Request {
    Create(Student),
    Update(Student),
    Delete(i64),
}

/// 状态机操作的响应
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Response {
    pub success: bool,
    pub message: String,
    pub data: Option<Student>,
}

/// OpenRaft 的类型配置
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TypeConfig {}

impl std::fmt::Display for TypeConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TypeConfig")
    }
}

impl RaftTypeConfig for TypeConfig {
    type D = Request;
    type R = Response;
    type NodeId = u64;
    type Node = openraft::impls::EmptyNode;
    type Entry = openraft::Entry<TypeConfig>;
    type SnapshotData = std::io::Cursor<Vec<u8>>;
    type AsyncRuntime = openraft::impls::TokioRuntime;
    type Responder = openraft::impls::OneshotResponder<TypeConfig>;
}
