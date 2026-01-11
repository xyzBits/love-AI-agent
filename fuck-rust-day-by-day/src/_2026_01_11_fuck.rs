use std::fmt::Debug;

trait Bird {
    type Food;

    fn eat(&self, food: Self::Food);
}

struct Eagle;
struct Rabbit;

#[allow(unused_variables)]
impl Bird for Eagle {
    type Food = Rabbit;

    fn eat(&self, food: Self::Food) {
        println!("老鹰吃兔子");
    }
}

trait Transformer<T> {
    fn transform(input: T) -> Self;
}

struct MyNumber(i32);

impl Transformer<i32> for MyNumber {
    fn transform(input: i32) -> Self {
        MyNumber(input)
    }
}

impl Transformer<String> for MyNumber {
    fn transform(input: String) -> Self {
        MyNumber(input.parse().unwrap_or(0))
    }
}

trait DataSource {
    type Item;

    fn fetch(&self) -> Self::Item;
}

struct StringSource(String);

impl DataSource for StringSource {
    type Item = String;

    fn fetch(&self) -> Self::Item {
        self.0.clone()
    }
}

fn print_data<S>(source: S)
where
    S: DataSource<Item: Debug>,
    // S::Item: Debug,
{
    let data = source.fetch();
    println!("Data is {:?}", data);
}

#[test]
fn test_1() {
    let source = StringSource("Hello World".to_string());
    print_data(source);
}

struct MyBuffer;

impl MyBuffer {
    fn write(&mut self, data: &str) {
        println!("正在写入底层 Buffer: {}", data);
    }
}

struct MyFormatter<'a> {
    buf: &'a mut MyBuffer,
}

trait MyDisplay {
    // 这里的 _ 就是在说，
    // fmt 函数在执行期间，f 借用的那个 MyBuffer 必须一直活着
    fn fmt(&self, f: &mut MyFormatter<'_>);
}

struct User(i32);

impl MyDisplay for User {
    fn fmt(&self, f: &mut MyFormatter<'_>) {
        f.buf.write("User 100");
    }
}

#[test]
fn test_format() {
    let mut real_buffer = MyBuffer;

    {
        let mut f = MyFormatter {
            buf: &mut real_buffer,
        };

        let user = User(100);
        user.fmt(&mut f);
    }
}

#[test]
fn test_format_1() {
    let user = User(100);

    let f_reference: Option<MyFormatter> = None;
    {
        let mut temp_buffer = MyBuffer;

        let mut f = MyFormatter {
            buf: &mut temp_buffer,
        };

        // f_reference = Some(f);

        user.fmt(&mut f);
    }

    if let Some(mut f) = f_reference {
        user.fmt(&mut f);
    }
}
