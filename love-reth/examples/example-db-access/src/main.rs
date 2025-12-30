// 如果我在 Cargo.toml 里声明了一个依赖包，但是没有使用它，
// 那么编译器会给我一个警告，提醒我这个依赖包是未使用的。
// 这有助于保持代码的整洁，避免不必要的依赖包。
// 多余的依赖包会增加编译时间和二进制文件的大小，
// 所以及时清理未使用的依赖包是一个好习惯。

#![warn(unused_crate_dependencies)]

use alloy_primitives::{Address, B256, keccak256};
use eyre::Ok;
use reth_ethereum::chainspec::{ChainSpecProvider, EthChainSpec};
use reth_ethereum::primitives::{AlloyBlockHeader, RecoveredBlock, SealedBlock};
use reth_ethereum::provider::{
    BlockNumReader, BlockReader, TransactionVariant, TransactionsProvider,
};
use reth_ethereum::rpc::eth::primitives::Filter;
use reth_ethereum::storage::{AccountReader, BlockSource, ReceiptProvider, StateProvider};
use reth_ethereum::{
    Block, Receipt, TransactionSigned, chainspec::ChainSpecBuilder, node::EthereumNode,
    primitives::SealedHeader, provider::providers::ReadOnlyConfig, storage::HeaderProvider,
};

// 引入 alloy-primitives 包，但不直接使用它

// Providers are zero cost abstractions on top of an opened MDBX Transaction
// exposing a familiar API to query the chain's information without requiring knowledge
// of the inner tables.
//
// These abstractions do not include any caching and the user is responsible for doing that.
// Other parts of the code which include caching are parts of the `EthApi` abstraction.
fn main() -> eyre::Result<()> {
    // The path to data directory, e.g. "~/.local/share/reth/mainnet"
    let datadir = std::env::var("RETH_DATADIR")?;

    // Instantiate a provider factory for Ethereum mainnet using the provided datadir path.
    let spec = ChainSpecBuilder::mainnet().build();

    println!("current chain id={}", &spec.chain.id());

    let factory = EthereumNode::provider_factory_builder()
        .open_read_only(spec.into(), ReadOnlyConfig::from_datadir(datadir))?;

    println!("mdbx chain id={}", factory.chain_spec().chain_id());
    // The call opens a RO transaction on the database. To write to the DB you'd need to call
    // the `provider_rw` function and look for the `Writer` variants of the traits.
    let provider = factory.provider()?;

    // Run basic queries against the DB
    let block_num = 100;

    header_provider_example(&provider, block_num)?;
    block_provider_example(&provider, block_num)?;
    txs_provider_example(&provider)?;
    receipts_provider_example(&provider)?;

    state_provider_example(factory.latest()?, &provider, provider.best_block_number()?)?;
    state_provider_example(
        factory.history_by_block_number(block_num)?,
        &provider,
        block_num,
    )?;

    // Closes the RO transaction opened in the `factory.provider()` call. This is optional and
    // would happen anyway at the end of the function scope.
    drop(provider);

    // 如果这里报错，eyre 会处理
    Ok(())
}

/// The `TransactionsProvider` allows querying transaction-related information
fn txs_provider_example<T: TransactionsProvider<Transaction = TransactionSigned>>(
    provider: T,
) -> eyre::Result<()> {
    // Try the 5th tx
    let txid = 5;

    // query a transaction by its primary ordered key in the db
    // 在创世块以来的第 5 笔交易
    let tx = provider
        .transaction_by_id(txid)?
        .ok_or(eyre::eyre!("transaction not found"))?;

    println!("tx.hash={}", tx.hash());
    // Can query by the tx hash
    let tx_by_hash = provider
        .transaction_by_hash(*tx.tx_hash())?
        .ok_or(eyre::eyre!("txhash not found"))?;
    assert_eq!(tx, tx_by_hash);

    // Can query the tx by hash with info about the block it was included in
    let (tx, meta) = provider
        .transaction_by_hash_with_meta(*tx.tx_hash())?
        .ok_or(eyre::eyre!("tx hash not found"))?;
    assert_eq!(*tx.hash(), meta.tx_hash);

    // Can query the txs in the range [100, 200)
    let _txs_by_tx_range: Vec<TransactionSigned> = provider.transactions_by_tx_range(100..200)?;

    // Can query the txs in the _block_ range [100, 200）
    let _txs_by_block_range: Vec<Vec<TransactionSigned>> =
        provider.transactions_by_block_range(100..200)?;

    Ok(())
}

/// The `HeaderProvider` allows querying the header-related tables.
fn header_provider_example<T: HeaderProvider>(provider: T, number: u64) -> eyre::Result<()> {
    // Can query the header by number
    let header = provider
        .header_by_number(number)?
        .ok_or(eyre::eyre!("header not found"))?;

    // We can convert a header to a sealed header which contains the hash w/o needing to recompute
    // it every time.
    // SealedHeader 包含 hash，不用每次都去计算，提高性能
    let sealed_header = SealedHeader::seal_slow(header);

    // Can also query the header by hash
    let header_by_hash = provider
        .header(sealed_header.hash())?
        .ok_or(eyre::eyre!("header by hash not found"))?;
    assert_eq!(sealed_header.header(), &header_by_hash);

    // Can query headers by range as well, already sealed
    let headers = provider.sealed_headers_range(100..200)?;
    assert_eq!(headers.len(), 100);

    Ok(())
}

