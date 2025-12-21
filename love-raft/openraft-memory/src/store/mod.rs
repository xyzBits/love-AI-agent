use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fmt::Debug;
use std::io::Cursor;
use std::ops::RangeBounds;
use std::sync::Arc;

use openraft::Entry;
use openraft::EntryPayload;
use openraft::LogId;
use openraft::RaftLogReader;
use openraft::RaftSnapshotBuilder;
use openraft::Snapshot;
use openraft::SnapshotMeta;
use openraft::StorageError;
use openraft::StoredMembership;
use openraft::Vote;
use openraft::storage::LogState;
use openraft::storage::RaftStorage;
use tokio::sync::RwLock;

use crate::model::{Request, Response, Student, TypeConfig};

/// StateMachine (状态机)
/// 负责存储已提交(Committed)的日志数据，这里使用内存 HashMap 存储学生信息。
#[derive(Debug, Default)]
pub struct StateMachine {
    /// 最后一次应用到状态机的日志 ID 幂等性检查的关键。如果节点崩溃重启，它需要知道自己上次执行到哪条日志了，防止重复执行。
    pub last_applied_log_id: Option<LogId<u64>>,
    /// 核心数据存储：学生 ID -> 学生对象
    pub data: HashMap<i64, Student>, // 业务数据
    /// 记录最近一次的集群成员配置
    pub last_membership: StoredMembership<u64, openraft::impls::EmptyNode>,
}

/// LogStore (日志存储)
/// 负责持久化所有接收到的 Raft 日志条目，以及节点的投票信息。
#[derive(Debug, Default)]
pub struct LogStore {
    /// 日志条目存储：索引 -> 日志对象
    /// 使用 BTreeMap 而不是 HashMap。原因：Raft 经常需要进行范围查询（例如：获取索引 100 到 200 的日志同步给 Follower），
    /// BTreeMap 的 Key 是有序的，支持高效的范围扫描。
    pub logs: BTreeMap<u64, Entry<TypeConfig>>,
    /// 最近一次的投票信息
    pub vote: Option<Vote<u64>>,
}

/// Store (存储中心)
/// 将状态机和日志存储封装在一起，协调两者的读写。
#[derive(Clone, Default)]
pub struct Store {
    pub state_machine: Arc<RwLock<StateMachine>>,
    pub log_store: Arc<RwLock<LogStore>>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            state_machine: Arc::new(RwLock::new(StateMachine::default())),
            log_store: Arc::new(RwLock::new(LogStore::default())),
        }
    }
}

impl RaftLogReader<TypeConfig> for Store {
    /// 根据范围获取一批日志条目
    /// 场景：当你是 Leader，有一个 Follower 落后了，你需要把旧日志发给它。
    ///
    /// 逻辑：利用 BTreeMap 的范围查找功能 (range)，快速拉取一批日志返回。
    async fn try_get_log_entries<RB>(
        &mut self,
        range: RB,
    ) -> Result<Vec<Entry<TypeConfig>>, StorageError<u64>>
    where
        RB: RangeBounds<u64> + Clone + Debug + Send,
    {
        let l = self.log_store.read().await;
        Ok(l.logs.range(range).map(|(_, val)| val.clone()).collect())
    }
}

/// 实现 RaftSnapshotBuilder 接口
/// 负责创建快照，以防止日志无限增长。当前内存示例暂未实现。
impl RaftSnapshotBuilder<TypeConfig> for Store {
    async fn build_snapshot(&mut self) -> Result<Snapshot<TypeConfig>, StorageError<u64>> {
        Err(StorageError::from_io_error(
            openraft::ErrorSubject::Snapshot(None),
            openraft::ErrorVerb::Read,
            std::io::Error::new(std::io::ErrorKind::Other, "快照功能暂未实现"),
        ))
    }
}

/// 实现 RaftStorage (v1 兼容模式)
/// 0.9.x 推荐通过 Adaptor 桥接这种单接口实现，更加简洁。
impl RaftStorage<TypeConfig> for Store {
    type LogReader = Self;

