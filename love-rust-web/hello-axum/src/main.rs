use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use axum::{
    Json, Router,
    extract::{
        Path, Query, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

#[tokio::main] // 启动 tokio 异步运行时
async fn main() {
    // 初始化共享状态
    let shared_state = Arc::new(AppState {
        db: Mutex::new(HashMap::new()),
    });

    // 构建应用路由
    // 当用户访问根路径 / 时，调用 root 函数
    // GET / 返回纯文本
    // POST /json 接收json返回json
    let app: Router = Router::new()
        .route("/", get(root))
        .route("/json", post(echo_json))
        .route("/users", post(create_user).get(search_users)) // 同一个路径，不同方法
        .route("/users/:id", get(get_user_by_id)) // :id 是路径参数占位符
        .route("/ws", get(ws_handler)) // 添加 WebSocket 路由
        .with_state(shared_state) // 注入状态！
        .fallback(handler_404); // 处理所有未匹配路由;

    // 定义监听地址
    let listiner = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("🚀 Server running on http://127.0.0.1:3000");

    // 启动服务
    axum::serve(listiner, app).await.unwrap();
}

/// 5. 处理函数 root
/// axum 非常智能，只要你的返回值实现了 IntoResponse tarit 它就能变成 http 响应
/// &'static str axum 会自动把它变成 text/plain 响应
async fn root() -> Html<&'static str> {
    Html("<h1> Hello, World! From Axum. </h1>")
}

// 这里用到了 serde
#[derive(Deserialize, Serialize, Clone, Debug)]
struct User {
    id: u64,
    username: String,
    age: u8,
}

// 魔法在这里：
// Axum 看到参数是 Json<User>，会自动检查 Content-Type，
// 自动读取 Body，自动用 serde_json 反序列化成 User 结构体。
// 如果格式不对，Axum 会自动返回 400 Bad Request，你都不用写错误处理代码。
// 参数解构语法
async fn echo_json(Json(payload): Json<User>) -> Json<User> {
    println!("收到用户: {}, 年龄: {}", payload.username, payload.age);

    // 直接返回 json 包裹的结构体，axum 会自动序列化回 json 字符串
    Json(payload)
}

// 这是前端创建用户时发来的 JSON
#[derive(Deserialize)] // Deserialize: 为了解析前端传来的 JSON
struct CreateUserPayload {
    username: String,
    age: u8,
}

// 这是查询参数 /users?id=1
#[derive(Deserialize)] // Deserialize: 为了解析 URL 里的查询参数
struct SearchParams {
    id: Option<u64>,
}

// --- 2. 定义共享状态 (模拟数据库) ---
// 真实项目中，这里通常放 sqlx::Pool 或 Redis 连接
struct AppState {
    // Key是ID, Value是User。
    // 使用 Mutex 是因为 Axum 是多线程并发的，修改数据必须加锁。
    db: Mutex<HashMap<u64, User>>,
}

// --- 3. Handlers (业务逻辑) ---

// 场景 A: 创建用户 (读取 State, 读取 JSON)
async fn create_user(
    // 1. 获取状态 (必须是 Clone 的，所以我们用 Arc)
    State(state): State<Arc<AppState>>,
    // 2. 解析 JSON Body
    Json(payload): Json<CreateUserPayload>,
) -> impl IntoResponse {
    let mut db = state.db.lock().unwrap(); //以此获取写锁

    let new_id = (db.len() as u64) + 1;
    let new_user = User {
        id: new_id,
        username: payload.username,
        age: payload.age,
    };

    db.insert(new_id, new_user.clone());

    // 返回 201 Created 和 创建的用户数据
    (StatusCode::CREATED, Json(new_user))
}

// 场景 B: 路径参数 (GET /users/1)
async fn get_user_by_id(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>, // 自动解析 URL 中的 :id
) -> impl IntoResponse {
    let db = state.db.lock().unwrap();

    match db.get(&id) {
        Some(user) => Json(user.clone()).into_response(),
        None => (StatusCode::NOT_FOUND, "User not found").into_response(),
    }
}

// 场景 C: 查询参数 (GET /users?id=1)
async fn search_users(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchParams>, // 自动解析 ?id=1
) -> Json<Vec<User>> {
    let db = state.db.lock().unwrap();

    if let Some(req_id) = params.id {
        // 如果 URL 里有 ?id=xx，只返回那个用户
        let users = db.get(&req_id).cloned().into_iter().collect();
        Json(users)
    } else {
        // 否则返回所有
        let users = db.values().cloned().collect();
        Json(users)
    }
}

// 场景 D: 404 处理
async fn handler_404() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "哎呀，你迷路了 (404)")
}

// --- 1. 定义通信协议 (JSON 格式) ---

// 客户端发送给服务器的消息
#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "lowercase")] // 这样 JSON 会长这样: {"type": "ping"}
enum ClientMsg {
    Ping,
    Subscribe { topic: String },
    Unsubscribe { topic: String },
}

// 服务器回复给客户端的消息
#[derive(Serialize, Debug)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ServerMsg {
    Pong,
    Subscribed { topic: String },
    Unsubscribed { topic: String },
    Error { msg: String },
}

// --- 2. WebSocket 握手处理 ---

// 这个 Handler 负责处理 HTTP 升级到 WebSocket 的握手请求
async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    // on_upgrade 接受一个闭包，这个闭包里写具体的 socket 处理逻辑
    ws.on_upgrade(handle_socket)
}

// --- 3. 具体的连接逻辑 ---
async fn handle_socket(mut socket: WebSocket) {
    println!("新连接已建立");

    // 【关键点】：这是属于“当前连接”的私有状态
    // 用 HashSet 存储该连接订阅的所有 topic，避免重复订阅
    let mut subscribed_topics: HashSet<String> = HashSet::new();
    // 循环接收消息
    while let Some(msg) = socket.recv().await {
        let msg = if let Ok(msg) = msg {
            msg
        } else {
            // 客户端断开连接
            println!("客户端断开连接");
            return;
        };

        if let Message::Text(text) = msg {
            // 1. 解析客户端发来的 JSON
            let client_msg: Result<ClientMsg, _> = serde_json::from_str(&text);

            match client_msg {
                Ok(cmd) => {
                    // 2. 根据指令处理逻辑
                    let response = match cmd {
                        ClientMsg::Ping => {
                            println!("收到 Ping");
                            ServerMsg::Pong
                        }
                        ClientMsg::Subscribe { topic } => {
                            println!("收到订阅: {}", topic);
                            // 保存 topic 到 HashSet
                            subscribed_topics.insert(topic.clone());
                            ServerMsg::Subscribed { topic }
                        }
                        ClientMsg::Unsubscribe { topic } => {
                            println!("收到取消订阅: {}", topic);
                            // 从 HashSet 删除 topic
                            subscribed_topics.remove(&topic);
                            ServerMsg::Unsubscribed { topic }
                        }
                    };

                    // 3. 发送响应回客户端
                    let response_text = serde_json::to_string(&response).unwrap();
                    if socket.send(Message::Text(response_text)).await.is_err() {
                        println!("发送消息失败，可能连接已断开");
                        break;
                    }
                }
                Err(_) => {
                    // JSON 格式不对
                    let err_msg = ServerMsg::Error {
                        msg: "无效的 JSON 格式".into(),
                    };
                    let _ = socket
                        .send(Message::Text(serde_json::to_string(&err_msg).unwrap()))
                        .await;
                }
            }
        }
    }
}
