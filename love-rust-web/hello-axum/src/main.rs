use axum::{Json, Router, response::Html, routing::{get, post}};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;


#[tokio::main]// 启动 tokio 异步运行时
async fn main() {
    // 构建应用路由
    // 当用户访问根路径 / 时，调用 root 函数 
    // GET / 返回纯文本
    // POST /json 接收json返回json
    let app: Router = Router::new()
    .route("/", get(root))
    .route("/json", post(echo_json));

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
#[derive(Deserialize, Serialize)]
struct User {
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