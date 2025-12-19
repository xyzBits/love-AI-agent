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

## 3. 核心架构与端口分配

为了职责清晰，本项目将业务接口与 Raft 内部通信接口进行了物理隔离（不同端口）。

### 端口映射表

| 节点 ID | Raft 端口 (内网/Raft RPC) | Student gRPC 端口 (外网/业务) | 业务 HTTP 端口 (外网) |
| :--- | :--- | :--- | :--- |
| 配置项 | `raft_grpc_port` | `business_grpc_port` | `business_http_port` |
| **节点 1** | **50051** | **60051** | **8081** |
| **节点 2** | **50052** | **60052** | **8082** |
| **节点 3** | **50053** | **60053** | **8083** |

> [!IMPORTANT]
> **端口隔离说明**：
> - 当 `RAFT_PROTOCOL=grpc` 时，`raft_grpc_port` 运行 gRPC 服务。
> - 当 `RAFT_PROTOCOL=http` 时，`raft_grpc_port` 运行独立的 HTTP 服务，仅响应 `/raft/*` 路径。
> - 业务逻辑始终运行在 `business_http_port` (808x) 上。

---

## 4. HTTP API 接口参考

### 4.1 业务接口 (Port: 808x)

这些接口用于外部客户端管理学生数据及查询集群状态。

| 动作 | 路径 | 描述 | 请求体 (JSON) / 参数 |
| :--- | :--- | :--- | :--- |
| **POST** | `/student` | 创建学生信息 | `{"id": 101, "name": "张三", "score": 95.5, ...}` |
| **GET** | `/student/:id` | 获取学生信息 | URL 参数 `id` |
| **DELETE** | `/student/:id` | 删除学生信息 | URL 参数 `id` |
| **GET** | `/cluster/info` | 查询集群拓扑与状态 | 无 |

### 4.2 Raft 内部接口 (Port: 5005x)

*注：仅在 `RAFT_PROTOCOL=http` 时启用。*

| 路径 | 描述 | 说明 |
| :--- | :--- | :--- |
| `/raft/append_entries` | 日志复制/心跳 | 由 Leader 调用，同步数据给 Follower |
| `/raft/vote` | 投票请求 | 候选者在选举期间发起 |
| `/raft/install_snapshot` | 安装快照 | 用于同步大量滞后数据 |

---

## 5. 运行与操作指南

### 启动节点

```powershell
# 启动节点 1 (默认)
$env:NODE_ID="1"; cargo run

# 切换协议启动节点 2 (HTTP 模式)
$env:NODE_ID="2"; $env:RAFT_PROTOCOL="http"; cargo run
```

### 业务接口测试示例 (HTTP)

```bash
# 1. 创建学生
curl -X POST http://127.0.0.1:8081/student \
     -H "Content-Type: application/json" \
     -d '{"id": 101, "name": "张三", "age": 20, "gender": "男", "score": 95.5}'

# 2. 查询学生
curl http://127.0.0.1:8081/student/101

# 3. 查询集群状态
curl http://127.0.0.1:8081/cluster/info | jq
```

### 业务接口测试示例 (gRPC)

```bash
# 使用 grpcurl 访问业务 gRPC (60051)
grpcurl -plaintext -import-path ./proto -proto raft.proto \
    -d '{"student": {"id": 102, "name": "李四", "age": 21, "gender": "女", "score": 88.0}}' \
    127.0.0.1:60051 raft_service.StudentService/CreateStudent
```

---

## 6. 一个请求的生命周期 (Raft 流程)

1. **接收**: 业务 HTTP (808x) 收到 `POST /student`。
2. **提交**: 调用 `raft.client_write(...)`。
3. **共识**: Leader 通过 Raft 端口 (5005x) 与其他节点通信（gRPC 或 HTTP）。
4. **提交**: 多数派确认后，日志在所有节点的状态机中应用。
5. **返回**: 业务接口返回成功响应。

希望这份文档能帮助您快速上手分布式一致性系统！
