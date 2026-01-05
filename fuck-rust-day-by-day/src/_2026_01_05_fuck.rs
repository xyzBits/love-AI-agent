
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
