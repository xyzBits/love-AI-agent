
//关键点： auto_impl 必须作用于一种 “包装容器”（如引用、指针、智能指针）。
// i32 会被 宏忽略，所以并不报错
#[allow(dead_code)]
#[allow(unused)]
#[allow(unused_must_use)]



// 把 :: 理解为文件系统里的根目录符号 /
//auto_impl::auto_impl (相对路径)： 编译器会从当前位置开始找。它会先看当前模块里有没有叫 auto_impl 的东西，如果没有，再去外部找。
//
// ::auto_impl::auto_impl (绝对路径)： 编译器跳过当前模块的所有干扰，直接去**全局外部包（Crate Root）**里找。

#[::auto_impl::auto_impl(&, Box, i32)]
trait Foo {
    fn foo(&self);
}

impl Foo for i64 {
    fn foo(&self) {}
}

impl Foo for i32 {
    fn foo(&self) {}
}

#[allow(dead_code)]
fn requires_foo(_: impl Foo) {}

#[test]
fn test_auto_impl() {
    requires_foo(0_i32);
    requires_foo(std::i64::MAX);
    requires_foo(&0_i32);
    requires_foo(Box::new(0_i32));
}
