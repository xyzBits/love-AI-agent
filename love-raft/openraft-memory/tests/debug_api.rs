// 定义特征：持有身份证的人
trait _Holder {
    // 这里就是关联类型！
    // 它的意思是：凡是实现这个特征的人，必须告诉我你的 ID 是什么类型。
    // 我们先起个代号叫 IDType，具体是什么，实现的时候再定。
    type IDType;

    // 这个函数的返回值，就是上面那个代号
    fn show_id(&self) -> Self::IDType;
}

struct _Chinese;

impl _Holder for _Chinese {
    // 关联 ID 的类型是和人的类型绑定在一起的
    type IDType = String;

    fn show_id(&self) -> Self::IDType {
        "12345444".to_string()
    }
}

struct _American;

impl _Holder for _American {
    type IDType = u64;
    fn show_id(&self) -> Self::IDType {
        1234566
    }
}

#[test]
fn test() {}
