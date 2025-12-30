use bytes::{Buf, BytesMut};
use serde::{Deserialize, Serialize};
use tokio_util::codec::Decoder;

#[allow(dead_code)]
#[allow(unused_variables)]
// ================= 1. 定义消息协议 =================
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum P2PMessage {
    Hello { version: u32 },
    Ping,
    Pong,
}

// 解码器结构体（通常这里是空的，除非你需要存一些状态，比如“正在读头部”）
#[allow(dead_code)]
pub struct P2PCodec;

// ================= 2. 核心实现：Decoder =================
impl Decoder for P2PCodec {
    type Item = P2PMessage;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Step 1: 【守门】检查头部是否完整
        // 我们的协议头是 4 字节 (u32)
        if src.len() < 4 {
            // 数据不够，告诉 Tokio：别急，再去网卡读点数据，攒够了再叫我
            return Ok(None);
        }

        // Step 2: 【偷看 (Peek)】读取长度，但不消耗数据
        // 注意：千万不能用 src.get_u32()，因为它会把这4个字节吃掉（移动游标）。
        // 如果后面 Payload 不够，下次进来游标就不对了。
        // 所以我们用切片语法 src[..4] 只是看一眼。
        let mut length_bytes = [0u8; 4];
        length_bytes.copy_from_slice(&src[..4]);
        let length = u32::from_be_bytes(length_bytes) as usize;

        // Step 3: 【验货】检查剩余数据是否满足 Payload 长度
        // 需要的总长度 = 头部(4) + 内容(length)
        if src.len() < 4 + length {
            // 头部有了，但身子还没收全（这就是典型的“半包/拆包”）。
            // 告诉 Tokio：我要更多数据。
            // 优化：告诉 buffer 预留空间，避免频繁扩容
            src.reserve(4 + length - src.len());
            return Ok(None);
        }

        // Step 4: 【切割】数据齐了！开始动刀（解决粘包）
        // 走到这里，src 里一定包含至少一个完整的包。

        // A. 真的消耗掉前4个字节（头部）
        src.advance(4);

        // B. 切割下 Payload
        // split_to(n) 会做两件事：
        // 1. 把 src 的前 n 个字节切下来返回给 data。
        // 2. src 剩下的部分保留（可能是下一个粘包的数据）。
        let data = src.split_to(length);

        // Step 5: 反序列化
        match serde_json::from_slice(&data) {
            Ok(msg) => Ok(Some(msg)),
            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        }
    }
}

// ================= 3. 测试用例验证 =================
#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BufMut;

    #[test]
    fn test_sticky_and_partial() {
        let mut codec = P2PCodec;
        let mut buf = BytesMut::new();

        // 构造两个消息
        let msg1 = P2PMessage::Hello { version: 1 };
        let json1 = serde_json::to_string(&msg1).unwrap();

        let msg2 = P2PMessage::Ping;
        let json2 = serde_json::to_string(&msg2).unwrap();

        // --- 模拟场景：极端粘包 + 拆包 ---

        // 1. 放入 Msg1 的完整数据 (Length + Payload)
        buf.put_u32(json1.len() as u32);
        buf.put_slice(json1.as_bytes());

        // 2. 紧接着放入 Msg2 的头部 (粘在 Msg1 后面)
        buf.put_u32(json2.len() as u32);

        // 3. 放入 Msg2 的 Payload 的一半 (拆包，没发完)
        let half_len = json2.len() / 2;
        buf.put_slice(&json2.as_bytes()[..half_len]);

        println!("当前 Buffer 状态: {:?}", buf);

        // --- 第一次 Decode ---
        // 预期：应该成功解析出 Msg1，但 Msg2 数据不够，应该停下
        let res1 = codec.decode(&mut buf).unwrap();
        assert_eq!(res1, Some(msg1));
        println!("成功解析 Msg1，Buffer 剩余长度: {}", buf.len());

        // --- 第二次 Decode (Tokio 自动循环调用) ---
        // 此时 Buffer 里剩下了 Msg2 的头 + 半个身子
        let res2 = codec.decode(&mut buf).unwrap();
        assert_eq!(res2, None, "数据不够，应该返回 None");

        // --- 模拟网络又来了新数据：补齐 Msg2 剩下的一半 ---
        buf.put_slice(&json2.as_bytes()[half_len..]);
        println!("补齐数据后 Buffer 长度: {}", buf.len());

        // --- 第三次 Decode ---
        let res3 = codec.decode(&mut buf).unwrap();
        assert_eq!(res3, Some(msg2));
        println!("成功解析 Msg2，Buffer 清空");

        assert!(buf.is_empty());
    }

    #[test]
    fn test_bytes_mut_api() {
        let mut buf = BytesMut::with_capacity(1024);

        buf.put_u8(0x01);
        buf.put_u16(0x0203);
        buf.put_slice(b"Reth");

        assert_eq!(buf.len(), 7);
    }
}
