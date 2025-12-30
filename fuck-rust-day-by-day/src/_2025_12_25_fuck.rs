#[allow(unused_variables)]
#[allow(unused_assignments)]
#[allow(unused)]
#[tokio::test]
async fn test_busy_loop_trap() {
    // 1. 创建两个通道
    // data_channel 模拟下载区块的数据流
    let (data_tx, mut data_rx) = tokio::sync::mpsc::channel::<String>(10);

    // signal_channel: 模拟控制信号，比如心跳检测
    let (signal_tx, mut signal_rx) = tokio::sync::mpsc::channel::<String>(10);

    // 2. 启动一个生产者任务， 模拟 网络下载
    tokio::spawn(async move {
        for i in 0..=3 {
            data_tx.send(format!("Block #{}", i)).await.unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }

        println!("==> Data Sender finished and dropped");
        // data_tx 在这里被销毁 drop 通道关闭
    });

    // 3. 启动另一个生产者任务，模拟偶尔心跳
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            if signal_tx.send("Heartbeat".to_string()).await.is_err() {
                break;
            }
        }
    });

    // 4. 主循环 engine loop，痛苦陷阱在这里
    println!("Engine started...");

    #[allow(unused_variables)]
    let mut count = 0;

    // 标记数据通道是否还活着
    let mut data_channel_active = true;
    loop {
        tokio::select! {
            // 监听数据通道
            val = data_rx.recv(), if data_channel_active => {
                // // 这里逻辑看似正常，收到数据就处理
                // println!("Received data: {:?}", val);
                // count += 1;
                match val {
                    Some(data) => {
                        // 正常收到数据
                        println!("Received data: {}", data);
                        count += 1;
                    }
                    None => {
                        // 收到 None，说明发送端 sender 已经 全部销毁了
                        // 这是一个永久性的，以后永远只会有 None
                        println!("Data channel closed! Exiting loop.");
                        // break;// 退出整个 loop
                        // 修改标记，不再监听这个通道，但loop 继续 跑
                        data_channel_active = false;
                    }
                }
            }

            // 监听信号通道
            _ = signal_rx.recv() => {
                println!("Received signal (Heartbeat)");
            }
        }

        // 安全阀，为了防止你的电脑卡死，跑20次循环强制退出
        // 如果逻辑正确，应该收到 3 个数据后就退出了，不用等到 20 次
        // if count >= 20 {
        //     println!("Detected busy loop! Force quiting");
        //     break;
        // }

        if !data_channel_active {
            println!("Only listening to heartbeats now...");
        }
    }
}
