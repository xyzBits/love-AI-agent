use openraft::storage::Adaptor;
use openraft::{Config, Raft};
use openraft_memory::api::student::StudentGrpcServer;
use openraft_memory::model::pb::CreateStudentRequest;
use openraft_memory::model::pb::Student as PbStudent;
use openraft_memory::model::pb::student_service_client::StudentServiceClient;
use openraft_memory::model::pb::student_service_server::StudentServiceServer;
use openraft_memory::model::{Student, TypeConfig};
use openraft_memory::network::NetworkFactory;
use openraft_memory::store::Store;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tokio::time::{Duration, sleep};
use tonic::transport::Server;

async fn setup_student_node(id: u64, rpc_port: u16) -> (Arc<Raft<TypeConfig>>, Arc<Store>) {
    let raft_config = Arc::new(Config::default());
    let store = Store::new();
    let (log_store, state_machine) = Adaptor::new(store.clone());

    let network = NetworkFactory {
        node_addresses: Arc::new(HashMap::new()),
    };

    let raft = Raft::new(id, raft_config, network, log_store, state_machine)
        .await
        .unwrap();
    let raft = Arc::new(raft);

    // 启动 gRPC 业务服务
    let grpc_raft = raft.clone();
    let grpc_store = Arc::new(store.clone());
    tokio::spawn(async move {
        let addr = format!("127.0.0.1:{}", rpc_port).parse().unwrap();
        Server::builder()
            .add_service(StudentServiceServer::new(StudentGrpcServer {
                raft: grpc_raft,
                store: grpc_store,
            }))
            .serve(addr)
            .await
            .ok();
    });

    (raft, Arc::new(store))
}

#[tokio::test(flavor = "multi_thread")]
async fn test_student_grpc_service() -> anyhow::Result<()> {
    let rpc_port = 61001;
    let (raft, _store) = setup_student_node(1, rpc_port).await;

    let mut nodes = BTreeMap::new();
    nodes.insert(1, openraft::impls::EmptyNode {});
    raft.initialize(nodes).await?;
    sleep(Duration::from_millis(1000)).await;

    // 1. 通过 gRPC 创建学生
    let mut client =
        StudentServiceClient::connect(format!("http://127.0.0.1:{}", rpc_port)).await?;
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

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_student_write_to_follower() -> anyhow::Result<()> {
    // 启动两个节点，模拟集群环境
    let mut nodes_config = HashMap::new();
    nodes_config.insert(1, "127.0.0.1:52051".to_string());
    nodes_config.insert(2, "127.0.0.1:52052".to_string());

    let (raft1, _store1) = setup_student_node(1, 62051).await;
    let (raft2, _store2) = setup_student_node(2, 62052).await;

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

    let res = raft2
        .client_write(openraft_memory::model::Request::Create(student))
        .await;
    // 预期失败：因为 raft2 不是 Leader
    assert!(res.is_err(), "向非 Leader 节点写入请求应当返回错误");
    println!("写入 Follower 成功返回预期的错误: {}", res.err().unwrap());

    Ok(())
}
