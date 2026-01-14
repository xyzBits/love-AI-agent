use std::{fmt::Debug, marker::PhantomData};

/// 要设计一个通用的流水线系统，它能够连接 数据源 (Source) -> 处理器 (Processor) -> 输出端 (Sink)。

/// 核心难点： 整个流水线必须是强类型的。编译器必须保证：

/// Source 吐出的数据类型 A，必须和 Processor 接收的输入类型 A 一致。

/// Processor 吐出的数据类型 B，必须和 Sink 接收的输入类型 B 一致。
///
///

// 1. 数据源: 负责生产数据
// 类似于 Iterator 但它是 async 的
trait Source {
    type Item: Debug; // 关联类型，强制要求生产出来的东西必须能被 Debug 打印

    async fn next(&mut self) -> Option<Self::Item>;
}

// 2. 处理器：负责转换数据
// 输入是 In，输出是关联类型 Out
trait Processor<In> {
    type Out: Debug; // 关联类型，处理后的结果类型

    async fn process(&mut self, input: In) -> Self::Out;
}

// 3. 输出端：负责消费数据
// 类似于 axum 的 handler，它消费 Item
trait Sink<In> {
    async fn send(&mut self, item: In);
}

// ------ 2. 策略标记 PhantomData 用
struct FastMode;
struct SafeMode;

// ----- 3. 流水线结构体 核心难点
// 1. S: Source 必须是人数据源
// 2. P: Processor<S::Item> 必须是个接收端，且它的输入必须等于 P 处理完的 Out

struct Pipeline<S, P, K, Mode>
where
    S: Source,             // S 必须是数据源
    P: Processor<S::Item>, // P 是吃 S 吐出的东西，类型对齐
    K: Sink<P::Out>,       // K 是吃 P 吐出的东西 类型对齐
{
    source: S,
    processor: P,
    sink: K,
    // 使用 PhantomData 占位 Mode，不占用运行时内存
    _marker: PhantomData<Mode>,
}

impl<S, P, K, Mode> Pipeline<S, P, K, Mode>
// impl 这块也需要重复这些约束
where
    S: Source,
    P: Processor<S::Item>,
    K: Sink<P::Out>,
{
    pub fn new(source: S, processor: P, sink: K) -> Self {
        Pipeline {
            source,
            processor,
            sink,
            _marker: PhantomData,
        }
    }

    // 启动引擎
    pub async fn run(&mut self) {
        println!(
            "Pipeline starting in mode: {}",
            std::any::type_name::<Mode>()
        );

        // 循环
        // 1. fetch source
        while let Some(data) = self.source.next().await {
            println!("  [Source] 生产: {:?}", data);

            // 模拟耗时操作，仅在 SafeMode 下，演示 PhantomData 的用途
            if std::any::type_name::<Mode>().contains("SafeMode") {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }

            // 处理数据
            let processed = self.processor.process(data).await;
            println!("  [Processor] 转换: {:?}", processed);

            // 发送到 Sink
            self.sink.send(processed).await;
        }

        println!("✅ Pipeline 任务结束。");
    }
}

// ---- 4. 具体实现
struct NumberSource {
    current: u32,
    max: u32,
}

impl Source for NumberSource {
    type Item = u32;

    async fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.max {
            self.current += 1;

            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            Some(self.current)
        } else {
            None
        }
    }
}

struct ToStringProcessor;

impl Processor<u32> for ToStringProcessor {
    type Out = String;

    async fn process(&mut self, input: u32) -> Self::Out {
        format!("Data: #{}", input)
    }
}

struct ConsoleSink;

impl Sink<String> for ConsoleSink {
    async fn send(&mut self, item: String) {
        println!("   [Sink] 最终输出 -> {}", item);
        println!("   -------------------------");
    }
}

// ==========================================
// 5. Main 函数
// ==========================================

#[tokio::test]
async fn main() {
    // 1. 准备组件
    let source = NumberSource { current: 0, max: 3 };
    let processor = ToStringProcessor;
    let sink = ConsoleSink;

    // 2. 组装流水线
    // 这里的泛型 Mode 我们指定为 FastMode
    // 注意看类型推导：
    // Pipeline 自动推导出 S=NumberSource, P=ToStringProcessor, K=ConsoleSink
    // 我们只需要手动指定 Mode = FastMode
    let mut pipeline = Pipeline::<_, _, _, FastMode>::new(source, processor, sink);

    // 3. 运行
    pipeline.run().await;

    println!("\n--- 换个模式再跑一次 ---\n");

    let source2 = NumberSource { current: 0, max: 2 };
    // 这次我们用 SafeMode，注意 _marker 的作用
    let mut pipeline2 = Pipeline::<_, _, _, SafeMode>::new(source2, ToStringProcessor, ConsoleSink);
    pipeline2.run().await;
}
