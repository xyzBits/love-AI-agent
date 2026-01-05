//! unsafe 并不代表不安全，代表由程序员来保证安全，而不是编译器
//! unsafe 并不能违背 rust 的所有权和借用规则 ，给了5种特权
//! 1。解引用 raw pointer
//! 2。call unsafe functions
//! 3。access or modify mutable static
//! 4。implement unsafe traits
//! 5。 访问 union 的字段
#[cfg(test)]
mod unsafe_tests {
    use core::slice;
    /// borrow checker 只盯着借用 & 看，对于裸指针 * ，编译器把它当作一个普通的数字，内存地址
    /// 你可以同时拥有 100 个 指向同一个地址的可变裸指针，也可以同时拥有可变和不可变裸指针
    /// 如果在unsafe 中通过两个指针写同一块内存，就会出错
    #[test]
    fn test_1() {
        let mut num = 5;

        // 1. 创建裸指针，这是安全的，不需要 unsafe
        // 把一个引用 &num 强转为 裸指针
        let r1 = &num as *const i32;
        let r2 = &mut num as *mut i32;

        println!("指针地址: {:?}", r1);

        // 2. 解引用裸指针，这是危险的，必须 用 unsafe
        unsafe {
            // 编译器：我不确定 r1 指向的内存是否还是活着的，你自己负责
            println!("r1 指向的值是: {:?}", r1);

            // 修改值
            *r2 = 10;
            println!("r2 指向的值修改后是: {}", *r2);
        }

        // 回到安全世界
        println!("num 现在是: {}", num);
    }

    #[allow(unused_doc_comments)]
    /// 假设这是一个 C 语言标准库的函数  abs
    /// extern C 告诉编译器这个函数用 C 的调用约定
    /// application binary interface 调用约定
    unsafe extern "C" {
        unsafe fn abs(input: i32) -> i32;
    }

    #[test]
    fn test_2() {
        unsafe {
            // 调用外部函数必须在 unsafe 块中
            println!("-3 的绝对值是: {}", abs(-3));
        }
    }

    #[allow(dead_code)]
    #[allow(unused_variables)]
    fn split_at_mut(values: &mut [i32], mid: usize) -> (&mut [i32], &mut [i32]) {
        let len = values.len();
        // 获取 裸指针
        let ptr = values.as_mut_ptr();

        assert!(mid <= len);

        unsafe {
            // 我们利用裸指针算术，绕过借用检查器
            // 程序员保证：这两个切片在内存上绝对不重叠
            (
                slice::from_raw_parts_mut(ptr, mid),                // 前半部分
                slice::from_raw_parts_mut(ptr.add(mid), len - mid), // 后半部分
            )
        }
    }

    // #[allow(dead_code)]
    // fn split_at_mut_safe(values: &mut [i32], mid: usize) -> (&mut [i32], &mut [i32]) {
    //     let len = values.len();
    //     assert!(mid <= len);

    //     // 也就是：返回 0..mid 的可变引用，和 mid..len 的可变引用
    //     // 看起来它们不重叠，对吧？
    //     //  second mutable borrow occurs here
    //     (&mut values[0..mid], &mut values[mid..len])
    // }

    #[test]
    fn test_3() {
        let mut v = vec![1, 2, 3, 4, 5, 6];

        // 调用者不需要写 unsafe 因为函数内部已经处理好边界，对外 是安全的
        let (a, b) = split_at_mut(&mut v, 3);

        a[0] = 100;
        b[0] = 200;

        println!("v = {:?}", v);
    }

    #[test]
    #[ignore = "out of bond"]
    fn test_4() {
        #[allow(unused_unsafe)]
        unsafe {
            let v = vec![1, 2, 3];
            println!("v[100] = {}", v[100]);
        }
    }

    #[test]
    #[ignore = "unsafe"]
    fn test_5() {
        let p: *const i32 = std::ptr::null();
        unsafe {
            // 试图读取空指针
            println!("null ptr: {}", *p);
        }
    }

    #[test]
    #[ignore = "miri"]
    fn test_6() {
        let mut v = vec![1, 2, 3, 4];
        let ptr = v.as_mut_ptr();

        unsafe {
            // 假设你要访问第4个元素，越界了
            *ptr.add(4) = 999;
        }

        println!("程序看起来还在正常运行");
    }
}

#[cfg(test)]
mod send_sync_tests {
    use std::{
        cell::RefCell,
        sync::{Arc, Mutex},
        thread,
    };

    #[test]
    fn test_1() {
        let r = RefCell::new(10);

        thread::spawn(move || {
            println!("RefCell: {:?}", r.borrow());
        });
    }

