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
use tracing::debug;

use crate::model::pb::raft_service_client::RaftServiceClient;
use crate::model::pb::{
    AppendEntriesRequest as PbAppendEntriesRequest, VoteRequest as PbVoteRequest,
    InstallSnapshotRequest as PbInstallSnapshotRequest,
};
use crate::model::TypeConfig;
use crate::config::RaftProtocol;

/// NetworkFactory (网络工厂)
pub struct NetworkFactory {
    pub node_addresses: Arc<std::collections::HashMap<u64, String>>,
    pub protocol: RaftProtocol,
}

impl RaftNetworkFactory<TypeConfig> for NetworkFactory {
    type Network = NetworkConnection;

    async fn new_client(&mut self, target: u64, _node: &openraft::impls::EmptyNode) -> Self::Network {
        let addr = self.node_addresses.get(&target).cloned().expect("未找到节点地址");
        NetworkConnection {
            target,
            addr,
            protocol: self.protocol,
            client: reqwest::Client::new(),
        }
    }
}

/// NetworkConnection (网络连接实例)
pub struct NetworkConnection {
    target: u64,
    addr: String,
    protocol: RaftProtocol,
    client: reqwest::Client,
}

impl NetworkConnection {
    async fn get_grpc_client(&self) -> Result<RaftServiceClient<Channel>, NetworkError> {
        let addr = format!("http://{}", self.addr);
        RaftServiceClient::connect(addr)
            .await
            .map_err(|e| NetworkError::new(&e))
    }

    async fn send_http<Req, Resp>(&self, path: &str, req: &Req) -> Result<Resp, NetworkError>
    where
        Req: serde::Serialize,
        Resp: serde::de::DeserializeOwned,
    {
        let url = format!("http://{}{}", self.addr, path);
        let resp = self.client.post(&url)
            .json(req)
            .send()
            .await
            .map_err(|e| NetworkError::new(&e))?;
        
        resp.json().await.map_err(|e| NetworkError::new(&e))
    }
}

impl RaftNetwork<TypeConfig> for NetworkConnection {
    async fn append_entries(
        &mut self,
        req: AppendEntriesRequest<TypeConfig>,
        _option: RPCOption,
    ) -> Result<AppendEntriesResponse<u64>, RPCError<u64, openraft::impls::EmptyNode, RaftError<u64>>>
    {
        debug!("发送 AppendEntries 到节点 {}: {:?}", self.target, req);
        match self.protocol {
            RaftProtocol::Grpc => {
                let mut client = self.get_grpc_client().await.map_err(|e| RPCError::Network(NetworkError::new(&e)))?;
                let serialized = serde_json::to_string(&req).unwrap();
                let pb_req = PbAppendEntriesRequest { data: serialized };
                let res = client.append_entries(pb_req).await.map_err(|e| RPCError::Network(NetworkError::new(&e)))?;
                let pb_res = res.into_inner();
                serde_json::from_str(&pb_res.data).map_err(|e| RPCError::Network(NetworkError::new(&e)))
            }
            RaftProtocol::Http => {
                self.send_http("/raft/append_entries", &req).await.map_err(|e| RPCError::Network(e))
            }
        }
    }

    async fn vote(
        &mut self,
        req: VoteRequest<u64>,
        _option: RPCOption,
    ) -> Result<VoteResponse<u64>, RPCError<u64, openraft::impls::EmptyNode, RaftError<u64>>> {
        debug!("发送 Vote 到节点 {}: {:?}", self.target, req);
        match self.protocol {
            RaftProtocol::Grpc => {
                let mut client = self.get_grpc_client().await.map_err(|e| RPCError::Network(NetworkError::new(&e)))?;
                let serialized = serde_json::to_string(&req).unwrap();
                let pb_req = PbVoteRequest { data: serialized };
                let res = client.vote(pb_req).await.map_err(|e| RPCError::Network(NetworkError::new(&e)))?;
                let pb_res = res.into_inner();
                serde_json::from_str(&pb_res.data).map_err(|e| RPCError::Network(NetworkError::new(&e)))
            }
            RaftProtocol::Http => {
                self.send_http("/raft/vote", &req).await.map_err(|e| RPCError::Network(e))
            }
        }
    }

    async fn install_snapshot(
        &mut self,
        req: InstallSnapshotRequest<TypeConfig>,
        _option: RPCOption,
    ) -> Result<
        InstallSnapshotResponse<u64>,
        RPCError<u64, openraft::impls::EmptyNode, RaftError<u64, InstallSnapshotError>>,
    > {
        debug!("发送 InstallSnapshot 到节点 {}: {:?}", self.target, req);
        match self.protocol {
            RaftProtocol::Grpc => {
                let mut client = self.get_grpc_client().await.map_err(|e| RPCError::Network(NetworkError::new(&e)))?;
                let serialized = serde_json::to_string(&req).unwrap();
                let pb_req = PbInstallSnapshotRequest { data: serialized };
                let res = client.install_snapshot(pb_req).await.map_err(|e| RPCError::Network(NetworkError::new(&e)))?;
                let pb_res = res.into_inner();
                serde_json::from_str(&pb_res.data).map_err(|e| RPCError::Network(NetworkError::new(&e)))
            }
            RaftProtocol::Http => {
                self.send_http("/raft/install_snapshot", &req).await.map_err(|e| RPCError::Network(e))
            }
        }
    }
}
