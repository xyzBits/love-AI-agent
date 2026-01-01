mod jsonrpc_http_practice;
mod jsonrpc_practice;
mod myrpc_ext;

use eyre::Ok;
use myrpc_ext::{MyRpcExt, MyRpcExtApiServer};
use reth_ethereum::evm::revm::primitives::hardfork::SpecId::OSAKA;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    println!("Hello, world!");

    Ok(())
}
