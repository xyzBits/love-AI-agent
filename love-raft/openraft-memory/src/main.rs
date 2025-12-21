use std::collections::BTreeMap;
use std::sync::Arc;

use openraft::storage::Adaptor;
use openraft::{Config, Raft};
use openraft_memory::api::RaftGrpcServer;
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
        "正在启动节点 {}，Raft 端口: {}，gRPC 业务端口: {}",
        node_id, config.raft_grpc_port, config.business_grpc_port
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

    // 5. 初始化网络层
    let network = NetworkFactory {
        node_addresses: Arc::new(config.raft_nodes.clone()),
    };

    // 6. 创建并启动 Raft 实例
    let raft = Raft::new(
        node_id,
        Arc::new(raft_config),
        network,
        log_store,
        state_machine,
    )
    .await?;

    // 7. 如果是第一个节点，主动触发集群初始化
    if node_id == 1 {
        let mut nodes = BTreeMap::new();
        // 初始成员只有自己
        nodes.insert(1, openraft::impls::EmptyNode {});
        raft.initialize(nodes).await.ok();
        println!("节点 1 已尝试初始化集群");
    }

    let raft = Arc::new(raft);

    // 7.5 监控集群状态变化 (Log node join/leave)
    let raft_monitoring = raft.clone();
    tokio::spawn(async move {
        let mut metrics_rx = raft_monitoring.metrics();
        let mut last_members = std::collections::BTreeSet::new();
        while metrics_rx.changed().await.is_ok() {
            let metrics = metrics_rx.borrow().clone();
            let current_members = metrics
                .membership_config
                .nodes()
                .map(|(&id, _)| id)
                .collect::<std::collections::BTreeSet<_>>();

            // 检查新加入的节点
            for node in current_members.difference(&last_members) {
                tracing::info!("🔔 节点已加入集群: {}", node);
            }
            // 检查退出的节点
            for node in last_members.difference(&current_members) {
                tracing::info!("🔕 节点已离开集群: {}", node);
            }
            last_members = current_members;
        }
    });

    // 8. 启动 Raft 内部通信 gRPC 服务
    let grpc_raft = raft.clone();
    let raft_addr = format!("0.0.0.0:{}", config.raft_grpc_port).parse()?;
    let raft_task = tokio::spawn(async move {
        println!("gRPC Raft 服务监听于 {}", raft_addr);
        Server::builder()
            .add_service(RaftServiceServer::new(RaftGrpcServer { raft: grpc_raft }))
            .serve(raft_addr)
            .await
            .unwrap();
    });

    // 9. 启动 Student 业务 gRPC 服务
    let grpc_student = raft.clone();
    let student_store = Arc::new(store.clone());
    let student_addr = format!("0.0.0.0:{}", config.business_grpc_port).parse()?;
    let student_rpc_task = tokio::spawn(async move {
        println!("gRPC Student 服务监听于 {}", student_addr);
        Server::builder()
            .add_service(
                openraft_memory::model::pb::student_service_server::StudentServiceServer::new(
                    openraft_memory::api::StudentGrpcServer {
                        raft: grpc_student,
                        store: student_store,
                    },
                ),
            )
            .serve(student_addr)
            .await
            .unwrap();
    });

    // 等待服务任务运行
    tokio::select! {
        _ = raft_task => println!("Raft 内部通信服务已停止"),
        _ = student_rpc_task => println!("gRPC Student 服务已停止"),
    }

    Ok(())
}
