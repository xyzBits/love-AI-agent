use std::collections::BTreeMap;
use std::sync::Arc;

use axum::Router;
use axum::routing::{get, post};
use openraft::storage::Adaptor;
use openraft::{Config, Raft};
use openraft_memory::api::{
    AppState, RaftGrpcServer, delete_student_rest, get_student_rest, write_student,
};
use openraft_memory::config::AppConfig;
use openraft_memory::model::pb::raft_service_server::RaftServiceServer;
use openraft_memory::network::NetworkFactory;
use openraft_memory::store::Store;
use tonic::transport::Server;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. 初始化日志系统 (使用 tracing)
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    // 2. 根据环境变量 (NODE_ID) 加载节点配置，默认节点 ID 为 1
    let node_id = std::env::var("NODE_ID")
        .unwrap_or_else(|_| "1".to_string())
        .parse::<u64>()?;
    let config = AppConfig::default_node(node_id);

    println!(
        "正在启动节点 {}，RPC 端口: {}，REST 端口: {}",
        node_id, config.rpc_port, config.rest_port
    );

    // 3. Raft 协议核心配置
    // 包括心跳间隔、选举超时范围等
    let raft_config = Config {
        heartbeat_interval: 250,
        election_timeout_min: 500,
        election_timeout_max: 1000,
        ..Default::default()
    };

    // 4. 初始化存储层 (内存实现)
    let store = Store::new();

    // 使用 Adaptor 将 v1 接口的 RaftStorage 桥接到 v2 的 LogStorage 和 StateMachine
    // 这是 OpenRaft 0.9.x 提供的兼容性工具
    let (log_store, state_machine) = Adaptor::new(store.clone());

    // 5. 初始化网络层 (gRPC 客户端工厂)
    let network = NetworkFactory {
        node_addresses: Arc::new(config.raft_nodes.clone()),
    };

    // 6. 创建并启动 Raft 实例
    // 该实例负责日志复制、选举、一致性保证等所有核心逻辑
    let raft = Raft::new(
        node_id,
        Arc::new(raft_config),
        network,
        log_store,
        state_machine,
    )
    .await?;

    // 7. 如果是第一个节点，主动触发集群初始化
    // 在生产环境，通常外部通过管理接口触发初始化
    if node_id == 1 {
        let mut nodes = BTreeMap::new();
        // 初始成员只有自己
        nodes.insert(1, openraft::impls::EmptyNode {});
        raft.initialize(nodes).await.ok();
        println!("节点 1 已尝试初始化集群");
    }

    let raft = Arc::new(raft);

    // 8. 启动节点间通信服务 (gRPC)
    // 其他 Raft 节点将通过此服务与本节点交换日志和选票
    let grpc_raft = raft.clone();
    let grpc_addr = format!("127.0.0.1:{}", config.rpc_port).parse()?;
    let grpc_task = tokio::spawn(async move {
        println!("gRPC Raft 服务监听于 {}", grpc_addr);
        Server::builder()
            .add_service(RaftServiceServer::new(RaftGrpcServer { raft: grpc_raft }))
            .serve(grpc_addr)
            .await
            .unwrap();
    });

    // 9. 启动客户端业务接口服务 (REST API via Axum)
    // 用户通过此接口进行学生信息的 CRUD 操作
    let app_state = AppState {
        raft: raft.clone(),
        store: Arc::new(store),
    };
    let app = Router::new()
        .route("/student", post(write_student))
        .route(
            "/student/:id",
            get(get_student_rest).delete(delete_student_rest),
        )
        .with_state(app_state);

    let rest_addr = format!("127.0.0.1:{}", config.rest_port);
    let rest_task = tokio::spawn(async move {
        println!("REST 业务服务监听于 {}", rest_addr);
        let listener = tokio::net::TcpListener::bind(rest_addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    });

    // 等待服务任务运行 (实际上是无限循环)
    tokio::select! {
        _ = grpc_task => println!("gRPC 服务已停止"),
        _ = rest_task => println!("REST 服务已停止"),
    }

    Ok(())
}
