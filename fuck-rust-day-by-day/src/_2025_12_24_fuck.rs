use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

// 1. 模拟一个交易
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Transaction {
    pub hash: String,
    pub nonce: u64,
}

// 2. 模拟一个验证器（比如去查询数据库状态）
// 在 Reth 中，这通常是一个 trait ，这里简化为一个结构体
pub struct Validator;

impl Validator {
    // 模拟耗时的验证过程 IO Bound
    pub async fn validate(&self, tx: &Transaction) -> bool {
        // 模拟去读磁盘、数据库 耗时 10 ms
        tokio::time::sleep(Duration::from_micros(10)).await;

        // 简单逻辑，nonce 必须偶数才合法
        tx.nonce & 2 == 0
    }
}

// 3. 交易池主体
pub struct TxPool {
    // 共享状态：交易哈希 --> 交易实体
    // java 思维：用锁保护共享资源
    // pool: Arc<std::sync::Mutex<HashMap<String, Transaction>>>,
    pool: Arc<tokio::sync::Mutex<HashMap<String, Transaction>>>,
    validator: Validator,
}

impl TxPool {
    pub fn new() -> Self {
        Self {
            // pool: Arc::new(std::sync::Mutex::new(HashMap::new())),
            pool: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            validator: Validator,
        }
    }

    // ---- 痛苦核心区块 ---------------
    // 目标：添加一笔交易，如果已存在则忽略，如果不存在，先验证，通过后再插入
    pub async fn add_transaction(&self, tx: Transaction) -> Result<(), String> {
        // 步骤 A: 上锁，准备操作
        // let mut pool_guard = self.pool.lock().unwrap();
        let mut pool_guard = self.pool.lock().await;

        // 先把 hash 克隆一份存在局部变量里
        let hash_log = tx.hash.clone();

        // 步骤 B: 查重
        if pool_guard.contains_key(&hash_log) {
            return Ok(());
        }

        // 步骤 C: 异步验证，这里是最大的坑
        // 我们不想把垃圾交易放进来，所以必须先 validate
        let is_valid = self.validator.validate(&tx).await;

        if !is_valid {
            return Err("Invalid transaction".into());
        }

        // 步骤 D: 验证通过，写入
        pool_guard.insert(hash_log.clone(), tx);
        println!("Inserted tx: {}", hash_log);

        Ok(())
    }
}

// tokio::sync::Mutex 使用信号量，而不是操作系统
#[tokio::test]
async fn test() {
    let pool = Arc::new(TxPool::new());

    // 模拟并发，同时发 10 个交易进来
    let mut handles = vec![];
    for i in 0..10 {
        let pool_clone = pool.clone();
        handles.push(tokio::spawn(async move {
            let tx = Transaction {
                hash: format!("0x{}", i),
                nonce: i,
            };

            // 如果注释掉下面的代码，就没有并发问题
            match pool_clone.add_transaction(tx).await {
                Ok(_) => println!("Task {} done", i),
                Err(e) => println!("Task {} failed: {}", i, e),
            }
        }))
    }

    for h in handles {
        h.await.unwrap();
    }
}
