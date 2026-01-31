/// # Type State Pattern - 编译期状态机
///
/// 目标：设计一个 HTTP 请求构建器，让编译器帮你防止非法操作
///
/// ## 规则（必须在编译期强制执行）：
/// 1. 必须先设置 URL
/// 2. 设置 URL 后才能设置 Headers  
/// 3. 设置 Headers 后才能 send()
/// 4. send() 后请求被消费，不能再使用
///
/// ## 你的任务：
/// 1. 理解下面的代码结构
/// 2. 完成 TODO 部分
/// 3. 确保测试通过
/// 4. 尝试写出"非法代码"，验证编译器会报错
use std::collections::HashMap;
use std::marker::PhantomData;

// ==========================================
// 第一步：定义状态标记（零大小类型 - ZST）
// ==========================================

/// 初始状态：什么都没设置
struct NoUrl;

/// 已设置 URL
struct HasUrl;

/// 已设置 Headers，准备发送
struct Ready;

// ==========================================
// 第二步：定义请求构建器（带状态泛型）
// ==========================================

/// HTTP 请求构建器
///
/// `State` 是一个类型参数，用于在编译期追踪当前状态
/// `PhantomData<State>` 告诉编译器我们"使用"了这个类型，但不占用运行时内存
struct RequestBuilder<State> {
    url: Option<String>,
    headers: HashMap<String, String>,
    body: Option<String>,
    _state: PhantomData<State>,
}

// ==========================================
// 第三步：为不同状态实现不同的方法
// ==========================================

/// 只有 NoUrl 状态才能调用 new()
impl RequestBuilder<NoUrl> {
    fn new() -> Self {
        RequestBuilder {
            url: None,
            headers: HashMap::new(),
            body: None,
            _state: PhantomData,
        }
    }

    /// 设置 URL，状态从 NoUrl -> HasUrl
    ///
    /// TODO 1: 完成这个方法
    /// 提示：返回类型应该是 RequestBuilder<HasUrl>
    fn url(self, url: &str) -> RequestBuilder<HasUrl> {
        // todo!("实现 url 方法：创建新的 RequestBuilder<HasUrl>，把数据搬过去")
        RequestBuilder {
            url: Some(url.to_string()),
            headers: self.headers,
            body: self.body,
            _state: PhantomData,
        }
    }
}

/// 只有 HasUrl 状态才能设置 headers
impl RequestBuilder<HasUrl> {
    /// 添加一个 header
    ///
    /// TODO 2: 完成这个方法
    /// 注意：添加 header 后状态不变，还是 HasUrl，所以返回 Self
    fn header(mut self, key: &str, value: &str) -> Self {
        // todo!("实现 header 方法：往 self.headers 里插入 key-value")
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    /// 设置 body（可选）- 这个已经帮你实现了
    fn body(mut self, body: &str) -> Self {
        self.body = Some(body.to_string());
        self
    }

    /// 标记为准备就绪，状态从 HasUrl -> Ready
    ///
    /// TODO 3: 完成这个方法
    fn ready(self) -> RequestBuilder<Ready> {
        // todo!("实现 ready 方法：创建新的 RequestBuilder<Ready>")
        RequestBuilder {
            url: self.url,
            headers: self.headers,
            body: self.body,
            _state: PhantomData,
        }
    }
}

/// 只有 Ready 状态才能发送请求
impl RequestBuilder<Ready> {
    /// 发送请求（消费 self）
    ///
    /// TODO 4: 完成这个方法
    /// 提示：模拟发送，打印请求信息，返回一个假的 Response
    fn send(self) -> Response {
        // todo!("实现 send 方法：打印请求信息，返回 Response")
        Response {
            status: 200,
            body: "OK".to_string(),
        }
    }
}

// ==========================================
// 第四步：响应结构体
// ==========================================

#[derive(Debug)]
struct Response {
    status: u16,
    body: String,
}

// ==========================================
// 测试用例
// ==========================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 正确的使用流程
    #[test]
    fn test_correct_flow() {
        let response = RequestBuilder::new()
            .url("https://api.example.com/users")
            .header("Content-Type", "application/json")
            .header("Authorization", "Bearer token123")
            .body(r#"{"name": "Rust"}"#)
            .ready()
            .send();

        println!("Response: {:?}", response);
        assert_eq!(response.status, 200);
    }

    /// TODO 5: 完成实现后，取消下面的注释，验证编译器会报错
    /// 这些代码应该无法编译！

    #[test]
    fn test_cannot_send_without_url() {
        // 错误：没有设置 URL 就想 send
        let builder = RequestBuilder::new();
        let builder = builder.url("http://google.com");
        let builder = builder.body("hello google");
        builder.ready().send();
        // let response = RequestBuilder::new().send(); // 编译失败！
    }

    #[test]
    fn test_cannot_send_without_ready() {
        // 错误：没有调用 ready() 就想 send
        let response = RequestBuilder::new()
            .url("https://example.com")
            .ready()
            .send(); // 编译失败！
    }

    #[test]
    fn test_cannot_reuse_after_send() {
        // 错误：send 后不能再使用
        let builder = RequestBuilder::new().url("https://example.com").ready();

        let _ = builder.send();
        // let _ = builder.send(); // 编译失败！所有权已转移
    }

    #[test]
    fn test_phantom_data_size() {
        use std::mem::size_of;

        // PhantomData 本身是 0 字节
        println!(
            "PhantomData<NoUrl> 大小: {}",
            size_of::<PhantomData<NoUrl>>()
        );

        // 整个 RequestBuilder 的大小不会因为 State 不同而变化
        println!(
            "RequestBuilder<NoUrl> 大小: {}",
            size_of::<RequestBuilder<NoUrl>>()
        );
        println!(
            "RequestBuilder<HasUrl> 大小: {}",
            size_of::<RequestBuilder<HasUrl>>()
        );
        println!(
            "RequestBuilder<Ready> 大小: {}",
            size_of::<RequestBuilder<Ready>>()
        );
    }
}

mod example {
    use std::marker::PhantomData;

