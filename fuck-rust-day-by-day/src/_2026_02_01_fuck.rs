mod executor_practice {
    use std::collections::VecDeque;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};
    use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

    // ==========================================
    // 第一步：定义 Task（任务）
    // ==========================================

    /// 一个可执行的任务
    ///
    /// Task 是 Executor 调度的基本单位，包含：
    /// 1. 一个 Future（要执行的异步逻辑）
    /// 2. 一个指向任务队列的引用（wake 时把自己放回去）
    struct Task {
        /// 被 Pin 住的 Future
        /// - Pin: 防止 Future 被移动（有些 Future 有自引用）
        /// - Box<dyn Future>: 类型擦除，可以存放任意 Future
        /// - Send: 可以跨线程传递
        /// - Mutex: 因为可能被多线程访问（wake 可能在其他线程调用）
        future: Mutex<Pin<Box<dyn Future<Output = ()> + Send>>>,

        /// 任务队列的引用
        /// wake() 时需要把自己放回这个队列
        queue: Arc<TaskQueue>,
    }

    // ==========================================
    // 第二步：定义任务队列
    // ==========================================

    /// 任务队列
    ///
    /// 这是 Executor 的核心数据结构
    /// - 新任务通过 push 加入
    /// - Executor 通过 pop 取出任务执行
    /// - wake() 时任务会被重新 push 回来
    struct TaskQueue {
        /// 用 VecDeque 实现 FIFO 队列
        /// Mutex 保证线程安全
        queue: Mutex<VecDeque<Arc<Task>>>,
    }

    impl TaskQueue {
        fn new() -> Self {
            TaskQueue {
                queue: Mutex::new(VecDeque::new()),
            }
        }

        /// 添加任务到队列尾部
        fn push(&self, task: Arc<Task>) {
            self.queue.lock().unwrap().push_back(task);
        }

        /// 从队列头部取出任务
        fn pop(&self) -> Option<Arc<Task>> {
            self.queue.lock().unwrap().pop_front()
        }
    }

    // ==========================================
    // 第三步：实现 Waker（最核心的部分）
    // ==========================================
    //
    // Waker 的作用：当 Future 返回 Pending 时，它需要一种方式
    // 告诉 Executor "我准备好了，再来 poll 我"
    //
    // Waker 内部结构：
    // - data: *const ()  -- 指向 Task 的裸指针
    // - vtable: &RawWakerVTable -- 函数表，定义 clone/wake/drop 等操作
    //
    // 为什么用裸指针？
    // - 避免循环引用（Task 持有 queue，queue 持有 Task）
    // - 让用户自己管理生命周期

    /// 创建 Waker
    ///
    /// 把 Arc<Task> 转成 Waker，这样 Future 就可以通过
    /// cx.waker().wake() 来唤醒自己
    fn create_waker(task: Arc<Task>) -> Waker {
        // 1. 把 Arc<Task> 转成裸指针
        //    Arc::into_raw 会"忘记" Arc，不会减少引用计数
        //    返回的指针指向 Task 数据
        let ptr = Arc::into_raw(task) as *const ();

        // 2. 创建 RawWaker
        //    - ptr: 数据指针，指向我们的 Task
        //    - VTABLE: 函数表，定义如何 clone/wake/drop
        let raw_waker = RawWaker::new(ptr, &VTABLE);

        // 3. 转成 Waker
        //    unsafe 是因为我们要保证 VTABLE 的实现是正确的
        unsafe { Waker::from_raw(raw_waker) }
    }

    /// Waker 的函数表（虚函数表）
    ///
    /// 类似 C++ 的 vtable，定义了 4 个操作：
    /// - clone: 克隆 Waker
    /// - wake: 唤醒任务（消费 Waker）
    /// - wake_by_ref: 唤醒任务（不消费 Waker）
    /// - drop: 释放 Waker
    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone_fn, wake_fn, wake_by_ref_fn, drop_fn);

    /// clone: 克隆 Waker
    ///
    /// 当调用 waker.clone() 时执行
    fn clone_fn(ptr: *const ()) -> RawWaker {
        // 1. 从裸指针恢复 Arc<Task>
        let arc = unsafe { Arc::from_raw(ptr as *const Task) };

        // 2. 克隆 Arc（增加引用计数）
        let cloned = arc.clone();

        // 3. 忘记原来的 Arc，不要减少引用计数
        //    因为原来的 Waker 还在用这个指针
        std::mem::forget(arc);

        // 4. 返回新的 RawWaker
        RawWaker::new(Arc::into_raw(cloned) as *const (), &VTABLE)
    }

    /// wake: 唤醒任务（消费 Waker）
    ///
    /// 当调用 waker.wake() 时执行
    /// 这个方法会消费 Waker，所以不需要 forget
    fn wake_fn(ptr: *const ()) {
        // 1. 从裸指针恢复 Arc<Task>
        //    这会"接管"这个指针的所有权
        let arc = unsafe { Arc::from_raw(ptr as *const Task) };

        // 2. 把任务放回队列
        //    clone 是因为 push 需要 Arc，而我们还要让 arc 被 drop
        arc.queue.push(arc.clone());

        // 3. arc 在这里被 drop，引用计数 -1
        //    但因为我们 clone 了一份放进队列，所以 Task 不会被释放
    }

    /// wake_by_ref: 唤醒任务（不消费 Waker）
    ///
    /// 当调用 waker.wake_by_ref() 时执行
    /// 这个方法不消费 Waker，所以需要 forget
    fn wake_by_ref_fn(ptr: *const ()) {
        // 1. 从裸指针恢复 Arc<Task>
        let arc = unsafe { Arc::from_raw(ptr as *const Task) };

        // 2. 把任务放回队列
        arc.queue.push(arc.clone());

        // 3. 忘记 arc，不要减少引用计数
        //    因为原来的 Waker 还在用这个指针
        std::mem::forget(arc);
    }

    /// drop: 释放 Waker
    ///
    /// 当 Waker 被 drop 时执行
    fn drop_fn(ptr: *const ()) {
        // 从裸指针恢复 Arc<Task>
        // 这个 Arc 会在函数结束时被 drop，引用计数 -1
        unsafe { Arc::from_raw(ptr as *const Task) };
    }

    // ==========================================
    // 第四步：实现 Executor
    // ==========================================

    /// 简易 Executor
    ///
    /// Executor 的职责：
    /// 1. 接收用户提交的 Future（spawn）
    /// 2. 循环从队列取任务，poll 它们（run）
    /// 3. 当任务返回 Pending 时，等待 wake
    /// 4. 当任务返回 Ready 时，任务完成
    pub struct SimpleExecutor {
        queue: Arc<TaskQueue>,
    }

    impl SimpleExecutor {
        pub fn new() -> Self {
            SimpleExecutor {
                queue: Arc::new(TaskQueue::new()),
            }
        }

        /// 提交一个 Future 到 Executor
        ///
        /// 这个方法把 Future 包装成 Task，放入队列
        pub fn spawn<F>(&self, future: F)
        where
            F: Future<Output = ()> + Send + 'static,
        {
            // 创建 Task
            let task = Arc::new(Task {
                future: Mutex::new(Box::pin(future)),
                queue: self.queue.clone(),
            });

            // 放入队列
            self.queue.push(task);
        }

        /// 运行 Executor，直到所有任务完成
        ///
        /// 核心循环：
        /// 1. 从队列取任务
        /// 2. 创建 Waker
        /// 3. poll 任务
        /// 4. 如果 Pending，等 wake 把任务放回队列
        /// 5. 如果 Ready，任务完成
        /// 6. 队列空了就结束
        pub fn run(&self) {
            // 循环直到队列为空
            while let Some(task) = self.queue.pop() {
                // 1. 为这个任务创建 Waker
                //    clone 是因为 create_waker 会消费 Arc
                let waker = create_waker(task.clone());

                // 2. 创建 Context
                //    Context 是 poll 的参数，里面包含 Waker
                let mut cx = Context::from_waker(&waker);

                // 3. 获取 Future 的锁
                let mut future = task.future.lock().unwrap();

                // 4. poll Future
                match future.as_mut().poll(&mut cx) {
                    Poll::Ready(()) => {
                        // 任务完成，不需要做任何事
                        // Task 会在 Arc 引用计数归零时被释放
                    }
                    Poll::Pending => {
                        // 任务未完成
                        // Future 内部应该已经调用了 wake_by_ref()
                        // 把任务放回了队列，所以我们不需要做任何事
                    }
                }
            }
        }
    }

    // ==========================================
    // 测试用的 Future
    // ==========================================

    /// 简单的倒计时 Future
    struct CountDown {
        remaining: u32,
        name: &'static str,
    }

    impl CountDown {
        fn new(count: u32, name: &'static str) -> Self {
            CountDown {
                remaining: count,
                name,
            }
        }
    }

    impl Future for CountDown {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            if self.remaining == 0 {
                println!("[{}] ✅ 完成!", self.name);
                Poll::Ready(())
            } else {
                println!("[{}] 还剩 {} 次", self.name, self.remaining);
                self.remaining -= 1;
                // 唤醒自己，让 Executor 再次 poll
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }

    // CountDown 没有自引用，所以可以安全地 Unpin
    impl Unpin for CountDown {}

    // ==========================================
    // 测试
    // ==========================================

    #[test]
    fn test_simple_executor() {
        println!("=== 启动 SimpleExecutor ===\n");

        let executor = SimpleExecutor::new();

        // 提交两个任务
        executor.spawn(CountDown::new(3, "Task-A"));
        executor.spawn(CountDown::new(2, "Task-B"));

        // 运行 Executor
        executor.run();

        println!("\n=== 所有任务完成 ===");
    }

    #[test]
    fn test_single_task() {
        println!("=== 测试单个任务 ===\n");

        let executor = SimpleExecutor::new();
        executor.spawn(CountDown::new(5, "Solo"));
        executor.run();

        println!("\n=== 完成 ===");
    }
}
