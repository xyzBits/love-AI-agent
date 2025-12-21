use openraft::RaftTypeConfig;
use serde::{Deserialize, Serialize};

// === 咒语开始 ===

// 1. 定义容器
// 旁白：“编译器听令！我要在当前位置开辟一个对外公开（pub）的房间（mod）...”
// 旁白：“...这个房间的名字叫 pb（Protobuf 的缩写），专门用来存放那些生成的代码。”
// openraft_memory::model::pb::Student
pub mod pb {

    // 2. 召唤 Raft 服务代码
    // 旁白：“Tonic 助手（tonic::），请发动你的‘传送门魔法’（include_proto!）...”
    // 旁白：“...去那个我看不到的构建目录（OUT_DIR）里，找到对应 'raft_service' 包的 Rust 代码...”
    // 旁白：“...然后把它们全部瞬移、粘贴到这里！”
    // (此时，VoteRequest, VoteResponse, RaftService 等结构体凭空出现在了这里)
    tonic::include_proto!("raft_service");

    // 3. 召唤学生服务代码
    // 旁白：“Tonic 助手，再发动一次传送门魔法...”
    // 旁白：“...这次去把 'student_service' 包生成的代码也抓取过来...”
    // 旁白：“...同样粘贴到这里，不要和上面的冲突。”
    // (此时，Student, AddStudentRequest, StudentService 等结构体也凭空出现了)
    tonic::include_proto!("student_service");

    // 4. 封闭容器
    // 旁白：“好了，房间打包完毕，其他人可以通过 use crate::pb::* 来使用这些‘变’出来的代码了。”
}

pub use pb::Student;

/// Raft 节点的 ID 类型
/// 在本项目中，凡是用到“节点 ID”的地方，本质上都是在用一个 u64 整数，但我们统一称呼它为 NodeId。
/// 假设项目开发到一半，你发现 u64 太大了，想改成 u32；或者你需要支持字符串类型的 ID（比如 UUID）。
///
/// 如果没有别名：你需要全项目搜索 u64，然后小心翼翼地把代表 ID 的 u64 改成 String，同时避开代表时间戳或金额的 u64。这非常容易出错。
///
/// 有了别名：你只需要修改这一行代码：
/// type 定义类型别名

pub type NodeId = u64;

/// 状态机中的写操作（提案）
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Request {
    Create(Student),
    Update(Student),
    Delete(i64),
}

/// 状态机操作的响应
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Response {
    pub success: bool,
    pub message: String,
    pub data: Option<Student>,
}

/// OpenRaft 的类型配置
/// Hash 作为 map 的 key 进行比较
/// 这是一个空结构体（Unit Struct）。
/// 它本身不存储任何数据，唯一的存在的意义就是作为一个**“标签”**（Tag），用来实现 Traits。
#[derive(
    Debug, Clone, Copy, Default, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd, Hash,
)]
pub struct TypeConfig {}

// 实现 Display 特征（意味着这个类型可以用 "{}" 打印）
impl std::fmt::Display for TypeConfig {
    // 定义打印函数
    // &self:  我要看着我自己 (TypeConfig)
    // f:      我要往这个可变的工具箱/画布里写东西
    // Result: 无论成功失败，我都要汇报
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 动作：把 "TypeConfig" 这个字符串写入到 f 里面去
        // 并把写入的结果（成功或失败）返回出去
        write!(f, "TypeConfig")
    }
}

// 我要按照 RaftTypeConfig 的图纸，定制我的 TypeConfig 汽车：
impl RaftTypeConfig for TypeConfig {
    // 1. 车拉的货物（D）是 Request 类型。
    type D = Request;

    // 2. 送货后的回执单（R）是 Response 类型。
    type R = Response;

    // 3. 司机的工号（NodeId）必须是 u64 整数。
    type NodeId = u64;

    // 4. 司机名片（Node）不用印详细信息，用空白的 EmptyNode 就行。
    type Node = openraft::impls::EmptyNode;

    // 5. 货箱（Entry）使用官方原厂的 Entry 箱子，但尺寸要适配我的配置。
    type Entry = openraft::Entry<TypeConfig>;

    // 6. 车的黑匣子备份（SnapshotData）是一个内存里的字节流 Cursor。
    type SnapshotData = std::io::Cursor<Vec<u8>>;

    // 7. 发动机（AsyncRuntime）使用 Tokio 引擎。
    type AsyncRuntime = openraft::impls::TokioRuntime;

    // 8. 通讯员（Responder）使用一次性对讲机。
    type Responder = openraft::impls::OneshotResponder<TypeConfig>;
}
