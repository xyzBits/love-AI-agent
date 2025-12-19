# openraft-memory 架构设计文档

本文档旨在为 Raft 协议及 Rust 后端开发的初学者提供一个清晰的项目概览。我们将从核心概念出发，逐步深入到本项目的具体实现细节。

## 1. 什么是 Raft？

简单来说，Raft 是一种**共识算法**。在分布式系统中，多台服务器（节点）需要协同工作。Raft 的目标是让这组服务器像单台机器一样对外提供服务，即使部分机器宕机，整个集群依然能达成一致。

### 核心机制
- **选主 (Leader Election)**: 集群中总是有一个 Leader，负责处理所有请求。
- **日志复制 (Log Replication)**: Leader 接收数据，将其作为“日志条目”发送给其他节点（Followers）。
- **提交 (Commit)**: 当超过半数节点确认收到日志后，Leader 认为该日志已“提交”，并告知其他节点应用该日志。

---

## 2. 项目组件说明 (Rust 技术栈)

本项目利用了 Rust 生态中多个优秀的组件：

- **[OpenRaft](https://github.com/datafuselabs/openraft)**: 协议的核心实现。它处理了复杂的选举、同步、成员变更等逻辑，开发者只需实现其要求的“读写接口”即可。
- **[Tonic](https://github.com/hyperium/tonic)**: 这是一个高性能的 **gRPC** 框架。在本项目中，Raft 节点之间互发心跳、选票和日志条目，都是通过 Tonic 生成的 gRPC 服务完成的。
- **[Axum](https://github.com/tokio-rs/axum)**: 一个现代化的 **Web 框架**。我们通过 Axum 暴露了 RESTful API，让普通用户可以用 `curl` 或浏览器进行数据操作。
- **[Tokio](https://tokio.rs/)**: Rust 最主流的**异步运行时**。整个项目基于协程运行，能够并发处理上千个连接而不会阻塞线程。
- **[Serde](https://serde.rs/)**: 强大的序列化/反序列化库。我们用它将学生对象 (Student) 转换为 JSON 或二进制数据在网络上传输。

---

## 3. 详细架构流程

### 3.1 存储层 ([src/store/mod.rs](file:///c:/Users/lidf0/xyz/personal/language/rust/github/raft/openraft-memory/src/store/mod.rs))
存储层是数据的“港湾”。本项目使用了内存实现，主要包含两部分：
1. **LogStore (日志存储)**: 记录了 Raft 的所有历史操作。即使数据还没应用到数据库，只要在这里持久化了，Raft 就能保证它不丢失。
2. **StateMachine (状态机)**: 真实的业务数据库。当日志被“提交”后，会被应用到这里。本项目中就是一个 `HashMap<ID, Student>`。

### 3.2 网络层 ([src/network/mod.rs](file:///c:/Users/lidf0/xyz/personal/language/rust/github/raft/openraft-memory/src/network/mod.rs))
网络层是节点的“嘴巴”和“耳朵”：
- **NetworkFactory**: 负责根据节点 ID 查找 IP 地址并建立连接。
- **NetworkConnection**: 发送 AppendEntries (快照/日志同步) 和 Vote (请求投票) 的具体实现。

### 3.3 接口层 ([src/api/mod.rs](file:///c:/Users/lidf0/xyz/personal/language/rust/github/raft/openraft-memory/src/api/mod.rs))
这是系统的“大门”：
- **RaftGrpcServer**: 监听 50051 等端口，接收来自其他机器的消息。
- **Axum Router**: 监听 8081 等端口，接收普通用户的 HTTP 请求。

---

## 4. 一个写请求的生命周期

当您发送 `POST /student` 时，发生了什么？

1. **接收**: Axum 收到请求，解析成 `Student` 结构体。
2. **提交**: 调用 `raft.client_write(Request::Create(student))`。
3. **共识**: Leader (当前节点) 将此请求存入自己的 `LogStore`，同时通过网络层发给其他节点。
4. **决策**: 当 Leader 收到其他节点的回执，发现已经有“过半数”节点存好了这行日志。
5. **应用**: Leader 将日志传给 `Store::apply_to_state_machine`。
6. **落库**: `StateMachine` 将学生信息存入 HashMap。
7. **返回**: Leader 告诉用户“创建成功”。

---

## 5. 为什么本项目的 OpenRaft 实现比较特别？

我们在代码中使用了 **OpenRaft 0.9.x** 的最新特性：
- **Native Async**: 我们直接使用 Rust 原生的 `async fn` 定义接口，不需要额外的宏，运行效率更高。
- **Adaptor 模式**: 我们实现了一个统一的 `Store`，然后通过 `Adaptor` 自动将其拆分为 OpenRaft 需要的日志流和状态机流。这种设计让代码结构非常清晰。

希望这份文档能帮助您快速上手！如果您有任何问题，请阅读代码中详尽的中文注释。
