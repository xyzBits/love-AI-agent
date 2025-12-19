# openraft-memory 实现计划

本项目旨在使用 `openraft` 库在 Rust 中实现一个支持 REST 和 gRPC 接口的分布式 Student 对象 Key-Value 存储系统。

## 用户评审需求

> [!IMPORTANT]
> 1. 数据存储初期使用内存，后续支持 `sled`。
> 2. 节点间通信使用 gRPC。
> 3. 提供 Student 对象的 CRUD 操作。

## 方案设计

### 1. 技术栈
- **Raft 框架**: `openraft`
- **通信 (RPC)**: `tonic` (gRPC 实现)
- **REST 接口**: `axum` 或 `actix-web`
- **序列化**: `serde`, `prost`
- **配置**: `config` 或 `toml`

### 2. 核心组件
- **LogStore**: 实现 Raft 日志持久化（内存版）。
- **StateMachine**: 实现 Raft 状态机，维护 Student 信息的内存映射。
- **Network**: 实现 `RaftNetwork` 接口，使用 gRPC 进行节点间心跳和数据同步。
- **Server**: 同时启动 gRPC 服务（用于 Raft 通信和客户端 CRUD）和 HTTP 服务（REST CRUD）。

### 3. 数据模型
`Student` 结构体：
```rust
pub struct Student {
    pub id: i64,
    pub name: String,
    pub age: i32,
    pub gender: String,
    pub score: f32,
}
```

## 变更内容

### openraft-memory

#### [MODIFY] [Cargo.toml](file:///c:/Users/lidf0/xyz/personal/language/rust/github/raft/openraft-memory/Cargo.toml)
添加 `openraft`, `tonic`, `axum`, `tokio`, `serde` 等依赖。

#### [NEW] [src/main.rs](file:///c:/Users/lidf0/xyz/personal/language/rust/github/raft/openraft-memory/src/main.rs)
应用入口，初始化配置与服务启动。

#### [NEW] [src/model.rs](file:///c:/Users/lidf0/xyz/personal/language/rust/github/raft/openraft-memory/src/model.rs)
定义数据结构与协议。

#### [NEW] [src/store/](file:///c:/Users/lidf0/xyz/personal/language/rust/github/raft/openraft-memory/src/store/)
实现 `LogStore` 与 `StateMachine`。

#### [NEW] [src/network/](file:///c:/Users/lidf0/xyz/personal/language/rust/github/raft/openraft-memory/src/network/)
基于 gRPC 的 Raft 通信网络实现。

#### [NEW] [src/api/](file:///c:/Users/lidf0/xyz/personal/language/rust/github/raft/openraft-memory/src/api/)
实现 REST 与客户端 gRPC 服务。

## 验证计划

### 自动化测试
- 编写集成测试，在一个进程内通过多线程/多运行环境模拟 Raft 三节点集群。
- 使用客户端发送 CRUD 请求并验证一致性。

### 手动验证
- 启动多个独立实例，模拟网络分区或宕机，观察一致性恢复。
