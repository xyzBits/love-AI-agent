#[cfg(test)]
mod tests {
    use jsonrpsee::core::RpcResult;
    use jsonrpsee::http_client::HttpClientBuilder;
    use jsonrpsee::proc_macros::rpc;
    use jsonrpsee::server::ServerBuilder;

    // --- 第一步：定义 RPC 接口 ---
    // 这个宏会自动生成：
    // 1. MyRpcServer trait (供服务端实现)
    // 2. MyRpcClient trait (供客户端调用)
    #[rpc(server, client)]
    pub trait MyRpc {
        #[method(name = "say_hello")]
        async fn say_hello(&self, name: String) -> RpcResult<String>;

        #[method(name = "add")]
        async fn add(&self, a: i32, b: i32) -> RpcResult<i32>;
    }

    // --- 第二步：实现服务端逻辑 ---
    struct MyRpcServerImpl;

    #[jsonrpsee::core::async_trait]
    impl MyRpcServer for MyRpcServerImpl {
        async fn say_hello(&self, name: String) -> RpcResult<String> {
            Ok(format!("你好, {}! 这是来自 jsonrpsee 的回复。", name))
        }

        async fn add(&self, a: i32, b: i32) -> RpcResult<i32> {
            Ok(a + b)
        }
    }

    #[tokio::test]
    async fn main() -> anyhow::Result<()> {
        // 1. 启动服务端
        let server = ServerBuilder::default().build("127.0.0.1:0").await?;
        let addr = server.local_addr()?;
        let handle = server.start(MyRpcServerImpl.into_rpc());

        // 2. 创建客户端并连接
        let url = format!("http://{}", addr);
        let client = HttpClientBuilder::default().build(url)?;

        // 3. 像调用本地函数一样调用远程接口
        // 注意：MyRpcClient 是宏自动生成的
        let response = client.say_hello("Gemini".to_string()).await?;
        println!("客户端收到回复: {}", response);

        let sum = client.add(10, 20).await?;
        println!("10 + 20 = {}", sum);

        // 停止服务
        handle.stop()?;
        handle.stopped().await;
        Ok(())
    }
}