/// The `BlockReader` allows querying the headers-related tables.
#[allow(dead_code)]
fn block_provider_example<T: BlockReader<Block = reth_ethereum::Block>>(
    provider: T,
    number: u64,
) -> eyre::Result<()> {
    // Can query a block by number
    let block: Block = provider
        .block(number.into())?
        .ok_or(eyre::eyre!("block num not found"))?;
    assert_eq!(block.number, number);

    // Can query a block with its senders, this is useful when you want to execute a block and do
    // not want o manually recover the senders for each transaction (as each transaction is
    // stored on disk with its v,r,s but not its `from` field.).
    let _recovered_block: RecoveredBlock<Block> = provider
        .sealed_block_with_senders(number.into(), TransactionVariant::WithHash)?
        .ok_or(eyre::eyre!("block number not found"))?;

    // Can seal the block to cash the hash, like the Header above
    let sealed_block = SealedBlock::seal_slow(block.clone());

    // Can also query the block by hash directly
    let block_by_hash = provider
        .block_by_hash(sealed_block.hash())?
        .ok_or(eyre::eyre!("block by hash not found"))?;
    assert_eq!(block, block_by_hash);

    // Or by relying in the internal conversion
    let block_by_hash2 = provider
        .block(sealed_block.hash().into())?
        .ok_or(eyre::eyre!("block by hash not found"))?;
    assert_eq!(block, block_by_hash2);

    // Or you can also specify the datasource. For this provider this always returns `None`, but
    // the blockchain tree is also able to access pending state not available in the db yet.
    let block_by_hash3 = provider
        .find_block_by_hash(sealed_block.hash(), BlockSource::Any)?
        .ok_or(eyre::eyre!("block hash not found"))?;

    assert_eq!(block, block_by_hash3);

    Ok(())
}

/// The `ReceiptProvider` allows querying the receipts tables
/// 回执 receipt 是交易执行后的收据，包含了交易是否成功 status，消耗了多少 gas，以及交易执行过程中产生的所有日志
fn receipts_provider_example<
    T: ReceiptProvider<Receipt = reth_ethereum::Receipt>
        + TransactionsProvider<Transaction = TransactionSigned>
        + HeaderProvider,
>(
    provider: T,
) -> eyre::Result<()> {
    let txid = 5;
    let header_num = 100;

    // Query a receipt by txid
    // 全局 id 查询
    let receipt = provider
        .receipt(txid)?
        .ok_or(eyre::eyre!("tx receipt not found"))?;

    // Can query receipt by tx hash too
    let tx = provider.transaction_by_id(txid)?.unwrap();
    let receipt_by_hash = provider
        .receipt_by_hash(*tx.tx_hash())?
        .ok_or(eyre::eyre!("tx receipt by hash not found"))?;

    assert_eq!(receipt, receipt_by_hash);

    // Can query all the receipts in a block
    let _receipts: Vec<Receipt> = provider
        .receipts_by_block(100.into())?
        .ok_or(eyre::eyre!("no receipts found for block"))?;

    // Can check if an address/topic filter is present in a header, if it is we query the block and
    // receipts and do something with the data
    // 1. get the bloom for the header
    // 每个 区块头 里面都有一个 256 字节的bloom filter，由该区块内所有交易日志的地址和topic 混合计算出来的
    let header = provider.header_by_number(header_num)?.unwrap();
    let bloom = header.logs_bloom();

    // 2. Construct the address/topics filters. topic0 always refers to the event signature, so
    // filter it with event_signature() (or use the .event() helper). The remaining helpers map to
    // the indexed parameters in declaration order (topic1 -> first indexed param, etc).
    let contract_addr = Address::random();
    let indexed_from = Address::random();
    let indexed_to = Address::random();
    let transfer_signature = keccak256("Transfer(address,address,uint256)");

    // This matches ERC-20 Transfer events emitted by contract addr where both indexed addresses are
    // fixed. If your event declares a third indexed parameter, continue with topic3(...).
    let filter = Filter::new()
        .address(contract_addr)
        .event_signature(transfer_signature)
        .topic1(indexed_from)
        .topic2(indexed_to);

    // 3. If the address & topics filters match do something. We use the outer check against the
    // bloom filter stored in the header to avoid having to query the receipts table when where
    // is no instance of any event that matches the filter in the header.
    if filter.matches_bloom(bloom) {
        let receipts = provider
            .receipt(header_num)?
            .ok_or(eyre::eyre!("receipt not found"))?;
        for log in &receipts.logs {
            if filter.matches(log) {
                // Do something with the log e.g. decode it.
                println!("Matching log found! {log:?}");
            }
        }
    }

    Ok(())
}

/// The `StateProvider` allows querying the state tables
fn state_provider_example<T: StateProvider + AccountReader, H: HeaderProvider>(
    provider: T,
    headers: &H,
    number: u64,
) -> eyre::Result<()> {
    let address = Address::random();
    let storage_key = B256::random();
    let slots = [storage_key];

    let header = headers
        .header_by_number(number)?
        .ok_or(eyre::eyre!("header not found"))?;
    let state_root = header.state_root();

    // Can get account / storage state with simple point queries
    let account = provider.basic_account(&address)?;
    let code = provider.account_code(&address)?;
    let storage_value = provider.storage(address, storage_key)?;

    println!(
        "state at block #{number}: addr={address:?}, nonce={}, balance={}, storage[{:?}]={:?}, has_code={}",
        account.as_ref().map(|acc| acc.nonce).unwrap_or_default(),
        account.as_ref().map(|acc| acc.nonce).unwrap_or_default(),
        storage_key,
        storage_value,
        code.is_some()
    );

    // Returns b bundled proof with the account's info
    let proof = provider.proof(Default::default(), address, &slots)?;

    // Can verify the returned proof against the state root
    proof.verify(state_root)?;
    println!("account proof verified against state root {state_root:?}");

    Ok(())
}