    /// 获取日志读取器实例
    async fn get_log_reader(&mut self) -> Self::LogReader {
        self.clone()
    }

    /// 获取日志当前状态
    async fn get_log_state(&mut self) -> Result<LogState<TypeConfig>, StorageError<u64>> {
        let l = self.log_store.read().await;
        let last = l.logs.iter().next_back().map(|(_, ent)| ent.log_id);
        Ok(LogState {
            last_purged_log_id: None,
            last_log_id: last,
        })
    }

    /// 持久化节点的投票信息 (用于选举)
    async fn save_vote(&mut self, vote: &Vote<u64>) -> Result<(), StorageError<u64>> {
        let mut l = self.log_store.write().await;
        l.vote = Some(*vote);
        Ok(())
    }

    /// 读取持久化的投票信息
    async fn read_vote(&mut self) -> Result<Option<Vote<u64>>, StorageError<u64>> {
        let l = self.log_store.read().await;
        Ok(l.vote)
    }

    /// 向日志存储追加条目
    async fn append_to_log<I>(&mut self, entries: I) -> Result<(), StorageError<u64>>
    where
        I: IntoIterator<Item = Entry<TypeConfig>> + Send,
    {
        let mut l = self.log_store.write().await;
        for ent in entries {
            l.logs.insert(ent.log_id.index, ent);
        }
        Ok(())
    }

    /// 删除与 Leader 冲突的日志 (回滚逻辑)
    async fn delete_conflict_logs_since(
        &mut self,
        log_id: LogId<u64>,
    ) -> Result<(), StorageError<u64>> {
        let mut l = self.log_store.write().await;
        l.logs.split_off(&log_id.index);
        Ok(())
    }

    /// 清理旧日志 (通常在合并快照后执行)
    async fn purge_logs_upto(&mut self, _log_id: LogId<u64>) -> Result<(), StorageError<u64>> {
        Ok(())
    }

    /// 获取状态机最后一次应用的状态 (用于恢复或同步)
    async fn last_applied_state(
        &mut self,
    ) -> Result<
        (
            Option<LogId<u64>>,
            StoredMembership<u64, openraft::impls::EmptyNode>,
        ),
        StorageError<u64>,
    > {
        let sm = self.state_machine.read().await;
        Ok((sm.last_applied_log_id, sm.last_membership.clone()))
    }

