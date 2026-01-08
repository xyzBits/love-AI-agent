// build.rs
use std::io::Result;

fn main() -> Result<()> {
    // 告诉 cargo：如果 proto 文件变了，就重新编译
    println!("cargo:rerun-if-changed=proto/market_data.proto");

    // 编译 proto 文件
    prost_build::compile_protos(&["proto/market_data.proto"], &["proto/"])?;
    Ok(())
}
