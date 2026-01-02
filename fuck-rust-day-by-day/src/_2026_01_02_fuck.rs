use core::panic;
#[allow(dead_code)]
use std::str;
use std::{collections::HashMap, sync::Arc, time::Duration};

use tokio::sync::RwLock;

#[allow(dead_code)]
#[async_trait::async_trait]
trait DataFetcher {
    async fn fetch_url(&self, url: &str) -> String;
}
#[allow(dead_code)]
struct MyFetcher;

#[allow(dead_code)]
#[async_trait::async_trait]
impl DataFetcher for MyFetcher {
    async fn fetch_url(&self, _url: &str) -> String {
        "ok".to_string()
    }
}

#[allow(dead_code)]
struct FileFetcher;

#[allow(dead_code)]
struct NetworkFetcher;

#[async_trait::async_trait]
impl DataFetcher for FileFetcher {
    async fn fetch_url(&self, _url: &str) -> String {
        "Content from file".to_string()
    }
}

#[async_trait::async_trait]
impl DataFetcher for NetworkFetcher {
    async fn fetch_url(&self, url: &str) -> String {
        tokio::time::sleep(Duration::from_millis(500)).await;
        format!("Content from network for {}", url)
    }
}

#[tokio::test]
async fn test_work() {
    let f = MyFetcher;

    // 不加 async_trait 宏，无法动态分发
    let _obj: Box<dyn DataFetcher> = Box::new(f);

    let file_fetcher = FileFetcher;
    let network_fetcher = NetworkFetcher;

    println!("{}", file_fetcher.fetch_url("test.txt").await);
    println!("{}", network_fetcher.fetch_url("http://google.com").await);
}

#[allow(dead_code)]
#[async_trait::async_trait]
trait Storage: Send + Sync {
    async fn set(&self, key: String, value: String);
    async fn get(&self, key: &str) -> Option<String>;
}

#[allow(dead_code)]
struct MemStore {
    data: Arc<RwLock<HashMap<String, String>>>,
}

#[allow(dead_code)]
impl MemStore {
    fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[allow(dead_code)]
#[async_trait::async_trait]
impl Storage for MemStore {
    async fn get(&self, key: &str) -> Option<String> {
        let guard = self.data.read().await;
        guard.get(key).cloned()
    }

    async fn set(&self, key: String, value: String) {
        let mut guard = self.data.write().await;
        guard.insert(key, value);
    }
}

// 模拟应用层，并不关心底层， memStore 还是 redisStore
#[allow(dead_code)]
struct Service {
    store: Box<dyn Storage>,
}

#[allow(dead_code)]
impl Service {
    pub fn new(store: Box<dyn Storage>) -> Self {
        Self { store }
    }

