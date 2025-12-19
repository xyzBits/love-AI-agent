use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AppConfig {
    pub node_id: u64,
    pub rpc_port: u16,
    pub rest_port: u16,
    pub raft_nodes: std::collections::HashMap<u64, String>,
}

impl AppConfig {
    pub fn default_node(id: u64) -> Self {
        let mut nodes = std::collections::HashMap::new();
        nodes.insert(1, "127.0.0.1:50051".to_string());
        nodes.insert(2, "127.0.0.1:50052".to_string());
        nodes.insert(3, "127.0.0.1:50053".to_string());

        let (rpc_port, rest_port) = match id {
            1 => (50051, 8081),
            2 => (50052, 8082),
            3 => (50053, 8083),
            _ => (50050 + id as u16, 8080 + id as u16),
        };

        Self {
            node_id: id,
            rpc_port,
            rest_port,
            raft_nodes: nodes,
        }
    }
}
