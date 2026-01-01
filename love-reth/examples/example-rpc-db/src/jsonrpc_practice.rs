#[cfg(test)]
mod tests {
    use jsonrpsee::{
        core::RpcResult, core::async_trait, http_client::HttpClientBuilder, proc_macros::rpc,
        server::Server,
    };

    #[rpc(server, client, namespace = "demo")]
    pub trait HelloApi {
        #[method(name = "sayHello")]
        async fn say_hello(&self, name: String) -> RpcResult<String>;
    }

    pub struct HelloImpl;

    #[async_trait]
    impl HelloApiServer for HelloImpl {
        async fn say_hello(&self, name: String) -> RpcResult<String> {
            println!("say_hello called with name: {}", name);
            Ok(format!("Hello, {}! Welcome to Rust RPC world", name))
        }
    }

    #[tokio::test]
    async fn it_works() -> eyre::Result<()> {
        let server = Server::builder().build("127.0.0.1:0").await?;
        let addr = server.local_addr()?;

        println!("Server running at {}", addr);

        let handle = server.start(HelloImpl.into_rpc());

        let client = HttpClientBuilder::default().build(format!("http://{}", addr))?;

        let response = client.say_hello("Rust developer".to_string()).await?;

        println!("Response from server: {}", response);

        handle.stopped().await;

        Ok(())
    }
}
