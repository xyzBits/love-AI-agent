use std::sync::OnceLock;

use alloy_evm::{
    EthEvm, EvmFactory,
    eth::EthEvmContext,
    precompiles::PrecompilesMap,
    revm::{
        Context, MainBuilder, MainContext,
        context::{
            BlockEnv, TxEnv,
            result::{EVMError, HaltReason},
        },
        handler::EthPrecompiles,
        inspector::NoOpInspector,
        precompile::{Precompile, PrecompileId, PrecompileOutput, PrecompileResult, Precompiles},
        primitives::hardfork::SpecId,
    },
};
use alloy_genesis::Genesis;
use alloy_primitives::{Bytes, address};
use reth_ethereum::{
    EthPrimitives,
    chainspec::{Chain, ChainSpec},
    evm::EthEvmConfig,
    node::{
        EthereumAddOns, EthereumNode,
        api::{FullNodeTypes, NodeTypes},
        builder::{NodeBuilder, NodeConfig, components::ExecutorBuilder},
        core::args::RpcServerArgs,
    },
    tasks::TaskManager,
};

use alloy_evm::revm::precompile::PrecompileError;
use reth_tracing::{RethTracer, Tracer};

mod practice_lib;

/// 单元结构体，空的结构体
/// rust 中，结构体中不一定要存数据，它也可以仅仅用来承载行为
/// 不占用内存，只是一个代号
#[derive(Debug, Clone, Default)]
#[non_exhaustive] // 这个结构体或者枚举的内容目前是这样，但在未来可能会增加新的字段，不要以为它永远是空的
//  加上后，外部无法直接实例化，left f = MyEvmFactory 会报错
pub struct MyEvmFactory;

/// EvmFactory 是 alloy 定义的一个接口，告诉系统，当我要执行交易时，请用这个逻辑给我造一个 EVM 出来
impl EvmFactory for MyEvmFactory {
    /// 1. 关联类型定义 associated types
    /// 这里定义了 EVM 运行时需要的各种组件的具体类型
    /// 大部分都直接使用了 alloy_evm 和 revm 提供的标准类型，如 EthEvm EthEvm Context
    /// 这个工厂方法没有重写 EVM 的解释逻辑，它只是组装  EVM
    /// 它拦截了组装过程，替换自定义的合约预编译列表
    ///
    /// 预编译合约是以太坊的一种特殊合约，它们不是用 solidity 写的字节码，而是直接用客户端 语言，这里是rust写的原生函数
    /// 通常用于通过地址直接调用复杂的加密算法
    type Evm<DB: alloy_evm::Database, I: alloy_evm::revm::Inspector<Self::Context<DB>>> =
        EthEvm<DB, I, Self::Precompiles>;

    type Context<DB: alloy_evm::Database> = EthEvmContext<DB>;

    type Tx = TxEnv; // 交易环境

    type Error<DBError: std::error::Error + Send + Sync + 'static> = EVMError<DBError>;

    type HaltReason = HaltReason;

    type Spec = SpecId; // 硬分叉规范 ID 如 cancun prague

    type BlockEnv = BlockEnv;

    type Precompiles = PrecompilesMap;

    /// 核心方法，创建 EVM
    fn create_evm<DB: alloy_evm::Database>(
        &self,
        db: DB,                                               // 数据库接口，读取余额、代码等
        input: alloy_evm::EvmEnv<Self::Spec, Self::BlockEnv>, // 环境参数（区块信息、配置）
    ) -> Self::Evm<DB, alloy_evm::revm::inspector::NoOpInspector> {
        let spec = input.cfg_env.spec; // 获取当前区块的硬分叉版本

        // A. 构建器模式 builder pattern 构建  evm 上下文
        let mut evm = Context::mainnet()
            .with_db(db)
            .with_cfg(input.cfg_env)
            .with_block(input.block_env)
            .build_mainnet_with_inspector(NoOpInspector {}) // 不带检查器，debugger
            // 加载默认的以太坊预编译合约，如 ecrecover sha256
            .with_precompiles(PrecompilesMap::from_static(
                EthPrecompiles::default().precompiles,
            ));

        // C. 自定义逻辑：如果是 prague 硬分叉
        if spec == SpecId::PRAGUE {
            // 加载我们要注入的私货，prague_cuscom
            evm = evm.with_precompiles(PrecompilesMap::from_static(prague_custom()));
        }

        // D. 返回封装好的 EVM
        EthEvm::new(evm, false)
    }

    fn create_evm_with_inspector<
        DB: alloy_evm::Database,
        I: alloy_evm::revm::Inspector<Self::Context<DB>>,
    >(
        &self,
        db: DB,
        input: alloy_evm::EvmEnv<Self::Spec, Self::BlockEnv>,
        inspector: I,
    ) -> Self::Evm<DB, I> {
        EthEvm::new(
            self.create_evm(db, input)
                .into_inner()
                .with_inspector(inspector),
            true,
        )
    }
}

#[derive(Debug, Default, Clone, Copy)]
#[non_exhaustive]
pub struct MyExecutorBuilder;