    async fn run_logic(&self) {
        self.store
            .set("user_1".to_string(), "Alice".to_string())
            .await;
        let val = self.store.get("user_1").await;
        println!("Got user: {:?}", val);
    }
}

#[tokio::test]
async fn test_dyn() {
    let db = MemStore::new();

    // Box::new(db) 将具体类型转为 Trait object 指针
    let service = Service::new(Box::new(db));

    service.run_logic().await;
}

// 代理结构体
#[allow(dead_code)]
struct LoggingFetcher<T> {
    inner: T,
}

#[async_trait::async_trait]
#[allow(dead_code)]
impl<T> DataFetcher for LoggingFetcher<T>
where
    T: DataFetcher + Send + Sync, // async_trait 生成的代码需要这些约束
{
    async fn fetch_url(&self, url: &str) -> String {
        println!("[Log] start fetching: {}", url);
        let res = self.inner.fetch_url(url).await;
        println!("[Log] finished fetching: {}", url);
        res
    }
}

#[tokio::test]
async fn test_wapper() {
    let origin = NetworkFetcher;

    let layer1 = LoggingFetcher { inner: origin };

    let result = layer1.fetch_url("http://rust-lang.org").await;
    println!("Final Result: {}", result);
}

/// Box<dyn Trait> Rust 实现多态的手段，可以在同一个容器内 Vec 中存储不同类型的对象，只要它们实现了同一个接口
/// 为什么需要 Box，因为 dyn Trait 的长度不确定
/// 不知道 dyn Animal 占用多少内存，一只猫占用 10 字节，还是一头大象占用 1000 字节
/// Rust 是强类型语言，栈上的变量必须知道大小
/// 解决办法，用 Box 把具体的数据扔到堆上，然后在栈上只留下一个指针，指针的大小是固定的
///
/// 为什么叫 dyn
///     static dispatch 静态分发 fn foo<T: Trait>(t: T) 编译器在编译时会为每种类型生成一份代码，速度快，但代码膨胀
///     dynamic dispatch Box<dyn Trait> 编译器只生成一份代码，运行时，程序 会查一个虚函数表来决定调用哪个具体的方法，这就叫 dyn
#[allow(dead_code)]
trait Widge {
    fn draw(&self);
}

#[allow(dead_code)]
struct Button {
    label: String,
}

#[allow(dead_code)]
struct Select {
    width: u32,
}

#[allow(dead_code)]
impl Widge for Button {
    fn draw(&self) {
        println!("Drawing Button: [{}]", self.label);
    }
}

impl Widge for Select {
    fn draw(&self) {
        println!("Drawing Select: width={}px v", self.width);
    }
}

#[test]
fn test_dyn_trait() {
    let mut components: Vec<Box<dyn Widge>> = vec![];

    components.push(Box::new(Button {
        label: "Submit".to_string(),
    }));

    components.push(Box::new(Select { width: 100 }));

    // 运行时金矿，这里并不关心具体类型是 Button 还是 Select
    for ele in components {
        ele.draw();
    }
}

#[allow(dead_code)]
trait Payment {
    fn pay(&self, amount: f64);
}

#[allow(dead_code)]
struct AliPay;
#[allow(dead_code)]
struct WeChatPay;

#[allow(dead_code)]
impl Payment for AliPay {
    fn pay(&self, amount: f64) {
        println!("Paid ${:.2} via AliPay", amount);
    }
}

#[allow(dead_code)]
impl Payment for WeChatPay {
    fn pay(&self, amount: f64) {
        println!("Paid ${:.2} via WeChatPay", amount);
    }
}

// 返回类型是 Box<dyn Payment>
// 这意味着函数返回的具体类型是运行时确定的
#[allow(dead_code)]
fn create_payment(method: &str) -> Box<dyn Payment> {
    match method {
        "alipay" => Box::new(AliPay),
        "wechat" => Box::new(WeChatPay),
        _ => panic!("Unknown payment method"),
    }
}

#[test]
fn test_dyn_return() {
    let ali_pay = create_payment("alipay");
    ali_pay.pay(100.0);

    let wechat_pay = create_payment("wechat");
    wechat_pay.pay(40.3);
}

#[allow(dead_code)]
trait Weapon {
    fn attack(&self);
}

#[allow(dead_code)]
struct Sword;
#[allow(dead_code)]
struct Bow;

#[allow(dead_code)]
impl Weapon for Sword {
    fn attack(&self) {
        println!("Swish! (Sword slash)");
    }
}

#[allow(dead_code)]
impl Weapon for Bow {
    fn attack(&self) {
        print!("Tang! (Arrow shot)");
    }
}

#[allow(dead_code)]
struct Hero {
    // 英雄不知道具体是使用武器，只要能 attack 就行
    weapon: Box<dyn Weapon>,
}

#[allow(dead_code)]
impl Hero {
    fn new(weapon: Box<dyn Weapon>) -> Self {
        Self { weapon }
    }

    fn fignt(&self) {
        self.weapon.attack();
    }
}

#[test]
fn test_composition_box() {
    // 初始化拿 sword 的英雄
    let mut hero = Hero::new(Box::new(Sword));
    hero.fignt();

    println!("Hero is switching weapon...");

    // 运行时动态切换组件
    // 这在游戏开发或插件系统中非常常用
    hero.weapon = Box::new(Bow);
    hero.fignt();
}
