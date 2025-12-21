// 引入 OpenRaft 及其适配器、配置
use openraft::storage::Adaptor;
use openraft::{Config, Raft};
// 引入我们自定义的模块（API、配置、Protobuf定义、网络、存储）
use openraft_memory::api::RaftGrpcServer;
use openraft_memory::config::AppConfig;
use openraft_memory::model::pb::raft_service_server::RaftServiceServer;
use openraft_memory::network::NetworkFactory;
use openraft_memory::store::Store;
// === 序幕：引入工具箱 ===
use std::collections::BTreeMap;
use std::sync::Arc;
// 引入 gRPC 服务端构建器
use tonic::transport::Server;
// 引入日志工具
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

// === 第一幕：引擎预热 ===
// 旁白：“Tokio 引擎启动！这是异步世界的主入口。”
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. 初始化日志系统
    // 旁白：“打开探照灯（Tracing）。设置过滤规则，让我们能看清系统运行的轨迹。”
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env()) // 读取环境变量 RUST_LOG 来决定打印级别
        .init();

    // 2. 身份确认
    // 旁白：“我是谁？检查环境变量 NODE_ID。如果没有，我默认就是 1 号节点。”
    let node_id = std::env::var("NODE_ID")
        .unwrap_or_else(|_| "1".to_string())
        .parse::<u64>()?;

    // 旁白：“读取我的详细配置（IP、端口映射表）。”
    let config = AppConfig::default_node(node_id);

    println!(
        "正在启动节点 {}，Raft 端口: {}，gRPC 业务端口: {}",
        node_id, config.raft_grpc_port, config.business_grpc_port
    );

    // 3. 制定家规 (Raft Core Config)
    // 旁白：“设定 Raft 协议的心跳节奏。心跳跳得太慢会被认为挂了，从而触发选举。”
    let raft_config = Config {
        heartbeat_interval: 250,   // 每 250ms 跳一次
        election_timeout_min: 500, // 至少等 500ms 没心跳才造反
        election_timeout_max: 1000,
        ..Default::default()
    };

    // 4. 挂载硬盘 (Storage Layer)
    // 旁白：“搬来我们的内存数据库（Store）。所有的数据和日志都存在这里。”
    let store = Store::new();

    // === 关键点解释 ===
    // 旁白：“这里用了一个适配器 (Adaptor)。OpenRaft v0.9 把存储分成了 Log 和 StateMachine 两部分。”
    // “但我们的 Store 可能是一个统一的实现。Adaptor 就像一个分线器，把一个 Store 拆分成 log_store 和 state_machine 两个接口给 Raft 用。”
    let (log_store, state_machine) = Adaptor::new(store.clone());

    // 5. 连接电话线 (Network Layer)
    // 旁白：“组装网络工厂。它知道怎么根据节点 ID 找到对应的 IP 地址，用来给别的节点打电话。”
    let network = NetworkFactory {
        node_addresses: Arc::new(config.raft_nodes.clone()),
    };

    // 6. === 注入灵魂 (Raft Node Initialization) ===
    // 旁白：“万事俱备。把身份证(node_id)、家规(config)、电话线(network)、日志本(log_store)和记账本(state_machine)合体。”
    // “Raft 节点正式诞生！”
    let raft = Raft::new(
        node_id,
        Arc::new(raft_config),
        network,
        log_store,
        state_machine,
    )
    .await?;

    // 7. 创世纪 (Bootstrap Cluster)
    // 旁白：“如果我是 1 号节点，我有特权。我要宣布集群成立，初始成员只有我自己。”
    // “这一步非常重要，否则集群永远不会开始工作，大家都在等 Leader。”
    if node_id == 1 {
        let mut nodes = BTreeMap::new();
        nodes.insert(1, openraft::impls::EmptyNode {}); // 初始集群配置
        raft.initialize(nodes).await.ok(); // 忽略错误，因为如果已经初始化过就会报错，但这没关系
        println!("节点 1 已尝试初始化集群");
    }

    // 旁白：“把 Raft 实例包装成 Arc，因为后面好几个任务都要共享它。”
    let raft = Arc::new(raft);

    // 7.5 安排保安 (Metrics Monitoring)
    // 旁白：“雇佣一个保安（后台任务），盯着集群成员名单。”
    let raft_monitoring = raft.clone();
    tokio::spawn(async move {
        // 订阅指标变化
        let mut metrics_rx = raft_monitoring.metrics();
        let mut last_members = std::collections::BTreeSet::new();

        // 只要指标有变化，就醒来干活
        while metrics_rx.changed().await.is_ok() {
            let metrics = metrics_rx.borrow().clone();
            // 提取当前成员 ID 列表
            let current_members = metrics
                .membership_config
                .nodes()
                .map(|(&id, _)| id)
                .collect::<std::collections::BTreeSet<_>>();

            // 比较差异：谁新来了？
            for node in current_members.difference(&last_members) {
                tracing::info!("🔔 节点已加入集群: {}", node);
            }
            // 比较差异：谁走了？
            for node in last_members.difference(&current_members) {
                tracing::info!("🔕 节点已离开集群: {}", node);
            }
            last_members = current_members;
        }
    });

    // 8. 开启内部通道 (Raft Internal gRPC)
    // 旁白：“打开后门。这是给其他 Raft 节点用的专用通道（投票、复制日志）。”
    let grpc_raft = raft.clone();
    let raft_addr = format!("0.0.0.0:{}", config.raft_grpc_port).parse()?;

    // 启动一个后台任务运行 gRPC Server
    let raft_task = tokio::spawn(async move {
        println!("gRPC Raft 服务监听于 {}", raft_addr);
        Server::builder()
            // 注册 Raft 服务
            .add_service(RaftServiceServer::new(RaftGrpcServer { raft: grpc_raft }))
            .serve(raft_addr)
            .await
            .unwrap();
    });

    // 9. 开启业务通道 (Client gRPC)
    // 旁白：“打开前门。这是给普通用户用的，处理 Student 数据的增删改查。”
    let grpc_student = raft.clone();
    let student_store = Arc::new(store.clone()); // 业务接口可能需要直接读 Store
    let student_addr = format!("0.0.0.0:{}", config.business_grpc_port).parse()?;

    // 启动另一个后台任务运行业务 gRPC Server
    let student_rpc_task = tokio::spawn(async move {
        println!("gRPC Student 服务监听于 {}", student_addr);
        Server::builder()
            // 注册 Student 服务
            .add_service(
                openraft_memory::model::pb::student_service_server::StudentServiceServer::new(
                    openraft_memory::api::StudentGrpcServer {
                        raft: grpc_student,
                        store: student_store, // 传入 store 用于读操作
                    },
                ),
            )
            .serve(student_addr)
            .await
            .unwrap();
    });

    // 10. 坚守岗位 (Wait Forever)
    // 旁白：“指挥官坐在控制台前，监视两个服务任务。”
    // “select! 宏的意思是：只要这两个任务中任意一个结束（通常是崩溃），整个程序就结束。”
    tokio::select! {
        _ = raft_task => println!("Raft 内部通信服务已停止"),
        _ = student_rpc_task => println!("gRPC Student 服务已停止"),
    }

    Ok(())
}