    #[test]
    #[ignore = "failed"]
    #[allow(dead_code)]
    #[allow(unused_variables)]
    fn test_fail() {
        let r = Arc::new(RefCell::new(10));

        let r_for_thread = r.clone();

        thread::spawn(move || {
            // println!("RefCell: {:?}", r_for_thread);
        });
    }

    #[test]
    fn test_mutex() {
        // 数据 0 被装进了 Mutex，然后用 Arc 包装以便跨线程共享
        let counter = Arc::new(Mutex::new(0));
        let mut handles = vec![];

        for _ in 0..10 {
            let counter = Arc::clone(&counter);
            handles.push(thread::spawn(move || {
                // 1. 拿到锁，如果别人拿着，这里会阻塞
                let mut num = counter.lock().unwrap();

                // 2. 修改数据，通过 deref 自动解引用
                *num += 1;

                // 3. 离开作用域，num 被 drop，锁自动释放
            }));
        }

        for ele in handles {
            ele.join().unwrap();
        }

        println!("Result: {}", *counter.lock().unwrap());
    }
}

#[cfg(test)]
mod interview_tests {
    use std::mem;

    #[allow(dead_code)]
    unsafe fn string_to_vec(s: String) -> Vec<u8> {
        let ptr = s.as_ptr();
        let len = s.len();
        let cap = s.capacity();

        // 防止 s 在函数结束运行时析构函数 drop，否则 ptr 指向的内存就被释放了
        mem::forget(s);

        unsafe { Vec::from_raw_parts(ptr as *mut u8, len, cap) }
    }

    /// as_ptr 拿到数据在内存中的真实物理地址
    /// 返回一个裸指针，类型通常是 *const T
    /// String 内部是 Vec<u8> 和普通的 Vec<u8> 有什么区别，String 是合法的 UTF-8 序列，而Vec<u8> 里面什么都能塞，是个垃圾桶
    #[allow(dead_code)]
    unsafe fn vec_to_string(v: Vec<u8>) -> String {
        let ptr = v.as_ptr();
        let len = v.len();
        let cap = v.capacity();

        mem::forget(v);

        unsafe { String::from_raw_parts(ptr as *mut u8, len, cap) }
    }

    #[test]
    fn test_string_to_vec() {
        let s = String::from("Hello Reth");

        // 1. String ---> Vec<u8>
        let v = unsafe { string_to_vec(s) };

        println!("Vec: {:?}", v);

        // 2. Vec<u8> ---> String
        let s_back = unsafe { vec_to_string(v) };

        println!("String back: {}", s_back);
    }

    #[test]
    fn test_2() {
        let s = String::from("Hello Reth");

        // 1. String -> Vec<u8> Safe & Zero Cost
        let v = s.into_bytes();
        println!("Vec: {:?}", v);

        // 2. Vec<u8> -> String safe
        let s_back = String::from_utf8(v).expect("Invalid UTF-8");
        println!("String back: {}", s_back);
    }

    /// 什么是 zero copy，就像是 原地排序一样，不开辟新的内存，直接原地操作
    /// let v = s.into_bytes() 时，内存里到底发生了什么
    /// 转换前  
    ///     ptr: 0x1000 指向堆内存
    ///     len: 5
    ///     cap: 5
    ///     堆上： H e l l o
    /// 转换后
    ///     ptr:　0x1000
    ///     len: 5  
    ///     cap: 5
    ///     堆上：还是 H e l l o
    /// 区别在于编译器对待这块内存的态度(类型)变了
    /// 以前叫 String 编译器会禁止你在里面存非 utf-8 字符
    /// 现在叫 Vec<u8> 编译器允许你在里面存任何字节
    #[test]
    fn test_3() {
        let s = String::from("Hello World"); // 假设这是大概 1GB 的数据
        // 笨办法 deep copy
        // 分配新内存，复制数据，s 依然存在
        let _v_copy = s.clone().into_bytes();

        // zero copy
        // 仅仅是转移所有权，s 消失了，v 接管了 s 的堆内存
        // 底层没有发生任何堆内存的分配和复制
        let _v_zero_copy = s.into_bytes();
    }
}

#[cfg(test)]
mod inner_mut_tests {
    use std::{cell::RefCell, collections::HashMap, sync::{Arc, RwLock}, thread};

    use dashmap::DashMap;

    #[allow(dead_code)]
    struct LocalCache {
        // 使用 RefCell 实现内部可变性，对外只需要 &self 就能修改数据
        map: RefCell<HashMap<String, String>>,
    }