    /// 函数签名：将已提交的日志条目应用到状态机
    // 旁白：“指挥官，这批日志（entries）已经得到了大多数节点的签字确认（Committed）。
    // 现在，请正式执行它们，修改我们的核心数据库！”
    async fn apply_to_state_machine(
        &mut self,
        entries: &[Entry<TypeConfig>], // 输入：一批有序的、已提交的日志
    ) -> Result<Vec<Response>, StorageError<u64>> {
        // 输出：执行结果列表

        // 1. 获取写锁
        // 旁白：“我要开始修改账本了。所有人暂停读写，把锁给我（write().await）。”
        // 这里的 state_machine_rw_lock 就是内存中的那个 BTreeMap，真正存数据的地方。
        let mut state_machine_rw_lock = self.state_machine.write().await; // 加写锁

        // 准备一个篮子，装每条命令执行后的返回值
        let mut res = Vec::new();

        // 2. 循环处理每一条日志
        // 旁白：“Raft 保证了这些日志的顺序绝对正确。我们要一条一条按顺序执行。”
        for entry in entries {
            // 3. 更新进度条 (关键点!)
            // 旁白：“每执行一条，我就要把书签往后移一格。”
            // “如果系统崩溃重启，我看一眼这个 ID，就知道我上次干到哪了，不会重复干。”
            state_machine_rw_lock.last_applied_log_id = Some(entry.log_id);

            // 4. 判断日志类型
            // 旁白：“打开这封信，看看里面是什么指令？”
            match entry.payload {
                // 情况 A: 空日志 (Blank)
                // 旁白：“这是一封空信。通常是新 Leader 上任时发的‘宣誓就职’贴。”
                // “它不包含业务数据，只为了确认 Leader 的地位。”
                EntryPayload::Blank => res.push(Response {
                    success: true,
                    message: "空日志应用成功".to_string(),
                    data: None,
                }),

                // 情况 B: 正常业务请求 (Normal) -> 这里的 req 就是你的 CRUD
                // 旁白：“这是一封真正的业务指令！快看具体要做什么。”
                EntryPayload::Normal(ref req) => {
                    match req {
                        // B1: 创建学生
                        Request::Create(student) => {
                            // 旁白：“指令是创建学生。把数据写入 HashMap。”
                            state_machine_rw_lock
                                .data
                                .insert(student.id, student.clone());
                            // 旁白：“写张回执单（Response），告诉客户端成功了。”
                            res.push(Response {
                                success: true,
                                message: "学生信息创建成功".to_string(),
                                data: Some(student.clone()),
                            });
                        }

                        // B2: 更新学生
                        Request::Update(std) => {
                            // 旁白：“指令是更新。先查查人在不在？”
                            if state_machine_rw_lock.data.contains_key(&std.id) {
                                state_machine_rw_lock.data.insert(std.id, std.clone()); // 覆盖写入
                                res.push(Response {
                                    success: true,
                                    message: "学生信息更新成功".to_string(),
                                    data: Some(std.clone()),
                                });
                            } else {
                                // 旁白：“查无此人，更新失败。”
                                res.push(Response {
                                    success: false,
                                    message: "未找到该学生".to_string(),
                                    data: None,
                                });
                            }
                        }

                        // B3: 删除学生
                        Request::Delete(id) => {
                            // 旁白：“指令是删除。从 HashMap 移除。”
                            let old = state_machine_rw_lock.data.remove(&id);
                            res.push(Response {
                                success: old.is_some(),
                                message: if old.is_some() {
                                    "已删除"
                                } else {
                                    "未找到"
                                }
                                .to_string(),
                                data: old,
                            });
                        }
                    }
                }

                // 情况 C: 成员变更 (Membership)
                // 旁白：“这是一封人事变动通知！有新节点加入或退出了。”
                EntryPayload::Membership(ref m) => {
                    // 旁白：“更新我脑子里‘谁是我们的伙伴’的名单。”
                    // 这非常重要，否则节点不知道该给谁发心跳。
                    state_machine_rw_lock.last_membership =
                        StoredMembership::new(Some(entry.log_id), m.clone());
                    res.push(Response {
                        success: true,
                        message: "集群配置已应用".to_string(),
                        data: None,
                    });
                }
            }
        }

        // 5. 完工
        // 旁白：“这一批所有指令都执行完了，锁释放，把一篮子回执单扔回去。”
        Ok(res)
    }

    /// 开始接收快照流
    async fn begin_receiving_snapshot(
        &mut self,
    ) -> Result<Box<Cursor<Vec<u8>>>, StorageError<u64>> {
        let data = <TypeConfig as openraft::RaftTypeConfig>::SnapshotData::default();
        Ok(Box::new(data))
    }

    /// 安装快照数据
    async fn install_snapshot(
        &mut self,
        _meta: &SnapshotMeta<u64, openraft::impls::EmptyNode>,
        _snapshot: Box<Cursor<Vec<u8>>>,
    ) -> Result<(), StorageError<u64>> {
        Ok(())
    }

    /// 获取当前的最新快照
    async fn get_current_snapshot(
        &mut self,
    ) -> Result<Option<Snapshot<TypeConfig>>, StorageError<u64>> {
        Ok(None)
    }

    type SnapshotBuilder = Self;
    /// 获取快照构建器
    async fn get_snapshot_builder(&mut self) -> Self::SnapshotBuilder {
        self.clone()
    }
}
