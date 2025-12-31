#[cfg(test)]
mod tests {
    use alloy_rlp::{Decodable, Encodable};

    #[test]
    fn practice_primitives() {
        println!("--- 练习 1: 基础类型 ---");

        let num_u8 = 100_u8;
        let mut buf = Vec::new();

        num_u8.encode(&mut buf);

        println!("u8(100) RLP Encoded: 0x{}", hex::encode(&buf));

        let s = "dog".to_string();
        let mut buf2 = Vec::new();
        s.encode(&mut buf2);
        println!("String(\"dog\") RLP Encoded: 0x{}", hex::encode(&buf2));

        let mut slice = buf.as_slice();
        let decoded_num = u8::decode(&mut slice).unwrap();
        assert_eq!(decoded_num, num_u8);

        let mut slice2 = buf2.as_slice();
        let decoded_str = String::decode(&mut slice2).unwrap();
        assert_eq!(decoded_str, s);
    }

    #[test]
    fn practice_lists() {
        println!("\n--- 练习 2: 列表 (Vec) ---");

        // 一个包含两个字符串的列表 ["cat", "dog"]
        // RLP 列表前缀通常是 0xC0 + 总长度
        let list = vec!["cat".to_string(), "dog".to_string()];

        let mut buf = Vec::new();
        list.encode(&mut buf);

        println!("List['cat', 'dog'] Encoded: 0x{}", hex::encode(&buf));
        // 预期分析:
        // "cat" -> 83 63 61 74 (4字节)
        // "dog" -> 83 64 6f 67 (4字节)
        // 总Payload长度 = 8字节
        // 列表前缀 = 0xC0 + 8 = 0xC8
        // 预期结果: c8 83636174 83646f67

        // 解码
        let mut slice = buf.as_slice();
        let decoded_list: Vec<String> = Decodable::decode(&mut slice).unwrap();
        assert_eq!(list, decoded_list);
        println!("解码列表成功！");
    }

    use alloy_rlp::{RlpDecodable, RlpEncodable};

    // 模拟一个极其简化的以太坊交易
    // 必须加 RlpEncodable 和 RlpDecodable
    #[derive(Debug, PartialEq, RlpEncodable, RlpDecodable)]
    struct SimpleTx {
        nonce: u64,
        price: u64,
        to: String, // 实际开发中应该用 Address 类型，这里用 String 演示
    }

    #[test]
    fn practice_structs() {
        println!("\n--- 练习 3: 自定义结构体 ---");

        let tx = SimpleTx {
            nonce: 5,
            price: 1000,
            to: "0x1234".to_string(),
        };

        let mut buf = Vec::new();
        tx.encode(&mut buf);

        let hex_output = hex::encode(&buf);
        println!("SimpleTx Encoded: 0x{}", hex_output);

        // 我们可以尝试把这个 hex 放到以太坊 RLP 解析器网站验证
        // 但在本地，我们直接解码验证
        let mut slice = buf.as_slice();
        let decoded_tx = SimpleTx::decode(&mut slice).unwrap();

        println!("Original: {:?}", tx);
        println!("Decoded : {:?}", decoded_tx);
        assert_eq!(tx, decoded_tx);
    }

    use tiny_keccak::{Hasher, Keccak};

    #[test]
    fn practice_hashing() {
        println!("\n--- 练习 4: 像 Reth 一样计算 Hash ---");

        let tx = SimpleTx {
            nonce: 1,
            price: 500,
            to: "eth".to_string(),
        };

        // 1. 第一步：RLP 编码拿到字节
        let mut rlp_buf = Vec::new();
        tx.encode(&mut rlp_buf);
        println!("RLP Bytes: 0x{}", hex::encode(&rlp_buf));

        // 2. 第二步：对字节做 Keccak256
        let mut hasher = Keccak::v256();
        let mut output = [0u8; 32]; // 哈希总是 32 字节
        hasher.update(&rlp_buf);
        hasher.finalize(&mut output);

        println!("Transaction Hash: 0x{}", hex::encode(output));
        println!("(这在以太坊中就是你在 Etherscan 上查的那串 0x... TxHash)");
    }

    // #[allow(unused)]
    // use reth_rpc_types::transactions::Transaction;

    // 这一般是自动生成的代码
    // 方式 A：直接写全路径（推荐，防止重名）
    #[allow(dead_code)]
    #[derive(Clone, PartialEq, prost::Message)]
    pub struct ProtoSimpleTx {
        #[prost(string, tag = "1")]
        pub to: String,

        #[prost(uint64, tag = "2")]
        pub price: u64,
    }

    impl From<&SimpleTx> for ProtoSimpleTx {
        fn from(tx: &SimpleTx) -> Self {
            Self {
                to: tx.to.to_string(),
                price: tx.price,
            }
        }
    }
}
