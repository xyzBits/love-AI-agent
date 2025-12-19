use std::sync::Arc;

use openraft::error::InstallSnapshotError;
use openraft::error::NetworkError;
use openraft::error::RPCError;
use openraft::error::RaftError;
use openraft::network::RPCOption;
use openraft::network::RaftNetwork;
use openraft::network::RaftNetworkFactory;
use openraft::raft::AppendEntriesRequest;
use openraft::raft::AppendEntriesResponse;
use openraft::raft::InstallSnapshotRequest;
use openraft::raft::InstallSnapshotResponse;
use openraft::raft::VoteRequest;
use openraft::raft::VoteResponse;
use tonic::transport::Channel;

use crate::model::TypeConfig;
use crate::model::pb::raft_service_client::RaftServiceClient;
use crate::model::pb::{
    AppendEntriesRequest as PbAppendEntriesRequest,
    InstallSnapshotRequest as PbInstallSnapshotRequest, VoteRequest as PbVoteRequest,
};

/// NetworkFactory (网络工厂)
/// 负责为特定的目标节点创建网络连接实例。
pub struct NetworkFactory {
    /// 存储所有节点的 ID 到 RPC 地址的映射 (例如 1 -> "127.0.0.1:50051")
    pub node_addresses: Arc<std::collections::HashMap<u64, String>>,
}

impl RaftNetworkFactory<TypeConfig> for NetworkFactory {
    type Network = NetworkConnection;

    /// 创建一个连接到 target 节点的客户端
    async fn new_client(
        &mut self,
        target: u64,
        _node: &openraft::impls::EmptyNode,
    ) -> Self::Network {
        let addr = self
            .node_addresses
            .get(&target)
            .cloned()
            .expect("未找到节点地址");
        NetworkConnection {
            addr: format!("http://{}", addr),
        }
    }
}

/// NetworkConnection (网络连接实例)
/// 封装了与单个节点的 gRPC 通信逻辑。
pub struct NetworkConnection {
    addr: String,
}

impl NetworkConnection {
    /// 获取 tonic gRPC 客户端实例
    async fn get_client(&self) -> Result<RaftServiceClient<Channel>, NetworkError> {
        RaftServiceClient::connect(self.addr.clone())
            .await
            .map_err(|e| NetworkError::new(&e))
    }
}

/// 实现 RaftNetwork 接口
/// 负责具体的请求发送。这里使用 JSON 序列化后通过 gRPC 发送字符串数据。
impl RaftNetwork<TypeConfig> for NetworkConnection {
    /// 发送日志同步请求 (AppendEntries)
    async fn append_entries(
        &mut self,
        req: AppendEntriesRequest<TypeConfig>,
        _option: RPCOption,
    ) -> Result<AppendEntriesResponse<u64>, RPCError<u64, openraft::impls::EmptyNode, RaftError<u64>>>
    {
        let mut client = self
            .get_client()
            .await
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;

        // 序列化 OpenRaft 的请求对象为 JSON 字符串
        let serialized = serde_json::to_string(&req).unwrap();
        let pb_req = PbAppendEntriesRequest { data: serialized };

        // 通过 gRPC 发送
        let res = client
            .append_entries(pb_req)
            .await
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;

        // 解析响应中的 JSON 字符串回到 OpenRaft 响应对象
        let pb_res = res.into_inner();
        let raft_res: AppendEntriesResponse<u64> = serde_json::from_str(&pb_res.data)
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;

        Ok(raft_res)
    }

    /// 发送投票请求 (Vote)
    async fn vote(
        &mut self,
        req: VoteRequest<u64>,
        _option: RPCOption,
    ) -> Result<VoteResponse<u64>, RPCError<u64, openraft::impls::EmptyNode, RaftError<u64>>> {
        let mut client = self
            .get_client()
            .await
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;

        let serialized = serde_json::to_string(&req).unwrap();
        let pb_req = PbVoteRequest { data: serialized };

        let res = client
            .vote(pb_req)
            .await
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;

        let pb_res = res.into_inner();
        let raft_res: VoteResponse<u64> = serde_json::from_str(&pb_res.data)
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;

        Ok(raft_res)
    }

    /// 发送快照安装请求 (InstallSnapshot)
    async fn install_snapshot(
        &mut self,
        req: InstallSnapshotRequest<TypeConfig>,
        _option: RPCOption,
    ) -> Result<
        InstallSnapshotResponse<u64>,
        RPCError<u64, openraft::impls::EmptyNode, RaftError<u64, InstallSnapshotError>>,
    > {
        let mut client = self
            .get_client()
            .await
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;

        let serialized = serde_json::to_string(&req).unwrap();
        let pb_req = PbInstallSnapshotRequest { data: serialized };

        let res = client
            .install_snapshot(pb_req)
            .await
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;

        let pb_res = res.into_inner();
        let raft_res: InstallSnapshotResponse<u64> = serde_json::from_str(&pb_res.data)
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;

        Ok(raft_res)
    }
}
