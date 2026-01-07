#[allow(dead_code)]
#[cfg(test)]
mod test_dyn_trait {

    trait Heater {
        fn heat(&self);
    }

    struct ElectricHeater {
        voltage: u64,
    }

    struct GasHeater {
        gas_type: u8,
    }

    impl Heater for ElectricHeater {
        fn heat(&self) {}
    }

    impl Heater for GasHeater {
        fn heat(&self) {}
    }

    /// let object: Box<dyn Heater>
    /// 栈上的胖指针包含
    /// data pointer ptr 指向堆上的 ElectricHeater 实例数据
    /// vtable pointer vptr 指向静态内存区的一个表格
    /// vtable 是编译阶段就生成好的，每个具体类型对应一份
    /// ElectricHeater 有一张表
    /// GasHeater 有另一张表
    /// 表中记录的有
    ///     drop 函数指针
    ///     size alignment 大小和内存，要分配多少内存，也被记录在 vtable 中
    ///     方法指针 就是heat 方法，有多个方法，会依次排列
    ///
    /// rust 做法，具体类型保持纯净，只有转为 dyn heater 时，才会在栈上的引用里额外增加 8 字节来存放 vptr
    #[test]
    #[allow(unused_variables)]
    fn test_box_dyn() {
        let object: Box<dyn Heater> = Box::new(ElectricHeater { voltage: 20 });
    }

    struct NamedHeater {
        voltage: u64,
        name: String,
    }

    /// 结构体本体在栈上
    /// 栈上，变量 h 作为一个整体，完全住在栈上，
    /// voltage 8字节
    /// name 24 字节 ptr + cap + len
    /// 堆上只有字符串 Hi的内容，存放在堆上
    /// 除非显式的调用 Box Vec Rc 等容器，否则 结构体实例本身，也就是那堆字段的集合，永远是在栈上的
    #[test]
    #[allow(unused_variables)]
    fn test_struct_mem_layout() {
        let h = NamedHeater {
            voltage: 20,
            name: String::from("Sony"),
        };
    }

    // 传入 &String 是如何转为 &str 的
    fn print_len(s: &str) {
        println!("Length: {}", s.len());
    }

    #[test]
    #[allow(unused_variables)]
    fn test_print_len() {
        let b = Box::new(String::from("World"));

        let star_b = *b;
        // let star_star_b = *star_b;
        let and_star_star_b = &(*star_b);
        print_len(and_star_star_b);

        let s = String::from("Hello");

        let b = Box::new(s);
        print_len(b.as_str());

        let s = String::from("Rust");

        // s.len 其实是 fn len(&self) -> usize
        // 也就是 &String 不是String，但是String 实现了 Deref<Target=str>
        println!("s.len = {}", s.len());

        let data = String::from("Hello world   ");
        let trim_data = data.trim();
        let _ = data.capacity();

        // let str_data: Box<dyn str> = Box::new("hello");
    }

    use std::mem::size_of;
    #[test]
    fn test_smart_pointer_size() {
        // 胖指针：带长度的引用
        println!("&str size: {}", size_of::<&str>()); // 输出 16

        // 瘦指针：指向结构体的普通引用
        println!("&String size: {}", size_of::<&String>()); // 输出 8

        // String 结构体本身 (ptr + len + cap)
        println!("String size: {}", size_of::<String>()); // 输出 24
    }
}

#[allow(dead_code)]
#[allow(unused_variables)]
#[cfg(test)]
mod test_type_state_pattern {

    // 定义状态 zero sized types 空结构体，在内存中占用 0 字节

    use std::marker::PhantomData;

    struct Grounded;
    struct Fueled;
    struct Launched;

    // Rocket 拥有一个泛型参数 state
    struct Rocket<State> {
        fuel: u64,

        // 需要用 PhantomData 告诉编译器
        state: PhantomData<State>,
    }

    // 状态流转
    impl Rocket<Grounded> {
        pub fn new() -> Self {
            Rocket {
                fuel: 0,
                state: PhantomData,
            }
        }

        pub fn fuel(self, amount: u64) -> Rocket<Fueled> {
            println!("Fueling...");
            Rocket {
                fuel: amount,
                state: PhantomData,
            }
        }
    }

    impl Rocket<Fueled> {
        pub fn launch(self) -> Rocket<Launched> {
            println!("Lifoff with fuel: {}", self.fuel);
            Rocket {
                fuel: self.fuel,
                state: PhantomData,
            }
        }
    }

    #[test]
    fn test_rocket() {
        let r = Rocket::new(); // r 是 Rocket<Grounded>

        // r.launch();
        // ❌ 编译报错！
        // no method named `launch` found for struct `Rocket<Grounded>`
        // 编译器直接告诉你：没加油发什么射？

        let r_fueled = r.fuel(100); // 状态转移：Grounded -> Fueled
        // r.fuel(10);
        // ❌ 编译报错！Use of moved value: `r`
        // 旧状态的火箭已经被“消耗”掉了，你不能对同一个火箭加两次油！

        let r_launched = r_fueled.launch(); // 状态转移：Fueled -> Launched

        // r_launched.launch();
        // ❌ 编译报错！已发射的火箭不能再发射。
    }

    // 1. 定义状态
    struct NoUrl;
    struct UrlSet;
    struct ReadyToSend; // 包含了 URL 和 Method

    // 2. 定义结构体
    struct RequestBuilder<State> {
        url: String,
        method: String,
        headers: Vec<String>,
        state: PhantomData<State>,
    }

    // 3. 初始状态，什么都没有
    impl RequestBuilder<NoUrl> {
        fn new() -> Self {
            RequestBuilder {
                url: String::new(),
                method: String::new(),
                headers: Vec::new(),
                state: PhantomData,
            }
        }

        // 第一步：设置 URL
        // 状态变迁：NoUrl --> UrlSet
        // 只有在 NoUrl下才能调用 url()
        fn url(self, u: &str) -> RequestBuilder<UrlSet> {
            RequestBuilder {
                url: u.to_string(),
                method: self.method,   // 虽然经时是空，但是为了通用性保留搬运
                headers: self.headers, // 搬运旧数据
                state: PhantomData,
            }
        }
    }

    // 4. UrlSet 状态：已经有了 URL，缺 method
    impl RequestBuilder<UrlSet> {
        // 第一步：设置 URL
        fn method(self, m: &str) -> RequestBuilder<ReadyToSend> {
            RequestBuilder {
                url: self.url,
                method: m.to_string(),
                headers: self.headers, // 设置新值
                state: PhantomData,
            }
        }
    }

    // 5. ReadyToSend 状态，万事具备
    impl RequestBuilder<ReadyToSend> {
        // 只有在这个状态下，才能发送
        pub fn send(self) {
            println!(
                "🚀 Sending request to {} with method {}",
                self.url, self.method
            );
            println!("Headers: {:?}", self.headers);
        }

        // 允许在这个阶段追加 header 返回自身状态
        pub fn header(mut self, h: &str) -> Self {
            self.headers.push(h.to_string());
            self
        }
    }

    #[test]
    fn test_url_builder() {
        // 链式调用，非常丝滑
        RequestBuilder::new()
            .url("https://rust-lang.org") // 变身 UrlSet
            .method("GET") // 变身 ReadyToSend
            .header("User-Agent: Rust") // 保持 ReadyToSend
            .send(); // 发射！

        // 下面这行代码连编译都过不去，因为 new() 返回 NoUrl，NoUrl 没有 send() 方法
        // RequestBuilder::new().send();
    }
}
