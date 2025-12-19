# openraft-memory 架构设计与实现文档

本文档旨在为 Raft 协议及 Rust 后端开发的初学者提供一个清晰的项目概览。我们将从核心概念出发，分析组件职责，并展示核心代码示例。

## 1. 什么是 Raft？

简单来说，Raft 是一种**共识算法**。在分布式系统中，多台服务器（节点）需要协同工作。Raft 的目标是让这组服务器像单台机器一样对外提供服务，即使部分机器宕机，整个集群依然能达成一致。

### 核心机制
- **选主 (Leader Election)**: 集群中总是有一个 Leader，负责处理所有写请求。
- **日志复制 (Log Replication)**: Leader 接收数据，将其作为“日志条目”发送给其他节点（Followers）。
- **提交 (Commit)**: 当超过半数节点确认收到日志后，Leader 认为该日志已“提交”，并告知其他节点应用该日志。

---

## 2. 项目组件说明 (Rust 技术栈)

本项目利用了 Rust 生态中多个优秀的组件：

- **[OpenRaft](https://github.com/datafuselabs/openraft)**: 协议的核心实现。
- **[Tonic](https://github.com/hyperium/tonic)**: 高性能 **gRPC** 框架，用于节点间通信。
- **[Axum](https://github.com/tokio-rs/axum)**: 现代化的 **Web 框架**，暴露 RESTful API。
- **[Tokio](https://tokio.rs/)**: Rust 最主流的**异步运行时**。
- **[Serde](https://serde.rs/)**: 强大的序列化/反序列化库。

---

## 3. 详细架构流程

### 3.1 核心数据结构 ([src/model/mod.rs](src/model/mod.rs))

我们需要定义 Raft 节点之间传递的消息格式：

```rust
// 定义业务数据：学生模型
pub struct Student {
    pub id: i64,
    pub name: String,
    pub score: f32,
}

// 定义 Raft 输入请求
pub enum Request {
    Create(Student),
    Update(Student),
    Delete(i64),
}
```

### 3.2 存储层 ([src/store/mod.rs](src/store/mod.rs))

存储层是数据的“港湾”。本项目使用内存实现，包含两个核心功能：

1. **LogStore (日志存储)**: 记录所有历史操作，保证数据不丢失。
2. **StateMachine (状态机)**: 真实的业务数据库 (一个 `HashMap`)。

```rust
// 状态机应用逻辑示例
async fn apply_to_state_machine(&mut self, entries: &[Entry<TypeConfig>]) -> Result<Vec<Response>, StorageError<u64>> {
    let mut sm = self.state_machine.write().await;
    for entry in entries {
        if let EntryPayload::Normal(Request::Create(std)) = &entry.payload {
            sm.data.insert(std.id, std.clone()); // 真正写入数据库
        }
    }
}
```

### 3.3 网络层 ([src/network/mod.rs](src/network/mod.rs))

网络层负责节点间的“握手”与“交谈”。节点间使用 gRPC 通信：

```rust
// 发送 AppendEntries 请求给其他节点
async fn append_entries(&mut self, req: AppendEntriesRequest<TypeConfig>) -> Result<AppendEntriesResponse<u64>, ...> {
    let client = RaftServiceClient::connect(self.addr.clone()).await?;
    let json_data = serde_json::to_string(&req)?; // 序列化
    let pb_req = PbAppendEntriesRequest { data: json_data };
    client.append_entries(pb_req).await // 发送
}
```

### 3.4 启动与初始化 ([src/main.rs](src/main.rs))

在 `main` 函数中，我们将各个组件连接起来：

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let store = Store::new(); // 1. 创建存储
    let network = NetworkFactory { ... }; // 2. 创建网络
    let raft = Raft::new(node_id, config, network, store_adaptor).await?; // 3. 创建 Raft 引擎
    
    // 4. 启动 REST API 和 gRPC Server
    tokio::spawn(axum::serve(listener, app));
    tokio::spawn(tonic_server.serve(grpc_addr));
}
```

---

## 4. 一个写请求的生命周期

当您发送 `POST /student` 时，发生了什么？

1. **接收**: Axum 收到请求，解析成 `Student` 结构体。
2. **提交**: 调用 `raft.client_write(Request::Create(student))`。
3. **共识**: Leader (当前节点) 将此请求存入自己的 `LogStore`，同时发送给 Followers。
4. **决策**: 超过“半数”节点确认后，Leader 标记日志为“已提交”。
5. **应用**: 日志被传给 `StateMachine`，学生信息存入 HashMap。
6. **返回**: Leader 告知用户“操作执行成功”。

---

## 5. 为什么本项目的 OpenRaft 实现比较特别？

- **Native Async (原生异步)**: 基于 Rust 1.75+ 的原生异步特性，不再需要巨大的宏。
- **Adaptor 模式**: 我们实现了一个统一的 `Store`，通过 `Adaptor` 自动桥接到 OpenRaft 内部。
- **详细代码引用**: 
    - 更多细节请参考 [src/store/mod.rs](src/store/mod.rs) 中的中文注释。
    - 接口定义见 [src/api/mod.rs](src/api/mod.rs)。

希望这份文档能帮助您快速上手分布式一致性系统！
