#[allow(unused_imports)]
use std::time::Duration;
use tokio::sync::mpsc;
#[allow(unused_imports)]
use tokio::time::sleep;

// --- 为了演示回收效果，我们定义一个带 Drop 打印的马甲 ---
#[allow(dead_code)]
struct LoudReceiver {
    inner: mpsc::Receiver<String>,
}

// 实现 recv 方法，透传给内部的 mpsc
impl LoudReceiver {
    #[allow(dead_code)]
    async fn recv(&mut self) -> Option<String> {
        self.inner.recv().await
    }
}

// 关键：实现 Drop trait，当它从内存消失时会大喊一声
impl Drop for LoudReceiver {
    fn drop(&mut self) {
        println!("♻️ 垃圾回收车来了：data_rx 已经被彻底销毁 (Dropped)！");
    }
}

#[tokio::test]
async fn test_drop_in_loop() {
    let (data_tx, data_rx) = mpsc::channel::<String>(10);
    let (signal_tx, mut signal_rx) = mpsc::channel::<String>(10);

    // 1. 发送端 (模拟发完数据就跑)
    tokio::spawn(async move {
        for i in 1..=3 {
            data_tx.send(format!("Block #{}", i)).await.unwrap();
            sleep(Duration::from_millis(50)).await;
        }
        println!("==> 发送端已关闭");
    });

    // 2. 信号端 (模拟持续运行)
    tokio::spawn(async move {
        loop {
            sleep(Duration::from_millis(200)).await;
            if signal_tx.send("Heartbeat".to_string()).await.is_err() {
                break;
            }
        }
    });

    // --- 核心逻辑开始 ---

    // 3. 关键步骤：把接收端装进 Option 盒子里！
    // 此时所有权在 rx_opt 手里
    let mut rx_opt = Some(LoudReceiver { inner: data_rx });

    println!("Engine started...");

    loop {
        tokio::select! {
            // 4. 写法解析：
            // val = <Future>, if <Condition> => ...
            //
            // 这里的逻辑是：
            // A. 先检查 if rx_opt.is_some()。
            // B. 如果是 None，直接忽略这行，根本不会去执行 .unwrap()，所以安全。
            // C. 如果是 Some，才去执行 .as_mut().unwrap().recv()。
            val = async { rx_opt.as_mut().unwrap().recv().await }, if rx_opt.is_some() => {
                match val {
                    Some(data) => {
                        println!("Received data: {}", data);
                    }
                    None => {
                        println!("收到 None，准备回收接收端...");

                        // 💀 究极回收时刻 💀
                        // .take() 做了两件事：
                        // 1. 把 rx_opt 变成 None。
                        // 2. 把里面的 LoudReceiver 拿出来返回。
                        // 因为我们没有把返回结果赋值给任何变量，
                        // 这个 LoudReceiver 在这一行结束时立即判定为"没人要了"，
                        // 于是触发 Drop！
                        rx_opt.take();

                        // 此时，LoudReceiver 已经从内存里消失了！
                        // 接下来的 loop 依然在跑，但再也不会检查这个分支了。
                    }
                }
            }

            // 信号通道依然活着，证明 loop 没退，只是 rx 死了
            _ = signal_rx.recv() => {
                println!("Received signal (Heartbeat) - I'm still alive!");
                // 为了演示效果，收到两个心跳后退出
                break;
            }
        }
    }
}

#[test]
fn test_if_guard() {
    let number = Some(4);

    match number {
        // 语法：模式(Pattern) + if 条件(Guard) => 执行代码
        // 读作：“匹配 x，但仅当 x < 5 时”
        Some(x) if x < 5 => println!("这个数小于 5"),

        #[allow(unused_variables)]
        Some(x) => println!("其他数"),
        None => (),
    }
}

#[test]
fn test_match_guard() {
    let num = Some(10); // 这是一个大于 5 的数

    match num {
        // 分支 A：你的写法
        Some(x) => {
            // 程序进到这里了！因为 Some(10) 匹配 Some(x)
            if x < 5 {
                println!("太小了");
            } else {
                // 哎呀！这里怎么办？
                // 我想去执行下面的“正常处理”逻辑，但我已经进到分支 A 里面了！
                // 我没法“跳出去”让程序去试分支 B。
                // 我只能在这里把分支 B 的代码复制粘贴一遍……太蠢了。
            }
        }

        // 分支 B：备胎逻辑
        #[allow(unreachable_patterns)]
        Some(x) => {
            println!("正常处理: {}", x);
        }
        _ => {}
    }
}

#[test]
fn test_if_guard_2() {
    let num = Some(10);

    match num {
        // 分支 A：带门卫的匹配
        // 逻辑：是 Some(x) 吗？是的。那 x < 5 吗？不是！
        // 结果：门卫拦住了，不许进这个分支！请去下一个分支！
        Some(x) if x < 5 => println!("太小了: {}", x),

        Some(x) if x < 4 => println!("x < 3"),

        // 分支 B：备胎逻辑
        // 刚才分支 A 没进去，所以程序流到了这里。
        Some(x) => println!("正常处理: {}", x),
        None => (),
    }
}

#[test]
fn test_option_take_1() {
    let mut x = Some(String::from("Hello"));

    // 调用 take
    // 动作：把 hello 拿出来 给 y，同时把 x 变成 None
    let y = x.take();

    println!("x = {:?}", x); // x = None
    println!("y = {:?}", y); // y = Some("Hello")
}

#[allow(dead_code)]
mod option_tests {

    struct Student {
        name: Option<String>,
    }

    impl Student {
        fn get_name_ownership(&mut self) -> Option<String> {
            // 拿走 string，把 self.name 设为 None
            let n = self.name.take();
            n
        }
    }

    struct Node {
        next: Option<Box<Node>>,
    }

    impl Node {
        fn remove_next(&mut self) -> Option<Box<Node>> {
            self.next.take()
        }
    }

    struct Job {
        action: Option<Box<dyn FnOnce()>>,
    }

    impl Job {
        fn run(&mut self) {
            if let Some(task) = self.action.take() {
                task();
                println!("任务执行完毕，action 字段现在是 None 了")
            } else {
                println!("没有任务可执行了");
            }
        }
    }
}
