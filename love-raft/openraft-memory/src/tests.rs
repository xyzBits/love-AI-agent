#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashMap};
    use std::sync::Arc;
    use openraft::{Config, Raft};
    use crate::model::{Request, Student, TypeConfig};
    use crate::store::Store;
    use crate::network::NetworkFactory;
    use openraft::storage::Adaptor;

    #[tokio::test]
    async fn test_single_node_crud() -> anyhow::Result<()> {
        let node_id = 1;
        let raft_config = Arc::new(Config::default());
        
        let store = Store::new();
        let (log_store, state_machine) = Adaptor::new(store.clone());

        let network = NetworkFactory {
            node_addresses: Arc::new(HashMap::new()),
            protocol: crate::config::RaftProtocol::Grpc,
        };

        let raft = Raft::new(node_id, raft_config, network, log_store, state_machine).await?;

        // 初始化单节点集群
        let mut nodes = BTreeMap::new();
        nodes.insert(1, openraft::impls::EmptyNode {});
        raft.initialize(nodes).await?;

        // 等待成为 Leader
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Create
        let student = Student {
            id: 1,
            name: "Alice".to_string(),
            age: 20,
            gender: "Female".to_string(),
            score: 95.0,
        };
        let resp = raft.client_write(Request::Create(student.clone())).await?.data;
        assert!(resp.success);
        assert_eq!(resp.data.unwrap().name, "Alice");

        // Read (直接从状态机读取，实际应该通过 raft.read_index)
        {
            let sm = store.state_machine.read().await;
            assert_eq!(sm.data.get(&1).unwrap().name, "Alice");
        }

        // Update
        let mut updated_student = student.clone();
        updated_student.score = 100.0;
        let resp = raft.client_write(Request::Update(updated_student)).await?.data;
        assert!(resp.success);

        // Delete
        let resp = raft.client_write(Request::Delete(1)).await?.data;
        assert!(resp.success);

        Ok(())
    }
}
