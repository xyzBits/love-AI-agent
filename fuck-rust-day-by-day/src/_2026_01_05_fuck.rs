//! std::sync::Mutex 使用的是操作系统提供的线程锁
//! tokio::sync::Mutex 是 tokio提供的，有复杂的运行时
//! 如果确实需要持有锁跨越 .await ，那就使用 tokio::sync::Mutex
//! 否则尽量使用 std::sync::Mutex 或者 AtomicXxx 或者尽量缩小锁的粒度
#[cfg(test)]
mod test_tokio {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[tokio::test]
    #[allow(dead_code)]
    #[allow(unused_variables)]
    async fn test_mutex_1() {
        // 全局计数器，用 Arc<Mutex> 保护
        let counter = Arc::new(std::sync::Mutex::new(0));

        let mut handles = vec![];

        for _ in 0..10 {
            let counter_clone = counter.clone();

            handles.push(tokio::spawn(async move {
                // // 1. 获取锁
                // let mut guard = counter_clone.lock().unwrap();

                // // 这里模拟在段耗时的 io 操作，比如写日志或查库
                // // .awsit 会让当前线程让出执行权
                // tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;

                // // 3. 修改数据
                // *guard += 1;

                // // 4. 锁在这里自动释放
            }));
        }

        for ele in handles {
            let _ = ele.await;
        }
    }

    #[tokio::test]
    #[allow(dead_code)]
    #[allow(unused_variables)]
    async fn test_mutex_2() {
        // 全局计数器，用 Arc<Mutex> 保护
        let counter = Arc::new(tokio::sync::Mutex::new(0));

        let mut handles = vec![];

        for _ in 0..10 {
            let counter_clone = counter.clone();

            handles.push(tokio::spawn(async move {
                // 1. 获取锁
                let mut guard = counter_clone.lock().await;

                // 这里模拟在段耗时的 io 操作，比如写日志或查库
                // .awsit 会让当前线程让出执行权
                tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;

                // 3. 修改数据
                *guard += 1;

                // 4. 锁在这里自动释放
            }));
        }

        for ele in handles {
            let _ = ele.await;
        }

        println!("counter={:?}", counter);
    }

    #[tokio::test]
    #[allow(dead_code)]
    #[allow(unused_variables)]
    async fn test_mutex_3() {
        // 全局计数器，用 Arc<Mutex> 保护
        let counter = Arc::new(std::sync::Mutex::new(0));

        let mut handles = vec![];

        for _ in 0..10 {
            let counter_clone = counter.clone();

            handles.push(tokio::spawn(async move {
                // 先修改数据，再释放锁
                {
                    // 1. 获取锁
                    let mut guard = counter_clone.lock().unwrap();
                    // 3. 修改数据
                    *guard += 1;
                }
                // 这里模拟在段耗时的 io 操作，比如写日志或查库
                // .awsit 会让当前线程让出执行权
                tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;

                // 4. 锁在这里自动释放
            }));
        }

        for ele in handles {
            let _ = ele.await;
        }

        println!("counter={:?}", counter);
    }

    #[tokio::test]
    #[allow(dead_code)]
    #[allow(unused_variables)]
    async fn test_mutex_4() {
        // 1. 初始化
        // 注意：这里不需要 Mutex，直接包裹 AtomicUsize 即可
        // AtomicUsize::new(0) 创建一个初始值为 0 的原子整数
        let counter = Arc::new(AtomicUsize::new(0));

        let mut handles = vec![];

        println!("开始执行任务...");

        // 模拟启动 100 个并发任务
        for i in 0..100 {
            let counter_clone = counter.clone();

            handles.push(tokio::spawn(async move {
                // 2. 模拟耗时操作 (IO)
                // 在 Atomic 模式下，sleep 前后都不涉及锁，所以完全不必担心 await 问题
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

                // 3. 原子加法
                // fetch_add(1, ...) 相当于 counter += 1
                // 返回值是"加之前"的值（旧值），如果不需要旧值可以直接忽略
                let prev_val = counter_clone.fetch_add(1, Ordering::Relaxed);

                // 只是为了演示，打印一下（实际高并发中不要频繁 println，会影响性能）
                if i % 10 == 0 {
                    println!("任务 {} 完成，之前的值是 {}", i, prev_val);
                }
            }));
        }

        // 等待所有任务完成
        for h in handles {
            let _ = h.await;
        }

        // 4. 读取最终结果
        // load(...) 用于读取当前值
        let final_count = counter.load(Ordering::Relaxed);

        println!("-----------------------");
        println!("最终计数结果: {}", final_count);
        assert_eq!(final_count, 100);
    }
}

