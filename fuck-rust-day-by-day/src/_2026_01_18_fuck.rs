use alloy::primitives::{Address, B256, Signature, keccak256};
use alloy::signers::Signer; // 签名 Trait (包含 sign_message 方法)
use alloy::signers::local::PrivateKeySigner; // 本地私钥钱包 // 基础类型

#[test]
fn test_hash_256() {
    let input = b"Hello, World!";

    let hash: B256 = keccak256(input);

    println!("input: {:?}", String::from_utf8_lossy(input));
    println!("hash: {:?}", hash);

    println!("Hex: {}", hex::encode(hash));
}

#[test]
fn test_address() {
    // 1. 随机生成一个私钥 (对应创建一个新钱包)
    let signer = PrivateKeySigner::random();

    // 2. 获取地址
    // Reth 中所有的地址都是 alloy_primitives::Address 类型
    let address = signer.address();

    println!("✅ 新钱包创建成功");
    println!("地址: {}", address); // 自动格式化为 checksum 格式 (大小写混合)

    // 3. 导出私钥 (用于备份，通常是 32 字节的 hex)
    // to_bytes() 返回的是 GenericArray，我们需要转 hex
    let private_key_hex = hex::encode(signer.to_bytes());
    println!("私钥: 0x{}", private_key_hex);
}

#[tokio::test]
async fn test_signer() -> eyre::Result<()> {
    // ==========================================
    // 1. 初始化钱包
    // ==========================================
    let signer = PrivateKeySigner::random();
    let my_address = signer.address();

    println!("✅ 钱包已创建");
    println!("地址: {}", my_address);

    // ==========================================
    // 2. 签名 (Sign)
    // ==========================================
    let message = b"Login to Reth App";

    // 🔥 修复点：
    // 1. 这里的 signature 类型会自动推导为 alloy::primitives::Signature
    // 2. sign_message 是异步的，需要 await
    let signature = signer.sign_message(message).await?;

    println!("--------------------");
    println!("原始消息: {:?}", String::from_utf8_lossy(message));
    // Signature 实现了 Display，可以直接打印出 hex 格式
    println!("签名结果: {:?}", signature);

    // ==========================================
    // 3. 验签 (Verify / Recover)
    // ==========================================
    // 从 [签名] + [消息] 中恢复出 [签名者的地址]
    let recovered_address = signature.recover_address_from_msg(message)?;

    println!("--------------------");
    println!("声称的地址: {}", my_address);
    println!("恢复的地址: {}", recovered_address);

    if recovered_address == my_address {
        println!("✅ 验证通过：确实是本人操作");
    } else {
        println!("❌ 验证失败");
    }

    Ok(())
}

#[test]
fn test_hex_encode() {
    // 1. 原始地址 (机器眼中的样子：20个字节的数组)
    // 假设这是地址 0x1122...
    let original_bytes: Vec<u8> = vec![0x11, 0x22, 0x33, 0x44, 0x55];

    // 2. 编码 (Encode) -> 变成字符串
    // 这一步是为了展示给用户看
    let encoded_string = hex::encode(&original_bytes);
    println!("编码后: {}", encoded_string); // 输出 "1122334455"

    // 3. 解码 (Decode) -> 变回字节
    // 这一步是把用户输入的字符串变回数据，以便程序处理
    let restored_bytes = hex::decode(encoded_string).unwrap();
    println!("复原后: {:?}", restored_bytes); // 输出 [17, 34, 51, 68, 85] (即 0x11, 0x22...)

    // 4. 验证
    assert_eq!(original_bytes, restored_bytes);
    println!("✅ 完美复原，字节一模一样！");
}

use base64::prelude::*; // 引入 Base64 的常用引擎
// use anyhow::Result;

#[test]
fn test_base_58_64() -> anyhow::Result<()> {
    // 原始数据 (字节数组)
    let original_msg = "Hello World";
    let original_bytes = original_msg.as_bytes();

    println!("📄 原始字符串: {}", original_msg);
    println!("💾 原始字节:   {:?}", original_bytes);
    println!("--------------------------------------------------");

    // ==========================================
    // 1. Base64 示例
    // ==========================================
    // Encode: 字节 -> String
    let b64_encoded = BASE64_STANDARD.encode(original_bytes);
    println!("🧮 Base64 编码后: {}", b64_encoded);

    // Decode: String -> 字节
    let b64_decoded_bytes = BASE64_STANDARD.decode(&b64_encoded)?;
    let b64_decoded_str = String::from_utf8(b64_decoded_bytes)?;
    println!("↩️  Base64 解码回: {}", b64_decoded_str);

    println!("--------------------------------------------------");

    // ==========================================
    // 2. Base58 示例 (比特币/Solana 风格)
    // ==========================================
    // Encode: 字节 -> String
    let b58_encoded = bs58::encode(original_bytes).into_string();
    println!("₿  Base58 编码后: {}", b58_encoded);

    // Decode: String -> 字节
    let b58_decoded_bytes = bs58::decode(&b58_encoded).into_vec()?;
    let b58_decoded_str = String::from_utf8(b58_decoded_bytes)?;
    println!("↩️  Base58 解码回: {}", b58_decoded_str);

    Ok(())
}
