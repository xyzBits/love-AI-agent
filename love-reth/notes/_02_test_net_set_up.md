既然你已经成功跑通了从节点启动到工具验证的全流程，我们将这一套 **Reth + Foundry (Cast)** 的硬核开发环境搭建过程总结如下。

这份总结可以直接作为你的“实验笔记”，方便日后复速查。

---

### 第一阶段：Reth 节点的编译与后台启动

这是你的“数据引擎”，为后续所有开发提供真实的区块链环境。

#### 1. 编译 Release 版本（核心：性能优化）

在 Reth 源码根目录下执行。必须使用 `--release`，否则同步速度无法忍受。

```bash
cargo build --release -p reth

```

#### 2. 挂后台启动同步（使用 tmux）

使用 `tmux` 可以保证你关闭终端后节点依然运行，且随时可以查看日志。

```bash
# 创建并进入名为 reth_node 的会话
tmux new -s reth_node

# 启动 Sepolia 同步（开启 HTTP RPC）
RUST_LOG=info ./target/release/reth node \
    --chain sepolia \
    --datadir ~/.local/share/reth/sepolia \
    --http \
    --http.api eth,net,web3,debug,trace \
    --ws

# 按 Ctrl + B，然后按 D 键，退回后台

```

#### 3. 查看与监控

* **重回日志界面**：`tmux attach -t reth_node`
* **查看磁盘占用**：`du -h --max-depth=1 ~/.local/share/reth/sepolia | sort -hr`

---

### 第二阶段：Foundry (Cast) 工具链安装

这是你的“瑞士军刀”，用于和节点进行 RPC 通信。

#### 1. 安装 Foundryup (安装器)

```bash
curl -L https://foundry.paradigm.xyz | bash

```

#### 2. 配置环境变量（关键：防止 command not found）

在 `~/.bashrc` (或 `~/.zshrc`) 末尾添加：

```bash
export PATH="$PATH:$HOME/.foundry/bin"

```

执行 `source ~/.bashrc` 使其立即生效。

#### 3. 安装工具集

```bash
foundryup

```

---

### 第三阶段：节点连接与数据验证

使用 `cast` 验证节点是否工作正常。

#### 1. 验证创世区块（Genesis Block）

查询 Sepolia 的 0 号区块，确认 Hash 是否符合官方标准。

```bash
# 设置本地 RPC 地址环境变量，简化后续命令
export ETH_RPC_URL=http://localhost:8545

# 查询创世块 Hash
cast block 0 --field hash
# 预期输出：0x25a5cc106eea7138acab33231d7160d69cb777ee0c2c553fcddf5138993e6dd9

```

#### 2. 检查同步高度

```bash
cast block-number

```

如果这个数字从 0 开始不断变大，说明节点已经开始从邻居那里下载并执行区块了。

#### 3. 时间戳转换（实战小技巧）

```bash
# 获取创世时间戳并转为人类可读格式
cast block 0 --field timestamp | xargs cast from-unix

```

---

### 💡 避坑小结

* **Debug vs Release**：永远不要在 Debug 模式下运行节点同步。
* **WSL 磁盘空间**：时刻关注 `du -sh` 的结果，Reth Sepolia 节点大约需要 300GB+ 空间。
* **RPC 访问**：启动命令中必须包含 `--http`，否则 `cast` 无法连接本地节点。
* **环境变量**：安装 Foundry 后如果命令不生效，优先检查 `~/.bashrc` 中的 `PATH` 配置。

**目前你的 Sepolia 节点正在后台静默同步，当你发现 `cast block-number` 达到几万甚至几十万时，你就可以开始正式进入 `db-access` 的代码学习了！**