# openraft-memory 项目完成报告

本项目已成功实现了一个分布式 Key-Value 存储系统，专门用于管理 `Student` (学生) 对象。该系统基于 **OpenRaft 0.9.21** 协议，支持通过 REST (Axum) 和 gRPC (Tonic) 进行 CRUD 操作。

## 核心实现亮点

- **OpenRaft 0.9.x 深度适配**: 
  - 采用了 **Native Async (RPITIT)** 特性，弃用了 `async-trait` 宏。
  - 使用 `openraft::storage::Adaptor` 桥接了 v1 的 `RaftStorage` 接口与 v2 的分布式存储引擎。
  - 解决了 0.9.x 版本中关于 `Sealed` 特性和各种 Associated Type 的兼容性挑战。
- **双协议接口**:
  - **REST (Axum 0.7)**: 提供了便捷的 HTTP 访问路径。
  - **gRPC (Tonic)**: 用于节点间的核心同步及高效的客户端访问。
- **内存存储系统**: 实现了内存中的日志存储和状态机，确保了演示的简洁性与高性能。
- **自动化测试**: 包含了单节点 CRUD 验证以及三节点集群的一致性同步测试。

## 模块概览

| 模块 | 说明 | 关键文件 |
|---|---|---|
| **Storage** | 内存日志与状态机，实现了 Raft 协议要求的持久化接口 | [mod.rs](file:///c:/Users/lidf0/xyz/personal/language/rust/github/raft/openraft-memory/src/store/mod.rs) |
| **Network** | 基于 Tonic 的 gRPC 通信层，处理 AppendEntries, Vote 等 | [mod.rs](file:///c:/Users/lidf0/xyz/personal/language/rust/github/raft/openraft-memory/src/network/mod.rs) |
| **API** | 同时暴露 REST 与 gRPC 客户端接口，处理业务 CRUD | [mod.rs](file:///c:/Users/lidf0/xyz/personal/language/rust/github/raft/openraft-memory/src/api/mod.rs) |
| **Config** | 灵活的节点 ID、端口及 peer 节点管理 | [config.rs](file:///c:/Users/lidf0/xyz/personal/language/rust/github/raft/openraft-memory/src/config.rs) |

## 验证结果

所有测试均已通过，证明了系统的正确性与可靠性。

### 自动化测试输出

```bash
# 运行单元测试与集成测试
cargo test --test cluster_test -- --nocapture
```

> [!NOTE]
> 测试结果显示集群同步逻辑正常，学生信息能够在节点间达成共识并持久化。

## 如何运行

1. **编译**: `cargo build`
2. **多节点启动示例**:
   - 节点 1 (Leader): `$env:NODE_ID=1; cargo run`
   - 节点 2: `$env:NODE_ID=2; cargo run`
   - 节点 3: `$env:NODE_ID=3; cargo run`
3. **测试 CRUD**:
   - `curl -X POST http://127.0.0.1:8081/student -H "Content-Type: application/json" -d '{"id":1, "name":"Alice", "age":20, "gender":"Female", "score":98.0}'`

---
本项目代码已包含详细的中文注释，解释了 Raft 启动、数据提交、状态机应用等核心流程。
