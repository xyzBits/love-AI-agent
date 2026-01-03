// #[cfg(test)]
// mod tests {
//     use bytes::BytesMut;
//     use reth_codecs::{Compact, main_codec};

//     // 使用 main_codec 宏，它会自动为我们实现 Compress 和 Decompress
//     // 类似于 derive(RlpEncodable)
//     #[main_codec]
//     #[derive(Debug, PartialEq, Clone)]
//     struct AccountInfo {
//         nonce: u64,
//         balance: u64,
//         // Option 在 Compact 中优化效果最好
//         code_hash: Option<u64>,
//     }

//     #[test]
//     fn practice_compact_flags() {
//         println!("--- 练习 1: Compact 的位图压缩 ---");

//         // 场景 1: 这是一个只有 nonce，没有 hash 的账户
//         let acc = AccountInfo {
//             nonce: 1,
//             balance: 0,
//             code_hash: None, // 这个字段完全不需要存
//         };

//         // 编码 (to_compact)
//         let mut buf = BytesMut::new();
//         // 这是一个 Reth 自动生成的辅助函数，类似于 RLP 的 encode
//         let len = AccountInfo::to_compact(&acc, &mut buf);

//         println!("Original: {:?}", acc);
//         println!("Encoded Hex: {}", hex::encode(&buf));
//         println!("Length: {}", len);

//         // 预期结果分析：
//         // Compact 可能会生成类似这样的数据：
//         // [Bitmask] [Nonce]
//         // 比如 Bitmask 告诉我们要读第1个字段，跳过第3个字段。
//         // Balance 是 0，Compact 甚至可能直接在 Bitmask 里标记 "它是0"，连字节都不写。

//         // 解码 (from_compact)
//         let (decoded, _) = AccountInfo::from_compact(&buf, len);
//         assert_eq!(acc, decoded);
//         println!("解码成功！");
//     }
// }
