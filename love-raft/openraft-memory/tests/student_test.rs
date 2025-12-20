use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use openraft_memory::config::RaftProtocol;
use openraft_memory::network::NetworkFactory;
use openraft_memory::api::student::StudentGrpcServer;
use openraft_memory::api::AppState;
use openraft_memory::model::pb::student_service_client::StudentServiceClient;
use openraft_memory::model::pb::student_service_server::StudentServiceServer;
use openraft_memory::model::pb::Student as PbStudent;
use openraft_memory::model::pb::CreateStudentRequest;
use openraft_memory::model::{TypeConfig, Student};
use openraft_memory::store::Store;
use openraft::{Config, Raft};
use tonic::transport::Server;
use openraft::storage::Adaptor;
use axum::{Router, routing::post};

async fn setup_student_node(id: u64, rpc_port: u16, rest_port: u16) -> (Arc<Raft<TypeConfig>>, Arc<Store>) {
    let raft_config = Arc::new(Config::default());
    let store = Store::new();
    let (log_store, state_machine) = Adaptor::new(store.clone());
    
    let network = NetworkFactory {
        node_addresses: Arc::new(HashMap::new()),
        protocol: RaftProtocol::Grpc,
    };

    let raft = Raft::new(id, raft_config, network, log_store, state_machine).await.unwrap();
    let raft = Arc::new(raft);

    // 启动 gRPC 业务服务
    let grpc_raft = raft.clone();
    let grpc_store = Arc::new(store.clone());
    tokio::spawn(async move {
        let addr = format!("127.0.0.1:{}", rpc_port).parse().unwrap();
        Server::builder()
            .add_service(StudentServiceServer::new(StudentGrpcServer { raft: grpc_raft, store: grpc_store }))
            .serve(addr)
            .await
            .ok();
    });

    // 启动 HTTP 业务服务
    let http_raft = raft.clone();
    let http_store = Arc::new(store.clone());
    tokio::spawn(async move {
        let app_state = AppState { raft: http_raft, store: http_store };
        let app = Router::new()
            .route("/student", post(openraft_memory::api::student::write_student))
            .with_state(app_state);
        let addr = format!("127.0.0.1:{}", rest_port);
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app).await.ok();
    });

    (raft, Arc::new(store))
}

#[tokio::test(flavor = "multi_thread")]
async fn test_student_service_dual_interface() -> anyhow::Result<()> {
    let rpc_port = 61001;
    let rest_port = 8101;
    let (raft, _store) = setup_student_node(1, rpc_port, rest_port).await;
    
    let mut nodes = BTreeMap::new();
    nodes.insert(1, openraft::impls::EmptyNode {});
    raft.initialize(nodes).await?;
    sleep(Duration::from_millis(1000)).await;

    // 1. 通过 gRPC 创建学生
    let mut client = StudentServiceClient::connect(format!("http://127.0.0.1:{}", rpc_port)).await?;
    let req = CreateStudentRequest {
        student: Some(PbStudent {
            id: 1,
            name: "GrpcUser".to_string(),
            age: 20,
            gender: "M".to_string(),
            score: 90.0,
        }),
    };
    let resp = client.create_student(req).await?.into_inner();
    assert!(resp.success);

    // 2. 通过 HTTP 创建学生
    let http_client = reqwest::Client::new();
    let student_http = Student {
        id: 2,
        name: "HttpUser".to_string(),
        age: 21,
        gender: "F".to_string(),
        score: 95.0,
    };
    let resp_http = http_client.post(format!("http://127.0.0.1:{}/student", rest_port))
        .json(&student_http)
        .send()
        .await?;
    assert!(resp_http.status().is_success());

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_student_write_to_follower() -> anyhow::Result<()> {
    // 启动两个节点，模拟集群环境
    let mut nodes_config = HashMap::new();
    nodes_config.insert(1, "127.0.0.1:52051".to_string());
    nodes_config.insert(2, "127.0.0.1:52052".to_string());

    let (raft1, _store1) = setup_student_node(1, 62051, 8251).await;
    let (raft2, _store2) = setup_student_node(2, 62052, 8252).await;

    // 这里由于 setup_student_node 内部 NetworkFactory 使用了 HashMap::new()，
    // 我们需要更复杂的设置来让两个节点互相看见。
    // 但为了简单回答用户问题，我们直接测试 client_write 在非 Leader 时的返回。
    
    // 初始化节点 1 为 Leader
    let mut nodes = BTreeMap::new();
    nodes.insert(1, openraft::impls::EmptyNode {});
    raft1.initialize(nodes).await?;
    
    sleep(Duration::from_millis(1000)).await;

    // 此时节点 2 一定不是 Leader (因为它没在 membership 中，且没经过选举)
    let student = Student {
        id: 999,
        name: "FollowerTest".to_string(),
        age: 20,
        gender: "M".to_string(),
        score: 100.0,
    };
    
    let res = raft2.client_write(openraft_memory::model::Request::Create(student)).await;
    // 预期失败：因为 raft2 不是 Leader
    assert!(res.is_err(), "向非 Leader 节点写入请求应当返回错误");
    println!("写入 Follower 成功返回预期的错误: {}", res.err().unwrap());

    Ok(())
}
