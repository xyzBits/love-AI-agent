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
    /// 最后一次应用到状态机的日志 ID
    pub last_applied_log_id: Option<LogId<u64>>,
    /// 核心数据存储：学生 ID -> 学生对象
    pub data: HashMap<i64, Student>,
    /// 记录最近一次的集群成员配置
    pub last_membership: StoredMembership<u64, openraft::impls::EmptyNode>,
}

/// LogStore (日志存储)
/// 负责持久化所有接收到的 Raft 日志条目，以及节点的投票信息。
#[derive(Debug, Default)]
pub struct LogStore {
    /// 日志条目存储：索引 -> 日志对象
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

    /// 将已提交的日志条目应用到状态机 (最终数据写入点)
    async fn apply_to_state_machine(
        &mut self,
        entries: &[Entry<TypeConfig>],
    ) -> Result<Vec<Response>, StorageError<u64>> {
        let mut sm = self.state_machine.write().await;
        let mut res = Vec::new();

        for entry in entries {
            sm.last_applied_log_id = Some(entry.log_id);

            match entry.payload {
                EntryPayload::Blank => res.push(Response {
                    success: true,
                    message: "空日志应用成功".to_string(),
                    data: None,
                }),
                EntryPayload::Normal(ref req) => {
                    // 处理客户端真正的 CRUD 请求
                    match req {
                        Request::Create(std) => {
                            sm.data.insert(std.id, std.clone());
                            res.push(Response {
                                success: true,
                                message: "学生信息创建成功".to_string(),
                                data: Some(std.clone()),
                            });
                        }
                        Request::Update(std) => {
                            if sm.data.contains_key(&std.id) {
                                sm.data.insert(std.id, std.clone());
                                res.push(Response {
                                    success: true,
                                    message: "学生信息更新成功".to_string(),
                                    data: Some(std.clone()),
                                });
                            } else {
                                res.push(Response {
                                    success: false,
                                    message: "未找到该学生".to_string(),
                                    data: None,
                                });
                            }
                        }
                        Request::Delete(id) => {
                            let old = sm.data.remove(&id);
                            res.push(Response {
                                success: old.is_some(),
                                message: if old.is_some() {
                                    "学生信息已删除"
                                } else {
                                    "未找到该学生"
                                }
                                .to_string(),
                                data: old,
                            });
                        }
                    }
                }
                EntryPayload::Membership(ref m) => {
                    // 处理成员配置变更
                    sm.last_membership = StoredMembership::new(Some(entry.log_id), m.clone());
                    res.push(Response {
                        success: true,
                        message: "集群配置已应用".to_string(),
                        data: None,
                    });
                }
            }
        }
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
