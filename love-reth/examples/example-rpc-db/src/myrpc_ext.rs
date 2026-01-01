use reth_ethereum::{
    Block,
    provider::{self, BlockReaderIdExt},
    rpc::eth::EthResult,
};

use jsonrpsee::proc_macros::rpc;

/// trait interface for a custom rpc namespace `myrpcExt`
///
/// This defines an additional namespace where all methods are configured as trait functions.

#[rpc(server, namespace = "myrpcExt")]
pub trait MyRpcExtApi {
    #[method(name = "customMethod")]
    fn custom_method(&self) -> EthResult<Option<Block>>;
}

pub struct MyRpcExt<Provider> {
    provider: Provider,
}

impl<Provider> MyRpcExtApiServer for MyRpcExt<Provider>
where
    Provider: BlockReaderIdExt<Block = Block> + 'static,
{
    fn custom_method(&self) -> EthResult<Option<Block>> {
        // Example implementation that fetches the latest block
        let block = self.provider.block_by_number(0)?;
        Ok(block)
    }
}