    impl LocalCache {
        fn new() -> Self {
            LocalCache {
                map: RefCell::new(HashMap::new()),
            }
        }

        // 如果 key 存在，返回 value，如果不存在，则 插入 default 并返回
        fn get_or_insert(&self, key: &str) -> String {
            // 获取不可变借用，检查  key 是否存在
            // 内部借用计数+1，变量map_ref 一直活着，作用域持续到函数结束

            {// 在这个块结束时，读锁会被释放
                let map_ref = self.map.borrow();

                if let Some(v) = map_ref.get(key) {
                    return v.clone();
                }
            }

            // drop(map_ref);
            // 走到这里时，map_ref 还没有被  drop 读锁依然被持有
            // 这里试图获取 可变借用 write lock
            // 规则：有读锁时，禁止获取写锁
            // 结果 RefCell 在运行时检查到违规，直接 panic
            // 2. 如果不存在，获取可变借用，插入数据
            self.map
                .borrow_mut()
                .insert(key.to_string(), "default".to_string());

            "default".into()
        }


        #[allow(dead_code)]
        fn get_or_insert_v2(&self, key: &str) -> String {
            // 直接获取可变借用，一步到位
            let mut map_mut = self.map.borrow_mut();

            // 1. entry(key) 查找  key
            // 2. or_inser() 如果空就插入，如果不空就返回已有的引用
            // 3. clone() 拿到值
            map_mut.entry(key.to_string())
            .or_insert("default".to_string())
            .clone()
        }
    }

    #[test]
    fn test_1() {
        let cache = LocalCache::new();

        // 第一次调用，应该插入
        println!("First call: {}", cache.get_or_insert("user_1"));

        // 第二次调用，应该直接返回
        println!("Second call: {}", cache.get_or_insert("user_1"));
    }



    /// Clone 可以让我们轻易的把引用计数复制给多个线程
    #[derive(Clone)]
    struct ThreadSafeCache {

        // Arc: 让 Cache 可以被多个线程持有
        // RwLock： 替代 RefCell，提供线程安全的读写锁
        map: Arc<RwLock<HashMap<String, String>>>,

    }


    impl ThreadSafeCache {
        fn new() -> Self {
            ThreadSafeCache { map: Arc::new(RwLock::new(HashMap::new())) }
        }

        fn get_or_insert(&self, key: &str) -> String {
            // 第一步，尝试读 
            {
                // 获取读锁
                let r_lock = self.map.read().unwrap();
                if let Some(v) = r_lock.get(key) {
                    return v.clone();
                }
            }// 读锁在这里释放
            // 如果不在这里释放，下面去拿写锁时，就会发生死锁

            // 第二步，尝试写 write lock 
            let mut w_lock = self.map.write().unwrap();
            // 为什么 double check 在释放读锁，加写锁的中间，可能已经有别的线程插入进来进行操作
            w_lock.entry(key.to_string())
            .or_insert_with(|| {
                println!("Thread {:?} is inserting...", thread::current().id());
                "default".to_string()
            })
            .clone()

        }
    }


    #[test]
    fn test_4() {
        let cache = ThreadSafeCache::new();
        let mut handles = vec![];

        // 模拟  10 个线程去读取插入同一个 key
        for i  in  0..20 {
            let cache_clone = cache.clone();// 增加了引用计数
            let handle = thread::spawn(move || {
                let val = cache_clone.get_or_insert("user_1");
                println!("Thread: {}, Got: {}", i, val);
            });

            handles.push(handle);

        }

        for ele in handles {
            ele.join().unwrap()
        }
    }


    #[derive(Clone)]
    struct RethStypeCache {
        map: Arc<DashMap<String, String>>,
    }

    impl RethStypeCache {
        fn new() -> Self {
            RethStypeCache { map: Arc::new(DashMap::new()) }
        }


        fn get_or_insert(&self, key: &str) -> String {
            self.map
            .entry(key.to_string())
            .or_insert("default".to_string())
            .value()
            .clone()
        }
    }


    #[test]
    fn test_5() {
                let cache = RethStypeCache::new();
        let mut handles = vec![];

        // 模拟  10 个线程去读取插入同一个 key
        for i  in  0..20 {
            let cache_clone = cache.clone();// 增加了引用计数
            let handle = thread::spawn(move || {
                let val = cache_clone.get_or_insert("user_1");
                println!("Thread: {}, Got: {}", i, val);
            });

            handles.push(handle);

        }

        for ele in handles {
            ele.join().unwrap()
        }
    }

}
