#[cfg(test)]
mod test_dead_lock {

    use std::sync::{Arc, Mutex};
    use std::thread;

    #[test]
    #[ignore = "dead-lock"]
    fn test_dead_lock() {
        // 两个共享资源：锁 A 和 锁 B
        let lock_a = Arc::new(Mutex::new(1));
        let lock_b = Arc::new(Mutex::new(2));

        // 克隆以便传入线程 1
        let a1 = Arc::clone(&lock_a);
        let b1 = Arc::clone(&lock_b);

        // 线程 1：先拿 A，再拿 B
        let handle1 = thread::spawn(move || {
            let _guard_a = a1.lock().unwrap();
            println!("Thread 1: Got Lock A, waiting for B...");

            // 模拟一点处理时间，让线程 2 有机会运行
            thread::sleep(std::time::Duration::from_millis(100));

            let _guard_b = b1.lock().unwrap(); // <--- 可能会卡在这里
            println!("Thread 1: Got Lock B!");
        });

        // 克隆以便传入线程 2
        let a2 = Arc::clone(&lock_a);
        let b2 = Arc::clone(&lock_b);

        // 线程 2：先拿 B，再拿 A (注意顺序！)
        let handle2 = thread::spawn(move || {
            let _guard_b = b2.lock().unwrap();
            println!("Thread 2: Got Lock B, waiting for A...");

            // 模拟一点处理时间
            thread::sleep(std::time::Duration::from_millis(100));

            let _guard_a = a2.lock().unwrap(); // <--- 可能会卡在这里
            println!("Thread 2: Got Lock A!");
        });

        handle1.join().unwrap();
        handle2.join().unwrap();
    }
}

#[allow(dead_code)]
#[allow(unused_variables)]
#[cfg(test)]
mod test_websocket {
    use serde::Deserialize;
    use smol_str::SmolStr;

    #[test]
    fn test_somol_str() {
        let s1 = SmolStr::new("hello");

        let string_val = String::from("World");
        let s2 = SmolStr::from(string_val);

        const MY_CONST: SmolStr = SmolStr::new_inline("Static string");

        println!("s1={}, s2={}", s1, s2);
    }

    use futures_util::StreamExt;
    use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
    use url::Url;

    // 定义接收到的 Trade 数据结构
    // 字段名映射参考 Binance 官方文档: https://binance-docs.github.io/apidocs/spot/en/#trade-streams
    #[derive(Debug, Deserialize)]
    struct TradeEvent {
        #[serde(rename = "e")]
        event_type: String, // 事件类型，如 "trade"

        #[serde(rename = "s")]
        symbol: String, // 交易对，如 "YBUSDT"

        #[serde(rename = "p")]
        price: String, // 成交价格 (字符串类型以保持精度)

        #[serde(rename = "q")]
        quantity: String, // 成交数量

        #[serde(rename = "T")]
        trade_time: u64, // 成交时间戳

        #[serde(rename = "m")]
        is_buyer_maker: bool, // 买方是否是做市商（true代表主动卖出，false代表主动买入）
    }

    #[tokio::test]
    async fn main() -> Result<(), Box<dyn std::error::Error>> {
        // 1. 设置 Binance WebSocket 地址
        // 格式: wss://stream.binance.com:9443/ws/<symbol>@trade
        // 注意: symbol 必须小写 (ybusdt)
        let binance_url = "wss://stream.binance.com:9443/ws/ybusdt@trade";

        // let url = Url::parse(binance_url)?;

        println!("正在连接到 Binance: {} ...", binance_url);

        // 2. 建立连接
        let (ws_stream, _) = connect_async(binance_url).await.expect("连接失败");
        println!("连接成功！开始接收 YB-USDT 成交数据...");

        // 3. 将流分为 读取(read) 和 写入(write) 部分
        // 我们只需要读取，所以忽略 write
        let (_, mut read) = ws_stream.split();

        // 4. 循环处理接收到的消息
        while let Some(message) = read.next().await {
            match message {
                Ok(msg) => {
                    // 处理文本消息
                    if let Message::Text(text) = msg {
                        // 尝试解析 JSON
                        match serde_json::from_str::<TradeEvent>(&text) {
                            Ok(trade) => {
                                // 打印解析后的数据
                                println!(
                                    "Token: {} | 价格: {} | 数量: {} | 时间: {} | 方向: {}",
                                    trade.symbol,
                                    trade.price,
                                    trade.quantity,
                                    trade.trade_time,
                                    if trade.is_buyer_maker {
                                        "卖单砸盘"
                                    } else {
                                        "买单吃货"
                                    }
                                );
                            }
                            Err(e) => eprintln!("解析JSON失败: {:?}", e),
                        }
                    }
                }
                Err(e) => {
                    eprintln!("WebSocket 错误: {:?}", e);
                    break;
                }
            }
        }

        Ok(())
    }
}