#[cfg(test)]
mod conditional_var_tests {
    use std::{
        sync::{Arc, Condvar, Mutex},
        thread,
        time::Duration,
    };

    #[test]
    fn test_1() {
        // 1. 创建共享数据：一个锁（保护数据） + 一个条件变量（负责通知）
        let pair = Arc::new((Mutex::new(false), Condvar::new()));
        let pair2 = pair.clone();

        // 2. 启动等待线程 (消费者)
        thread::spawn(move || {
            // &*pair 的意思是：“把 Arc 的外壳剥掉，拿到里面的数据，然后借用它。”
            let (lock, cvar) = &*pair2; // 解构 Arc 里的内容

            // 步骤 A: 获取锁
            let mut started = lock.lock().unwrap();

            // 步骤 B: 循环检查条件 (重点！必须用 while)
            while !*started {
                println!("线程: 条件不满足，我睡了...");
                // 步骤 C: 等待
                // wait() 会消耗掉 guard (即释放锁)，并挂起当前线程
                // 当被唤醒时，它会重新获取锁，并返回一个新的 guard

                // vvvvvvvvv 步骤 1：线程在这里挂起 (Pause) vvvvvvvvv
                started = cvar.wait(started).unwrap();
                // ^^^^^^^^^ 步骤 2：醒来 (Resume) 从这里返回 ^^^^^^^^^
            }

            println!("线程: 被唤醒了！开始干活！");
        });

        // 3. 主线程模拟一些工作 (生产者)
        println!("主线程: 正在准备数据...");
        thread::sleep(Duration::from_secs(2));

        let (lock, cvar) = &*pair;

        // 步骤 D: 修改状态
        let mut started = lock.lock().unwrap();
        *started = true; // 修改条件

        // 步骤 E: 通知
        println!("主线程: 数据好了，通知它！");
        cvar.notify_one(); // 唤醒一个正在 wait 的线程
    }
}

#[cfg(test)]
#[allow(dead_code)]
mod test_actor {
    use tokio::sync::{mpsc, oneshot};

    // --- 1. 定义消息 message -----
    // 使用 enum 是最常见的方式
    enum MyActorMessage {
        // 这种消息只管发，不求回
        SayHello(String),

        // 这种消息是请求响应 模式，需要带一个回信地址
        GetCount(oneshot::Sender<u32>),
    }

    // ---- 2. 定义 actor 后台打工人 -------------
    struct MyActor {
        receiver: mpsc::Receiver<MyActorMessage>, // 收件箱
        count: u32,                               //私有状态，注意不要用 mutex
    }

    impl MyActor {
        // 核心循环：不断处理收件箱中的消息
        async fn run(mut self) {
            while let Some(msg) = self.receiver.recv().await {
                match msg {
                    MyActorMessage::SayHello(name) => {
                        println!("Hello, {name}");
                        self.count += 1;
                    }

                    MyActorMessage::GetCount(respond_to) => {
                        // 把当前状态发回去
                        let _ = respond_to.send(self.count);
                    }
                }
            }
        }
    }

    // --- 3. 定义 handle 遥控器
    // 这个结构体是可以被 clone 并到处传的
    #[derive(Clone)]
    pub struct MyActorHandle {
        sender: mpsc::Sender<MyActorMessage>,
    }

    impl MyActorHandle {
        pub fn new() -> Self {
            let (sender, receiver) = mpsc::channel(32); // 创建信道

            let actor = MyActor {
                receiver: receiver,
                count: 0,
            };

            // 关键点，把 Actor 扔到后台去跑 spawm task
            tokio::spawn(async move {
                actor.run().await;
            });

            Self { sender: sender }
        }

        // 封闭发送逻辑，对用户隐藏 channel 细节
        pub async fn say_hell0(&self, name: String) {
            let _ = self.sender.send(MyActorMessage::SayHello(name)).await;
        }

        pub async fn get_count(&self) -> u32 {
            // 创建一个一次性信道用于接收回执
            let (sender, receiver) = oneshot::channel();

            // 发送请求，发 回信地址 附带过去
            let _ = self.sender.send(MyActorMessage::GetCount(sender)).await;

            // 等待回信
            receiver.await.unwrap()
        }

        // thiserror 定义一个巨大的 enum Error ，列出所有的可能 ，让调用者去 match ，调用者需要知道具体是哪种错误，以便处理
        // anyhow anyhow::Result<T> 可以吞下任何错误，不需要处理特定错误，只要把错误链条打印出来 给开发者看
    }
}