    // ==========================================
    // 状态标记
    // ==========================================
    struct Disconnected;
    struct Connected;
    struct InTransaction;
    struct Committed;

    // ==========================================
    // 数据库连接（带状态）
    // ==========================================
    struct DbConnection<State> {
        url: String,
        _state: PhantomData<State>,
    }

    // ==========================================
    // Disconnected: 只能 connect
    // ==========================================
    impl DbConnection<Disconnected> {
        fn new() -> Self {
            DbConnection {
                url: String::new(),
                _state: PhantomData,
            }
        }

        /// 连接数据库：Disconnected -> Connected
        fn connect(self, url: &str) -> DbConnection<Connected> {
            println!("🔌 连接到数据库: {}", url);
            DbConnection {
                url: url.to_string(),
                _state: PhantomData,
            }
        }
    }

    // ==========================================
    // Connected: 可以开启事务或断开
    // ==========================================
    impl DbConnection<Connected> {
        /// 开启事务：Connected -> InTransaction
        fn begin_transaction(self) -> DbConnection<InTransaction> {
            println!("📝 开启事务");
            DbConnection {
                url: self.url,
                _state: PhantomData,
            }
        }

        /// 断开连接：Connected -> Disconnected
        fn disconnect(self) -> DbConnection<Disconnected> {
            println!("🔌 断开连接");
            DbConnection {
                url: String::new(),
                _state: PhantomData,
            }
        }
    }

    // ==========================================
    // InTransaction: 可以执行 SQL、提交或回滚
    // ==========================================
    impl DbConnection<InTransaction> {
        /// 执行 SQL（状态不变）
        fn execute(self, sql: &str) -> Self {
            println!("⚡ 执行 SQL: {}", sql);
            self
        }

        /// 提交事务：InTransaction -> Committed
        fn commit(self) -> DbConnection<Committed> {
            println!("✅ 提交事务");
            DbConnection {
                url: self.url,
                _state: PhantomData,
            }
        }

        /// 回滚事务：InTransaction -> Connected
        fn rollback(self) -> DbConnection<Connected> {
            println!("⏪ 回滚事务");
            DbConnection {
                url: self.url,
                _state: PhantomData,
            }
        }
    }

    // ==========================================
    // Committed: 事务已提交，可以断开或开新事务
    // ==========================================
    impl DbConnection<Committed> {
        /// 断开连接
        fn disconnect(self) -> DbConnection<Disconnected> {
            println!("🔌 断开连接");
            DbConnection {
                url: String::new(),
                _state: PhantomData,
            }
        }

        /// 开启新事务
        fn begin_transaction(self) -> DbConnection<InTransaction> {
            println!("📝 开启新事务");
            DbConnection {
                url: self.url,
                _state: PhantomData,
            }
        }
    }

    // ==========================================
    // 测试
    // ==========================================
    #[test]
    fn test_correct_flow() {
        let conn = DbConnection::new()
            .connect("postgres://localhost:5432/mydb")
            .begin_transaction()
            .execute("INSERT INTO users VALUES (1, 'Rust')")
            .execute("UPDATE users SET name = 'Rustacean' WHERE id = 1")
            .commit()
            .disconnect();

        println!("🎉 完成！");
    }

    #[test]
    fn test_rollback_flow() {
        let conn = DbConnection::new()
            .connect("postgres://localhost:5432/mydb")
            .begin_transaction()
            .execute("DELETE FROM users") // 危险操作！
            .rollback() // 后悔了，回滚
            .disconnect();

        println!("🎉 已回滚！");
    }

    // ❌ 这些代码无法编译（取消注释试试）

    #[test]
    fn test_cannot_execute_without_transaction() {
        DbConnection::new()
            .connect("postgres://localhost")
            .begin_transaction()
            .execute("SELECT 1"); // 编译失败！Connected 没有 execute 方法
    }

    #[test]
    fn test_cannot_commit_without_transaction() {
        DbConnection::new()
            .connect("postgres://localhost")
            .begin_transaction()
            .commit(); // 编译失败！Connected 没有 commit 方法
    }
}
