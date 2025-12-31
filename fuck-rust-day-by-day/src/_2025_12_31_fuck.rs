use core::{fmt, str};
use std::{fmt::Display, hash::Hash};

//关键点： auto_impl 必须作用于一种 “包装容器”（如引用、指针、智能指针）。
// i32 会被 宏忽略，所以并不报错
#[allow(dead_code)]
#[allow(unused)]
#[allow(unused_must_use)]

pub trait Iterator {
    type Item;

    fn next(&mut self) -> Option<Self::Item>;
}

#[allow(dead_code)]
struct Counter;

#[allow(dead_code)]
impl Iterator for Counter {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

#[allow(dead_code)]
pub trait CacheableItem: Clone + Default + fmt::Debug {
    type Address: AsRef<[u8]> + Clone + fmt::Debug + Eq + Hash;

    fn is_null(&self) -> bool;
}

#[allow(dead_code)]
trait Container<A, B> {
    fn contains(&self, a: A, b: B) -> bool;
}

#[allow(dead_code)]
#[allow(unused_variables)]
fn difference<A, B, C>(container: &C) -> i32
where
    C: Container<A, B>,
{
    0
}

#[allow(dead_code)]
trait NewContainer {
    type A: Display + Clone + fmt::Debug;
    type B;

    fn contains(&self, a: Self::A, b: Self::B) -> bool;
}

#[allow(dead_code)]
#[allow(unused_variables)]
fn new_difference<C: NewContainer>(container: &C) -> i32
where
    C: NewContainer,
{
    0
}

// 如果你想要实现 OutlinePrint 特征，你需要确保你的类型实现了 Display 特征。
#[allow(dead_code)]
trait OutlinePrint: Display {
    fn outline_print(&self) {
        let output = self.to_string();
        let len = output.len();
        println!("{}", "*".repeat(len + 4));
        println!("*{}*", " ".repeat(len + 2));
        println!("* {} *", output);
        println!("*{}*", " ".repeat(len + 2));
        println!("{}", "*".repeat(len + 4));
    }
}

#[allow(dead_code)]
struct Point {
    x: i32,
    y: i32,
}

#[allow(dead_code)]
impl OutlinePrint for Point {
    
}

#[allow(dead_code)]
impl Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Point({}, {})", self.x, self.y)
    }
}

#[test]
fn test_work() {
    let mut counter = Counter;
    let _ = counter.next();

    let mut data = Vec::new();
    data.push(1);
    assert_eq!(data.contains(&1), true);

}
