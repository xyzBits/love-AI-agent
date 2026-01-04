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
