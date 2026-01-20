use rdkafka::Message;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::producer::{FutureProducer, FutureRecord};
use serde::{Deserialize, Serialize};
use tokio::time::{Duration, sleep};

#[derive(Serialize, Deserialize, Debug)]
struct KafkaMessage {
    content: String,
    timestamp: u64,
}

#[tokio::test]
#[ignore = "需要 Kafka 服务运行在 localhost:9092"]
async fn test_kafka_producer_consumer() {
    let topic = "fuck-kafka";
    let brokers = "localhost:9092";

    let producer_handle = tokio::spawn(kafka_producer(brokers.to_string(), topic.to_string()));
    let consumer_handle = tokio::spawn(kafka_consumer(brokers.to_string(), topic.to_string()));

    tokio::select! {
        _ = producer_handle => println!("Producer 结束"),
        _ = consumer_handle => println!("Consumer 结束"),
    }
}

async fn kafka_producer(brokers: String, topic: String) {
    println!("🚀 启动 Kafka Producer...");

    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &brokers)
        .set("message.timeout.ms", "5000")
        .create()
        .expect("创建 Producer 失败");

    let mut counter = 0u64;

    loop {
        counter += 1;
        let message = KafkaMessage {
            content: "hello world".to_string(),
            timestamp: counter,
        };
        let json_payload = serde_json::to_string(&message).expect("序列化失败");
        println!("📤 [Producer] 发送消息 #{}: {}", counter, json_payload);

        let delivery_status = producer
            .send(
                FutureRecord::to(&topic)
                    .payload(&json_payload)
                    .key(&format!("key-{}", counter)),
                Duration::from_secs(0),
            )
            .await;

        match delivery_status {
            Ok(_) => println!("✅ [Producer] 消息 #{} 发送成功", counter),
            Err((e, _)) => eprintln!("❌ [Producer] 发送失败: {:?}", e),
        }

        sleep(Duration::from_secs(1)).await;
    }
}

async fn kafka_consumer(brokers: String, topic: String) {
    println!("🎧 启动 Kafka Consumer...");

    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", "fuck-kafka-group")
        .set("bootstrap.servers", &brokers)
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "true")
        .set("auto.offset.reset", "earliest")
        .create()
        .expect("创建 Consumer 失败");

    consumer.subscribe(&[&topic]).expect("订阅 topic 失败");
    println!("✅ [Consumer] 已订阅 topic: {}", topic);

    loop {
        match consumer.recv().await {
            Ok(message) => {
                if let Some(payload) = message.payload() {
                    let payload_str = String::from_utf8_lossy(payload);
                    match serde_json::from_str::<KafkaMessage>(&payload_str) {
                        Ok(kafka_msg) => {
                            println!(
                                "📥 [Consumer] 收到消息: content='{}', timestamp={}",
                                kafka_msg.content, kafka_msg.timestamp
                            );
                        }
                        Err(e) => eprintln!("❌ [Consumer] JSON 解析失败: {:?}", e),
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ [Consumer] 接收消息失败: {:?}", e);
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
}