impl<Node> ExecutorBuilder<Node> for MyExecutorBuilder
where
    Node: FullNodeTypes<Types: NodeTypes<ChainSpec = ChainSpec, Primitives = EthPrimitives>>,
{
    type EVM = EthEvmConfig<ChainSpec, MyEvmFactory>;

    async fn build_evm(
        self,
        ctx: &reth_ethereum::node::builder::BuilderContext<Node>,
    ) -> eyre::Result<Self::EVM> {
        let evm_config =
            EthEvmConfig::new_with_evm_factory(ctx.chain_spec(), MyEvmFactory::default());

        Ok(evm_config)
    }
}

pub fn prague_custom() -> &'static Precompiles {
    // 1. OnceLock 实现单例模式 Singleton
    // 预编译合约列表是静态的、只读的，没有必要每次创建 EVM 都重新分配内存
    // OnceLock 保证这段代码只会在第一次调用时执行一次，后续直接返回引用
    static INSTANCE: OnceLock<Precompiles> = OnceLock::new();

    INSTANCE.get_or_init(|| {
        // 2. 复制一份标准的 Prague 预编译列表
        let mut precompiles = Precompiles::prague().clone();

        // custom precompile
        // 3. 定义我们自己的预编译合约
        /*let precompile = Precompile::new(
            PrecompileId::custom("custom"),
            address!("0x0000000000000000000000000000000000000999"),
            // 这是一个最简单的逻辑：直接返回成功
            // 消耗 0 gas 返回空的 bytes
            // |_, _| PrecompileResult::Ok(PrecompileOutput::new(0, Bytes::new())),
            |_, _| PrecompileResult::Ok(PrecompileOutput::new(0, Bytes::from("Hello Reth!"))),
        );*/

        // custom precompile
        let precompile = Precompile::new(
            PrecompileId::custom("custom"),
            address!("0x0000000000000000000000000000000000000999"),
            // ⬇️⬇️⬇️ 核心逻辑就在这里 ⬇️⬇️⬇️
            |input: &[u8], _gas_limit: u64| -> PrecompileResult {
                // 1. 检查输入长度
                if input.len() < 16 {
                    // ❌ 之前的写法 (错误):
                    // return Err(PrecompileError::Other("...".into()).into());

                    // ✅ 现在的写法 (正确):
                    // 直接返回 PrecompileError，不要再转了
                    return Err(PrecompileError::Other(
                        "Input must be at least 16 bytes".into(),
                    ));
                }

                // 2. 解析数据
                let a_bytes: [u8; 8] = input[0..8].try_into().unwrap();
                let b_bytes: [u8; 8] = input[8..16].try_into().unwrap();

                // 3. 转成数字
                let a = u64::from_be_bytes(a_bytes);
                let b = u64::from_be_bytes(b_bytes);

                // 4. 执行加法
                let sum = a.wrapping_add(b);
                println!("正在执行加法: {} + {} = {}", a, b, sum);

                // 5. 返回结果
                Ok(PrecompileOutput::new(
                    100,
                    Bytes::from(sum.to_be_bytes().to_vec()),
                ))
            }, // ⬆️⬆️⬆️ 逻辑结束 ⬆️⬆️⬆️
        );

        // 4. 将自定义的合约加入列表
        precompiles.extend([precompile]);
        precompiles
    })
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // 执行动作的函数，不需要返回任何数据()就代表执行成功，Err就代表失败
    let _f = MyEvmFactory;

    // 1. 开启日志系统 log
    // Reth 的监控探头
    // 后面节点运行时的所有日志 info debug error 都会被记录下来
    let _guard = RethTracer::new().init()?;

    // 2. 获取任务管理器
    // Reth 是异步的，它需要一个 工头 来管理成千上万个并发任务
    let tasks = TaskManager::current();

    // 定义区块链规则
    let spec = ChainSpec::builder()
        .chain(Chain::mainnet()) // 基础基因：基于以太坊
        .genesis(Genesis::default()) // 创建区块：使用默认的
        // 激活各种硬分叉版本
        .london_activated()
        .paris_activated()
        .shanghai_activated()
        .cancun_activated()
        .prague_activated()
        .build();

    // test 表示这是一个测试节点
    // 它的数据库是临时的，关机就删除，不像正式节点存在磁盘上
    let node_config = NodeConfig::test()
        // 开启rpc 服务
        // 这样你才能通过 metamask curl 向它发请求
        .with_rpc(RpcServerArgs::default().with_http())
        // 把链的规则放进去
        .with_chain(spec);

    let handle = NodeBuilder::new(node_config)
        .testing_node(tasks.executor())
        // 指定类型：要做的是一个标准的以太坊节点，以也可用 op stack 或 base 节点，
        .with_types::<EthereumNode>()
        // 注入组件，核心中的核心
        // EthereumNode::components 返回一套标准组件，网络、交易池、共识
        .with_components(
            EthereumNode::components()
                // 把执行器换成了我们自己写的 MyExecutorBuilder
                // 之后节点执行交易时，会用你的 MyEvmFactory
                // 从而加载你的 prague_custom 预编译合约
                .executor(MyExecutorBuilder::default()),
        )
        // 添加插件
        // 比如 json-rpc 接口的具体实现
        .with_add_ons(EthereumAddOns::default())
        .launch()
        .await
        .unwrap();

    println!("Node started");

    // 一个永远等待的 Future，除非节点崩溃或者 ctrl+c，否则程序会一直卡在这里，保持运行
    handle.node_exit_future.await
}
