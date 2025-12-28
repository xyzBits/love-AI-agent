这份**“Reth 全栈开发环境搭建终极指南”**已经为你整合了 WSL2 安装、网络优化（VPN 镜像模式）以及之前的编译配置全流程。

内容经过了重新编排，逻辑更顺畅，非常适合复制保存到你的 Notion 或本地笔记中存档。

---

# 📘 Reth 区块链开发环境搭建终极指南 (Windows 11 + WSL2)

**适用环境**：Windows 11 (推荐 22H2 及以上版本)
**目标系统**：Ubuntu 22.04 LTS
**核心目标**：构建一个网络通畅、编译速度极快、工具链完善的 Rust + Ethereum 开发环境。

---

### 第一阶段：WSL2 基础环境安装

#### 1. 一键安装 WSL2

在 Windows 上以**管理员身份**打开 PowerShell，执行：

```powershell
wsl --install

```

* **操作**：命令执行完毕后，**重启电脑**。
* **初始化**：重启后会自动弹出 Ubuntu 终端窗口，按提示设置你的 `Username` (用户名) 和 `Password` (密码)。

#### 2. 开启“镜像网络模式” (解决 VPN 痛点) 🔥

**这是 Windows 11 的杀手级功能**。开启后，WSL2 将与 Windows 共享 IP，完美兼容 Windows 上的 VPN 软件（访问 GitHub 走代理，访问国内源走直连）。

1. 在 Windows 资源管理器地址栏输入 `%UserProfile%` 进入用户主目录。
2. 新建文件 **`.wslconfig`** (注意前面有个点)。
3. 用记事本打开，粘贴以下内容：

```ini
[wsl2]
networkingMode=mirrored
dnsTunneling=true
autoProxy=true
ipv6=true

```

4. **重启 WSL 生效**：在 PowerShell 执行 `wsl --shutdown`。

---

### 第二阶段：Ubuntu 系统配置 (换源与依赖)

#### 1. 替换阿里云源 (解决 apt 下载慢)

进入 Ubuntu 终端，执行以下命令将软件源替换为国内最稳的阿里云：

```bash
# 1. 备份原配置
sudo cp /etc/apt/sources.list /etc/apt/sources.list.bak

# 2. 覆盖写入阿里云源 (针对 Ubuntu 22.04 Jammy)
sudo tee /etc/apt/sources.list <<EOF
deb http://mirrors.aliyun.com/ubuntu/ jammy main restricted universe multiverse
deb-src http://mirrors.aliyun.com/ubuntu/ jammy main restricted universe multiverse
deb http://mirrors.aliyun.com/ubuntu/ jammy-security main restricted universe multiverse
deb-src http://mirrors.aliyun.com/ubuntu/ jammy-security main restricted universe multiverse
deb http://mirrors.aliyun.com/ubuntu/ jammy-updates main restricted universe multiverse
deb-src http://mirrors.aliyun.com/ubuntu/ jammy-updates main restricted universe multiverse
deb http://mirrors.aliyun.com/ubuntu/ jammy-backports main restricted universe multiverse
deb-src http://mirrors.aliyun.com/ubuntu/ jammy-backports main restricted universe multiverse
EOF

# 3. 更新缓存
sudo apt update

```

#### 2. 安装核心编译工具链

安装 GCC、Git、OpenSSL 以及 **Reth 必须的 Clang**。

```bash
# 技巧：添加 -o Acquire::ForceIPv4=true 强制走 IPv4，防止进度条卡死
sudo apt -o Acquire::ForceIPv4=true install \
    build-essential git curl \
    libssl-dev pkg-config \
    clang libclang-dev -y

```

---

### 第三阶段：Rust 开发环境 (国内加速)

#### 1. 安装 Rustup (使用国内镜像)

默认脚本连国外很慢，我们强制指定走**字节跳动/Rsproxy** 镜像。

```bash
# 1. 临时设置环境变量
export RUSTUP_DIST_SERVER="https://rsproxy.cn/rustup"
export RUSTUP_UPDATE_ROOT="https://rsproxy.cn/rustup/rustup"

# 2. 运行安装脚本
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# (出现提示时，输入 1 回车)

# 3. 激活环境变量
source "$HOME/.cargo/env"

```

#### 2. 配置 Cargo 下载源 (永久生效)

解决 `cargo build` 下载依赖包卡住的问题。

1. 新建配置：`mkdir -p ~/.cargo && nano ~/.cargo/config.toml`
2. 粘贴内容：

```toml
[source.crates-io]
replace-with = 'rsproxy-sparse'
[source.rsproxy]
registry = "https://rsproxy.cn/crates.io-index"
[source.rsproxy-sparse]
registry = "sparse+https://rsproxy.cn/index/"
[registries.crates-io]
protocol = "sparse"

```

---

### 第四阶段：IDE 与代码获取

#### 1. VS Code 配置

在 Windows 上安装 VS Code，并安装以下必备插件：

* **WSL** (Microsoft 出品)：连接 Ubuntu 的桥梁。
* **rust-analyzer**：Rust 语法高亮与智能提示 (点击 "Install in WSL: Ubuntu")。

#### 2. 下载 Reth 源码

* **网络策略**：此时建议**开启 VPN**（得益于镜像模式，git clone 会自动走代理）。

```bash
cd ~
git clone https://github.com/paradigmxyz/reth.git
cd reth

```

#### 3. 编译高性能版本

* **网络策略**：此时建议**关闭 VPN**（让 Cargo 走国内字节源直连，速度最快）。

```bash
# --release: 优化版本，运行快但编译慢
# --bin reth: 只编译主程序
cargo build --release --bin reth

```

---

### 第五阶段：运行与验证

#### 1. 启动本地开发节点

使用 `--dev` 模式启动一个带挖矿功能的测试链。

```bash
# --dev: 开发模式
# --http: 开启 RPC 端口 (8545)
# --datadir: 指定数据目录 (推荐，防止重启丢失数据)
./target/release/reth node --dev --http --datadir ~/.local/share/reth/dev

```

#### 2. 验证节点状态

打开一个新的 WSL 终端窗口，查询 Chain ID：

```bash
curl -X POST -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}' \
  http://127.0.0.1:8545

```

* **成功标志**：返回 `{"result":"0x539"}` (即十进制 1337)。

---

### 💡 避坑锦囊 (Troubleshooting)

1. **关于 VPN 的开关时机**：
* **开启镜像网络模式后**：大部分时间可以开着 VPN。
* **唯独例外**：在使用 `apt install` (阿里云源) 或 `cargo build` (字节源) 时，如果发现速度不稳定，**彻底退出 VPN** 通常能跑满物理带宽。


2. **DNS 解析失败**：
* 如果报错 `Temporary failure in name resolution`，且关 VPN 无效，请检查 `/etc/resolv.conf`，强制写入 `nameserver 223.5.5.5`。


3. **编译报错 `Unable to find libclang**`：
* 这是因为没装 `libclang-dev`。请重新执行第二阶段的 `apt install` 命令。


4. **节点数据去哪了？**
* 如果不加 `--datadir`，数据在 `/tmp` 下，重启即焚。
* 加上 `--datadir`，数据永久保存，重启节点会自动读取旧区块。