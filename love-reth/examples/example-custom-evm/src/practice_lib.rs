/// Once 一次 + Lock 锁定
/// 只能写入一次，但可以被读取无数次的容器，而且是线程安全的
/// 懒加载，里面的值只能在第一次被使用时才会初始化
/// 单例模式，无论有多少个线程去初始化它，它保证只会被初始化一次
/// 全局变量神器，解决静态变量难以初始化复杂对象的问题
#[cfg(test)]
#[allow(dead_code)]
mod once_lock_tests {
    use std::sync::OnceLock;

    use std::collections::HashMap;

    use reth_ethereum::pool::NewBlobSidecar;

    // 1. 全局配置、单例
    // 定义一个全局的 onceLock，它是空的，不占什么资源
    static CONFIG: OnceLock<HashMap<String, String>> = OnceLock::new();

    // 模拟 从文件读取配置
    fn load_config() -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("host".to_string(), "localhost".to_string());
        map.insert("port".to_string(), "8080".to_string());

        map
    }

    fn get_config() -> &'static HashMap<String, String> {
        // get_or_init 核心方法
        // 如果 config 里有值，直接返回引用的引用
        // 如果 config 为空，执行闭包里的 load_config 存进去，然后返回引用
        CONFIG.get_or_init(|| load_config())
    }
    #[test]
    fn test_1() {
        // 第一次调用：会触发初始化
        let config1 = get_config();
        println!("Host: {}", config1.get("host").unwrap());

        // 第二次调用，直接返回内存里现成的值，不会打印正在加载
        let config2 = get_config();
        println!("Port: {}", config2.get("port").unwrap());

        // 证明是同一份数据
        assert_eq!(config1 as *const _, config2 as *const _);
    }

    // 有时候结构体里的某个字段计算非常昂贵，你希望只有用户真的访问时才计算
    struct BigData {
        raw_id: u32,
        // 这个字段计算很慢，用 OnceLock 包起来
        expensive_value: OnceLock<String>,
    }

    impl BigData {
        fn new(id: u32) -> Self {
            Self {
                raw_id: id,
                expensive_value: OnceLock::new(), // 此时这里是空的
            }
        }

        fn value(&self) -> &String {
            // 只有调用 value() 方法时，才真正去计算字符串
            self.expensive_value.get_or_init(|| {
                println!("------- 计算 expensive value -----");
                format!("Result-{}", self.raw_id * 100)
            })
        }
    }

    #[test]
    fn test_2() {
        let data = BigData::new(5);
        println!("结构体刚创建完毕，但昂贵数据还未计算");

        println!("第一次访问：{}", data.value()); // 触发计算
        println!("第二次访问：{}", data.value()); // 直接返回
    }

    #[test]
    fn test_static() {
        let a = 100;
        println!("a = {a}");

        // 生命周期：永生，程序启动时就在那，程序关闭时才消失
        // 全局静态数据区
        // 无论这个函数调用一次或者100万次，大家看到的都是同一块内存地址，同一份数据
        // 写在函数内部或者外部，都是一样的，都是全局永生的，区别在于可见性
        static DATA: &str = "hello world";

        println!("data = {DATA}");
    }


    #[test]
    fn test_bytes() {
        // 假设这是从以太坊发过来的 8 个字节数据
        let bytes: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 1];

        // 转换 
        let num = u64::from_be_bytes(bytes);

        println!("num ={}", num);


        let new_bytes = num.to_be_bytes();
        println!("new bytes ={:?}", new_bytes);

    }
}
