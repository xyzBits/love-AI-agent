/// &'static T
/// 1. 字符串字面量 &'static str 存在代码段，只读数据段
/// 2. static 声明的全局变量 &'static T 存在静态数据段
/// 3. Box::leak &'static T 存在堆上，但永不释放
/// 大概率是某种全局单例或者故意泄露的配置数据
#[cfg(test)]
#[allow(dead_code)]
mod tests_static {
    use std::borrow::Cow;

    use serde::Deserialize;

    // 全局配置
    struct Config {
        debug_model: bool,
        app_name: &'static str,
    }

    // 声明一个全局静态结构体
    static APP_CONFIG: Config = Config {
        debug_model: true,
        app_name: "SuperApp",
    };

    #[test]
    fn test_static_config() {
        // 获取结构体的 static 引用
        let config: &'static Config = &APP_CONFIG;

        if config.debug_model {
            println!("Starting {}", config.app_name);
        }
    }

    // &'static Vec<String> 运行时内存泄露
    #[test]
    fn test_box_leak() {
        // 1.  在运行时动态创建一个 Vec
        let my_vec = vec!["Rust".to_string(), "Java".to_string(), "C++".to_string()];

        // 2. 将其装入 box 移动到堆上
        let boxed_vec = Box::new(my_vec);

        // 3. 显式内存泄露
        // Box::leak 会消费掉box，并返回 个 &'static mut T
        let static_vec: &'static Vec<String> = Box::leak(boxed_vec);

        // 4. 你现在可以把这个引用传递给任何需要 'static 的地方了，比如thread::spawn
        std::thread::spawn(move || {
            // 这里的 static_vec 是引用，但因为它是 static的，所以可以在线程间安全传递
            println!("我们在子线程里访问：{:?}", static_vec);
        })
        .join()
        .unwrap();
    } // 注意，这块内存直到程序结束前永远不会被回收

    fn sanitize<'a>(input: &'a str) -> Cow<'a, str> {
        if input.contains("死") {
            // 发现敏感词，必须修改，不得不clone
            let new_string = input.replace("死", "*");
            Cow::Owned(new_string)
        } else {
            // 字符串很干净，无需修改
            // 直接把传进来的引用包一下返回，没有任何内存分配
            Cow::Borrowed(input)
        }
    }

    #[test]
    fn test_cow() {
        let s1 = "Hello Rust";
        let c1 = sanitize(s1);
        match c1 {
            Cow::Borrowed(_) => println!("是借用的，省内存了"),
            Cow::Owned(_) => println!("是拥有的，分配内存了"),
        }

        let s2 = "去死吧bug";
        let c2 = sanitize(s2);
        match c2 {
            Cow::Borrowed(_) => println!("是借用的，省内存了"),
            Cow::Owned(_) => println!("是拥有的，分配内存了"),
        }
    }

    #[derive(Deserialize, Debug)]
    struct User<'a> {
        // 使用 Cow 如果有转义字符就分配，没有就引用
        #[serde(borrow)] // 告诉 serde 尽可能的去借用数据
        name: Cow<'a, str>,
    }

    #[test]
    fn test_cow_user() {
        // 场景 1: 没有转义字符 -> Borrowed
        let json_1 = r#"{ "name": "zhangsan" }"#;
        let user_1: User = serde_json::from_str(json_1).unwrap();

        match user_1.name {
            Cow::Borrowed(s) => println!("场景1: 借用了! 原始数据是: {}", s),
            Cow::Owned(_) => println!("场景1: 分配内存了!"),
        }

        // 场景 2: 含有转义字符 (\n) -> Owned
        // 注意：原始 JSON 字符串里包含字面量的 \ 和 n
        let json_2 = r#"{ "name": "zhang\nsan" }"#;
        let user_2: User = serde_json::from_str(json_2).unwrap();

        match user_2.name {
            Cow::Borrowed(_) => println!("场景2: 借用了!"),
            Cow::Owned(s) => println!("场景2: 分配内存了! 因为要处理转义: {:?}", s),
        }
    }
}

/// $name: ident 告诉编译器，这块匹配什么，常用的有
///     ident 标识符，变量名，函数名，如x foo
///     expr 表达式，有返回值的代码块，如 1 + 1 func() x
///     ty 类型 如 i32 Vec<String>
///     stmt 语句 let x = 1;
///     block 代码块，用花括号包起搂的， 如 {}
#[allow(dead_code)]
#[allow(unused_macros)]
#[cfg(test)]
mod macro_test {
    // 定义宏的名字叫 say_hello

    macro_rules! say_hello {
        // 模式匹配 $name: ident
        // $name 是一个变量名，用来捕获输入
        // :ident 是指示符，告诉编译器要匹配一个 标识 符 Identifier
        ($name: ident) => {
            // 宏展开的代码
            fn $name() {
                println!("Hello, I am {}", stringify!($name));
            }
        };
    }

    #[test]
    fn test_say_hello() {
        // 调用宏
        // 编译器看到这行，会把它替换成上面的 fn rust() {}
        say_hello!(rust);

        // 现在我们可以调用由这个宏生成的函数了
        rust();
    }

    macro_rules! my_vec {
        // 模式匹配解析
        // $() 创建一个捕获组，类似正则的 group
        // $input_expr: expr  捕获一个表达式，赋值给变量 $x
        // , 元素之间必须用 , 分开
        // * 重复次数，0次或者多次，类似正则的 *
        ($($input_expr: expr), *) => {
            {
                // 宏展开的具体逻辑
                let mut temp_vec = Vec::new();

                // $(...)* 这里是 展开重复
                // 宏引擎会根据捕获到的 $x 的数量 ，把这行代码重复 N 次
                $(
                    temp_vec.push($input_expr);
                )*

                temp_vec // 返回 vec，因为这是一个表达式块
            }
        };

        ($input_expr: expr ; $count: expr) => {
            {
                let mut temp_vec = Vec::with_capacity($count);

                for _ in 0..$count {
                    temp_vec.push($input_expr);
                }

                temp_vec

            }
        }
    }

    #[test]
    fn test_my_vec() {
        let num = 4;
        let v = my_vec![10, 20, 30, num - 1];
        println!("v: {:?}", v);

        let v1 = my_vec! {num as f32 * 1.1; 2* 2};
        println!("v1: {:?}", v1);

        // Vec::with_capacity(capacity)
    }
}
