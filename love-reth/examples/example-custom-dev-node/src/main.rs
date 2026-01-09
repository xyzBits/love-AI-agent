//! This example shows how to run a custom dev node programmatically and submit a transaction
//! through rpc.

#![warn(unused_crate_dependencies)]

use std::sync::Arc;

use alloy_genesis::Genesis;
use alloy_primitives::{b256, hex};
use futures_util::StreamExt;
use reth_ethereum::{
    chainspec::ChainSpec,
    node::{
        EthereumNode,
        builder::{NodeBuilder, NodeHandle},
        core::{args::RpcServerArgs, node_config::NodeConfig},
    },
    provider::CanonStateSubscriptions,
    rpc::api::eth::helpers::EthTransactions,
    tasks::TaskManager,
};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    println!("Hello, world!");

    Ok(())
}

fn custom_chain() -> Arc<ChainSpec> {
    let custom_genesis = r#"
{
    "nonce": "0x42",
    "timestamp": "0x0",
    "extraData": "0x5343",
    "gasLimit": "0x5208",
    "difficulty": "0x400000000",
    "mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
    "coinbase": "0x0000000000000000000000000000000000000000",
    "alloc": {
        "0x6Be02d1d3665660d22FF9624b7BE0551ee1Ac91b": {
            "balance": "0x4a47e3c12448f4ad000000"
        }
    },
    "number": "0x0",
    "gasUsed": "0x0",
    "parentHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
    "config": {
        "ethash": {},
        "chainId": 2600,
        "homesteadBlock": 0,
        "eip150Block": 0,
        "eip155Block": 0,
        "eip158Block": 0,
        "byzantiumBlock": 0,
        "constantinopleBlock": 0,
        "petersburgBlock": 0,
        "istanbulBlock": 0,
        "berlinBlock": 0,
        "londonBlock": 0,
        "terminalTotalDifficulty": 0,
        "terminalTotalDifficultyPassed": true,
        "shanghaiTime": 0
    }
}
"#;
    let genesis: Genesis = serde_json::from_str(custom_genesis).unwrap();

    // 这里一定实现了 impl From<Genesis> for ChainSpec {
    Arc::new(genesis.into())
}
