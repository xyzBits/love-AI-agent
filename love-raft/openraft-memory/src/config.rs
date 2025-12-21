use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AppConfig {
    pub node_id: u64,
    pub raft_grpc_port: u16,
    pub business_grpc_port: u16,
    pub raft_nodes: std::collections::HashMap<u64, String>,
}

impl AppConfig {
    pub fn default_node(id: u64) -> Self {
        let mut nodes = std::collections::HashMap::new();
        nodes.insert(1, "127.0.0.1:50051".to_string());
        nodes.insert(2, "127.0.0.1:50052".to_string());
        nodes.insert(3, "127.0.0.1:50053".to_string());

        let (raft_grpc_port, business_grpc_port) = match id {
            1 => (50051, 60051),
            2 => (50052, 60052),
            3 => (50053, 60053),
            _ => (50050 + id as u16, 60050 + id as u16),
        };

        Self {
            node_id: id,
            raft_grpc_port,
            business_grpc_port,
            raft_nodes: nodes,
        }
    }
}
