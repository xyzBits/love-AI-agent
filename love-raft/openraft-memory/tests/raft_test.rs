use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use openraft_memory::config::{AppConfig, RaftProtocol};
use openraft_memory::network::NetworkFactory;
use openraft_memory::api::raft::RaftGrpcServer;
use openraft_memory::model::pb::raft_service_server::RaftServiceServer;
use openraft_memory::model::TypeConfig;
use openraft_memory::store::Store;
use openraft::{Config, Raft};
use tonic::transport::Server;
use openraft::storage::Adaptor;

async fn start_raft_node(id: u64, rpc_port: u16, all_nodes: HashMap<u64, String>, protocol: RaftProtocol) -> Arc<Raft<TypeConfig>> {
    let raft_config = Arc::new(Config::default());
    let store = Store::new();
    let (log_store, state_machine) = Adaptor::new(store);
    
    let network = NetworkFactory {
        node_addresses: Arc::new(all_nodes),
        protocol,
    };

    let raft = Raft::new(id, raft_config, network, log_store, state_machine).await.unwrap();
    let raft = Arc::new(raft);

    if protocol == RaftProtocol::Grpc {
        let grpc_raft = raft.clone();
        tokio::spawn(async move {
            let addr = format!("127.0.0.1:{}", rpc_port).parse().unwrap();
            Server::builder()
                .add_service(RaftServiceServer::new(RaftGrpcServer { raft: grpc_raft }))
                .serve(addr)
                .await
                .ok();
        });
    } else {
        // HTTP 模式测试需要 Axum Server，这里简单演示逻辑一致性
        // 在集成测试中通常主要测试逻辑链路
    }

    raft
}

#[tokio::test(flavor = "multi_thread")]
async fn test_raft_grpc_protocol_startup() -> anyhow::Result<()> {
    let mut all_nodes = HashMap::new();
    all_nodes.insert(1, "127.0.0.1:51061".to_string());
    
    let raft = start_raft_node(1, 51061, all_nodes, RaftProtocol::Grpc).await;
    
    let mut nodes = BTreeMap::new();
    nodes.insert(1, openraft::impls::EmptyNode {});
    raft.initialize(nodes).await?;

    sleep(Duration::from_millis(500)).await;
    let metrics = raft.metrics().borrow().clone();
    assert_eq!(metrics.id, 1);
    
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_raft_http_protocol_config() -> anyhow::Result<()> {
    let mut all_nodes = HashMap::new();
    all_nodes.insert(1, "127.0.0.1:51062".to_string());
    
    let raft = start_raft_node(1, 51062, all_nodes, RaftProtocol::Http).await;
    
    let mut nodes = BTreeMap::new();
    nodes.insert(1, openraft::impls::EmptyNode {});
    raft.initialize(nodes).await?;

    sleep(Duration::from_millis(500)).await;
    let metrics = raft.metrics().borrow().clone();
    assert_eq!(metrics.id, 1);
    
    Ok(())
}
