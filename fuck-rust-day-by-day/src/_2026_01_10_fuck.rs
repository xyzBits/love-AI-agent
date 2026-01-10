use std::fmt::Debug;

use serde::{Deserialize, Serialize};

// 它要求，T 必须能处理任何时长的数据  for<'de>
fn static_parser<T>()
where
    T: for<'de> Deserialize<'de>,
{
    println!("恭喜！你的类型足够强壮，通过了检查！");
}

// 内部使用 String，数据是拷贝的，跟输入源解耦了
#[derive(Deserialize)]
struct OwnedUser {
    name: String,
}

// 内部使用 &'a str 它的命是跟输入源绑定的
// 它并没有实现 for<'de> 它只实现了 for specific 'a
#[derive(Deserialize)]
struct BorrowedUser<'a> {
    name: &'a str,
}

#[test]
fn test_life_time() {
    static_parser::<OwnedUser>();

    // static_parser::<BorrowedUser>();

    let json_source = String::from("Rust");
    {
        let user = BorrowedUser { name: &json_source };

        println!("User name: {}", user.name);
    }
}

#[test]
#[allow(unused_variables)]
#[allow(unused_assignments)]
fn test_life_time_2() {
    let user_outside;
    {
        let inner_name = String::from("Rust");

        let user_inside = BorrowedUser { name: &inner_name };

        user_outside = user_inside;
    }

    // println!("User: {:?}", user_outside.name);
}

#[test]
fn test_life_time_3() {
    let json_data = String::from(r#"{ "name": "DeepSeek"}"#);

    let user: BorrowedUser = serde_json::from_str(&json_data).unwrap();

    println!("User: {}", user.name);

    drop(json_data);

    // println!("User: {}", user.name);
}

pub trait Connector
where
    Self: Clone + Default + Debug + for<'de> Deserialize<'de> + Serialize + Sized,
{
    type Channel: AsRef<str>;

    type Market: AsRef<str>;
}

// struct Binance {
//     // server: PhantomData<Server>,
// }

// impl Connector for Binance {
//     type Channel = &'static str;
//     type Market = String;
// }

#[derive(Debug)]
struct WsMessage(String);

fn bybit_ping() -> WsMessage {
    WsMessage(r#"{"op": "pring"}"#.to_string())
}

#[derive(Debug)]
#[allow(private_interfaces)]
pub struct PingInterval<T> {
    pub interval: tokio::time::Interval,
    pub ping: fn() -> T, // 函数指针，比 闭包更小
}

#[tokio::test]
#[allow(unused_variables)]
async fn test_ping() {
    let p = PingInterval {
        interval: tokio::time::interval(std::time::Duration::from_secs(30)),
        ping: bybit_ping,
    };

    // (p.ping)();
}

trait Container {
    type Item;

    fn add(&mut self, item: Self::Item);
    fn get(&self) -> Option<&Self::Item>;
}

struct BoxContainer {
    items: Vec<u32>,
}

impl Container for BoxContainer {
    type Item = u32;

    fn add(&mut self, item: Self::Item) {
        self.items.push(item);
    }

    fn get(&self) -> Option<&Self::Item> {
        self.items.last()
    }
}

struct TextContainer {
    items: Vec<String>,
}

impl Container for TextContainer {
    type Item = String;

    fn add(&mut self, item: Self::Item) {
        self.items.push(item);
    }

    fn get(&self) -> Option<&Self::Item> {
        self.items.last()
    }
}

struct GeneralContainer<T> {
    items: Vec<T>,
}

impl<T> Container for GeneralContainer<T> {
    type Item = T;

    fn add(&mut self, item: Self::Item) {
        self.items.push(item);
    }

    fn get(&self) -> Option<&Self::Item> {
        self.items.last()
    }
}

#[test]
fn test_associated_type() {
    let mut b = BoxContainer { items: vec![] };

    b.add(100);
    println!("last: {:?}", b.get());

    let mut con = GeneralContainer { items: Vec::new() };

    con.add("hello".to_string());
    println!("last: {:?}", con.get());
}
