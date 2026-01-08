use futures_util::StreamExt;
use prost::Message as ProstMessage; //以此别名引入，避免和 tungstenite::Message 冲突
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;

// 1. 引入生成的 Protobuf 代码
// prost 会把生成的代码放在 OUT_DIR 环境变量指定的目录里
pub mod binance_proto {
    include!(concat!(env!("OUT_DIR"), "/binance.rs"));
}
// 使用生成的结构体
use binance_proto::Trade;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 2. 设置 URL
    // 关键点：加上 ?responseFormat=proto (或者是 ?format=proto，具体看币安最新公告)
    // 这里假设我们连接的是支持 proto 的流
    let binance_url = "wss://stream.binance.com:9443/ws/ybusdt@trade?responseFormat=proto";
    let url = Url::parse(binance_url).unwrap();

    println!("正在连接 (Protobuf模式): {} ...", binance_url);

    let (ws_stream, _) = connect_async(url.to_string()).await.expect("连接失败");
    println!("连接成功！");

    let (_, mut read) = ws_stream.split();

    while let Some(msg) = read.next().await {
        match msg {
            Ok(message) => {
                match message {
                    // 3. 重点：处理二进制消息
                    Message::Binary(payload) => {
                        // 使用 prost 进行反序列化 (decode)
                        match Trade::decode(&payload[..]) {
                            Ok(trade) => {
                                println!(
                                    "PB数据 -> Symbol: {} | 价格: {} | 数量: {} | 时间: {}",
                                    trade.symbol, trade.price, trade.quantity, trade.trade_time
                                );
                            }
                            Err(e) => {
                                // 如果报错，通常说明你的 .proto 文件里的字段编号(Tag)和币安发的不一致
                                eprintln!("Protobuf 解码失败: {}", e);
                            }
                        }
                    }

                    // 币安偶尔还是会发 Text 类型的 Ping/Pong 或报错信息
                    Message::Text(text) => println!("收到文本消息: {}", text),

                    Message::Ping(_) => println!("收到 Ping"),
                    _ => {}
                }
            }
            Err(e) => eprintln!("WebSocket 错误: {:?}", e),
        }
    }

    Ok(())
}
