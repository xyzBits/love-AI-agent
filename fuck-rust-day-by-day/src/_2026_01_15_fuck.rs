use async_trait::async_trait; // 👈 引入宏
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};

// ==========================================
// 1. 定义核心类型
// ==========================================

type BlockNumber = u64;

#[derive(Debug, PartialEq)]
enum StageResult {
    /// 同步完成
    Done { height: BlockNumber },
    /// 取得进展
    Progress { height: BlockNumber },
    /// 🚨 请求回滚
    Unwind { unwind_to: BlockNumber },
}

// ==========================================
// 2. 模拟数据库
// ==========================================
#[derive(Clone, Debug)]
struct Database {
    // Key: Stage ID, Value: BlockNumber
    progress: Arc<Mutex<HashMap<String, BlockNumber>>>,
}

impl Database {
    fn new() -> Self {
        Self {
            progress: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn get_progress(&self, stage_id: &str) -> BlockNumber {
        *self.progress.lock().unwrap().get(stage_id).unwrap_or(&0)
    }

    fn save_progress(&self, stage_id: &str, height: BlockNumber) {
        println!("💾 [DB] 保存进度: {} -> Block #{}", stage_id, height);
        self.progress
            .lock()
            .unwrap()
            .insert(stage_id.to_string(), height);
    }
}

// ==========================================
// 3. Stage Trait (核心修改点)
// ==========================================

// 1. 使用 #[async_trait] 宏
// 2. 加上 Send + Sync 是为了让 Box<dyn Stage> 在多线程环境（Tokio）更安全
#[async_trait]
trait Stage: Send + Sync {
    fn id(&self) -> &'static str;

    // 这里原本直接写 async fn 导致不兼容 dyn，现在有了宏就可以写了
    async fn execute(&mut self, db: &Database, target: BlockNumber) -> StageResult;

    async fn unwind(&mut self, db: &Database, to: BlockNumber);
}

// ==========================================
// 4. 具体实现：HeaderStage
// ==========================================

struct HeaderStage;

#[async_trait] // 👈 实现处也必须加这个宏
impl Stage for HeaderStage {
    fn id(&self) -> &'static str {
        "Headers"
    }

    async fn execute(&mut self, db: &Database, target: BlockNumber) -> StageResult {
        let current = db.get_progress(self.id());

        // 如果已经追上目标，完成
        if current >= target {
            return StageResult::Done { height: current };
        }

        // 模拟下载过程，每次同步 10 个块
        let new_height = std::cmp::min(current + 10, target);
        sleep(Duration::from_millis(100)).await;

        println!("⬇️  [Headers] 下载中... {} -> {}", current, new_height);

        // --- 模拟故障注入 ---
        // 场景：当我们下载到 #40，且目标是 #50 时，假装发现了分叉
        if new_height == 40 && target == 50 {
            println!("⚠️  [Headers] 警告：在 Block #40 发现分叉链！请求回滚至 #30");
            // 返回回滚指令
            return StageResult::Unwind { unwind_to: 30 };
        }

        // 正常情况
        StageResult::Progress { height: new_height }
    }

    async fn unwind(&mut self, db: &Database, to: BlockNumber) {
        println!("🏳️  [Headers] 正在执行回滚操作 -> 目标 Block #{}", to);
        // 真实场景会在这里 truncate 数据库表
        db.save_progress(self.id(), to);
    }
}

// ==========================================
// 5. 流水线 Pipeline
// ==========================================

struct Pipeline {
    // 这里的 Box<dyn Stage> 现在是合法的了
    stages: Vec<Box<dyn Stage>>,
    db: Database,
}

impl Pipeline {
    fn new(db: Database) -> Self {
        Self {
            stages: vec![],
            db,
        }
    }

    fn add_stage<S: Stage + 'static>(&mut self, stage: S) {
        self.stages.push(Box::new(stage));
    }

    /// 核心调度引擎
    async fn run(&mut self, target: BlockNumber) {
        println!("🚀 Pipeline 启动，最终目标: #{}", target);

        // 外层循环：当发生回滚时，通过这里重启流水线
        loop {
            let mut all_done = true; // 假设所有阶段都做完了

            // 内层循环：按顺序执行每个 Stage
            for i in 0..self.stages.len() {
                
                // 【技巧点】：限制可变借用的范围
                // 我们在一个单独的代码块里执行 execute，执行完后 `stage` 借用就结束了
                // 这样我们在下面的 Unwind 分支里就可以再次借用 self.stages
                let result = {
                    let stage = &mut self.stages[i];
                    stage.execute(&self.db, target).await
                }; 

                match result {
                    StageResult::Done { .. } => {
                        // 当前阶段没事干了，检查下一个
                        continue;
                    }
                    StageResult::Progress { height } => {
                        // 取得了进展，保存进度
                        // 注意：这里我们为了简化，再次获取了 id (避免上面的借用冲突)
                        let stage_id = self.stages[i].id();
                        self.db.save_progress(stage_id, height);
                        
                        // 只要有一个阶段还在 Progress，就说明没完全结束
                        all_done = false;
                    }
                    StageResult::Unwind { unwind_to } => {
                        println!("🚨 Pipeline 收到中断指令：回滚至 #{}", unwind_to);
                        all_done = false;

                        // --- 回滚逻辑 ---
                        // 从当前的阶段 i 开始，倒着回到 0，依次调用 unwind
                        // 比如：先回滚 Bodies，再回滚 Headers
                        for j in (0..=i).rev() {
                            let stage = &mut self.stages[j];
                            stage.unwind(&self.db, unwind_to).await;
                        }

                        println!("🔄 回滚完成，重启 Pipeline...\n");
                        
                        // 关键：跳出 for 循环，触发外层 loop 重新开始
                        // 因为回滚后状态变了，必须从头跑 Stage 0
                        break; 
                    }
                }
            }

            // 如果跑了一圈发现所有 Stage 都返回 Done，那就真的结束了
            if all_done {
                println!("✅ 恭喜！链同步完成，到达高度 #{}", target);
                break;
            }
        }
    }
}

// ==========================================
// 6. 主程序
// ==========================================

#[tokio::test]
async fn main() {
    let db = Database::new();
    let mut pipeline = Pipeline::new(db.clone());

    // 添加阶段
    pipeline.add_stage(HeaderStage);

    // 运行！目标高度 50 会触发我们的测试回滚逻辑
    pipeline.run(50).await;
}