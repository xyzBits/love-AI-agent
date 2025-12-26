use std::cmp::Ordering;
use std::collections::{BTreeMap, BinaryHeap, HashMap};

// 模拟以太坊地址
type Address = u64;

// 模拟  Nonce
type Nonce = u64;

// 模拟 Gas Price (简化为 priority fee)
type GasPrice = u64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    pub sender: Address,
    pub nonce: Nonce,
    pub gas_price: GasPrice,
    pub hash: String, // 模拟  tx hash
}

//=========== 核心设计：候选人凭证 ===================
// 这个结构体专门放进 BinaryHeap 里，
// 它的全部意义就是：告诉我们谁有一笔多贵的交易
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Candidate {
    sender: Address,
    nonce: Nonce,
    gas_price: GasPrice,
}

// 必须实现 Ord 才能进 BinaryHeap
// 我们希望 GasPrice 最高的排前面，如果价格一样，Nonce 小的排前面
impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> Ordering {
        // 先比价格，价格高的在 Grater 堆内
        self.gas_price
            .cmp(&other.gas_price)
            // 如果价格一样，为了确定性，我们让 Sender ID 小的排在前面，或者其他规则
            .then_with(|| other.sender.cmp(&self.sender))
    }
}

// PartialOrd 是 Ord 的一部分，照抄即可
impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// 你的任务是填充这个结构体和实现逻辑
pub struct BlockBuilder {
    // todo 你需要设计内部数据结构来存储待处理的交易
    // 仓库：存放真实的交易数据
    // BTreeMap 保证了对同一个 Sender ，交易是按 Nonce 0 1 2 自动排好的
    pool: HashMap<Address, BTreeMap<Nonce, Transaction>>,

    // 榜单：只存每个 sender 的队头交易快照，每个用户只有一笔交易在榜单中
    frontier: BinaryHeap<Candidate>,
}

#[allow(dead_code)]
impl BlockBuilder {
    pub fn new() -> Self {
        Self {
            pool: HashMap::new(),
            frontier: BinaryHeap::new(),
        }
    }

    /// 向池子中添加一笔交易
    /// 假设所有交易都是合法的，且余额足够
    pub fn add_transaction(&mut self, tx: Transaction) {
        // 1. 先把交易存入仓库
        let sender_txs = self.pool.entry(tx.sender).or_default();
        sender_txs.insert(tx.nonce, tx.clone());

        // 2. 检查这笔交易是否有资格进入 榜单 frontier
        // 只有当这笔交易是该 Sender 当前最小的 Nonce 时，才需要更新榜单
        // 简化题目，
        // 如果是该 sender 的第一笔交易，肯定进 榜
        // 如果是更小的nonce 插队，这属于复杂情况，在pop 时处理 stale 也可以
        // 假设 add 是一次性完成的，只把新头部的放进去

        if let Some((&min_nonce, _)) = sender_txs.iter().next() {
            if min_nonce == tx.nonce {
                self.frontier.push(Candidate {
                    sender: tx.sender,
                    nonce: tx.nonce,
                    gas_price: tx.gas_price,
                });
            }
        }
    }

    /// 弹出当前最优的一笔交易
    /// 必须遵守
    /// 1. 同一个 sender 的 Nonce 必须严格递增，先出 0 才能出 1
    /// 2. 在满足 1 的前提下，优先出 GasPrice 最高的
    pub fn pop_best(&mut self) -> Option<Transaction> {
        // 循环直到找到一个有效的交易，或者堆空了
        while let Some(candidate) = self.frontier.pop() {
            // 1。拿到 候选人信息，
            // 2。去仓库核实一下，这个候选人是不是真的还没被处理
            // 并且它是不是该用户当前  nonce 最小的那个，防止过期数据

            if let Some(sender_txs) = self.pool.get_mut(&candidate.sender) {
                // 检查 队头是不是这个 nonce
                // BTreeMap first_key_value 获取最小 key
                if let Some((&head_nonce, _)) = sender_txs.iter().next() {
                    if head_nonce == candidate.nonce {
                        // 命中，这是合法的最优交易
                        // 1. 从仓库移除并取出交易
                        let tx = sender_txs.remove(&head_nonce).unwrap();

                        // 2. 关键一步，惰性填充
                        // 刚刚移除了 Nonce N，现在检查 Nonce N+1 是否存在
                        if let Some((&next_nonce, next_tx)) = sender_txs.iter().next() {
                            // 如果存在，就把 N+1 加入榜单参与竞争
                            self.frontier.push(Candidate {
                                sender: next_tx.sender,
                                nonce: next_nonce,
                                gas_price: next_tx.gas_price,
                            });
                        } else {
                            // 如果没交易了，清理  hashMap时里的空项
                            self.pool.remove(&candidate.sender);
                        }

                        return Some(tx);
                    }
                }
            }
        }

        None
    }
}

// ========================= 测试用例 不要修改 =====================
#[test]
fn test_work() {
    let mut builder = BlockBuilder::new();

    // 场景模拟：
    // 土豪 A: 有一个便宜的 Nonce 0，和一个巨贵的 Nonce 1
    // 穷人 B: 有一个中等价格的 Nonce 0
    //
    // 预期顺序：
    // 1. 必须先看 A:0 和 B:0。因为 B:0 (50) > A:0 (10)，所以先出 B:0。
    // 2. B 没交易了。剩下 A:0 (10)。出 A:0。
    // 3. A:0 出完后，A:1 (100) 解锁了。现在出 A:1。

    // 错误陷阱：如果你只按价格排，会先出 A:1，这是非法的（因为 A:0 还没出）。

    let txs = vec![
        Transaction {
            sender: 0xA,
            nonce: 0,
            gas_price: 10,
            hash: "A0".into(),
        }, // 便宜的门票
        Transaction {
            sender: 0xA,
            nonce: 1,
            gas_price: 100,
            hash: "A1".into(),
        }, // 巨贵的后续
        Transaction {
            sender: 0xB,
            nonce: 0,
            gas_price: 50,
            hash: "B0".into(),
        }, // 中等的首发
        Transaction {
            sender: 0xA,
            nonce: 2,
            gas_price: 20,
            hash: "A2".into(),
        },
    ];

    for tx in txs {
        builder.add_transaction(tx);
    }

    let mut result = Vec::new();
    while let Some(tx) = builder.pop_best() {
        result.push(tx.hash);
    }

    println!("Result: {:?}", result);

    // 验证逻辑
    let expected = vec!["B0", "A0", "A1", "A2"];
    assert_eq!(result, expected, "顺序错了！被虐了吧？");
    println!("恭喜！你成功模拟了 Reth 的交易排序逻辑！");
}
