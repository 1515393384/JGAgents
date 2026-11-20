//! 架阁 · 通用 Agent 客户端内核：状态机 + 只算一次的前缀。
//!
//! 边界：本 crate 不碰窗口、不碰网络传输，只做「读上下文 → 要模型 → 执行工具 → 回填」。

use serde::{Deserialize, Serialize};

/// 一条消息。前缀只由前面的消息决定，后追加的消息永远不改变它。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Msg {
    pub role: Role,
    pub body: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    User,
    Assistant,
    Tool,
}

/// 前缀指纹：同样的一串消息 → 同样的字节 → 同样的 hash，用来判定「这段算过了」。
pub fn prefix_hash(msgs: &[Msg]) -> u64 {
    const PRIME: u64 = 0x1000_0000_01b3;
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for m in msgs {
        h ^= m.role as u64;
        h = h.wrapping_mul(PRIME);
        for b in m.body.bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(PRIME);
        }
    }
    h
}

/// 状态机：在任何时刻，会话只处于其中一种状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// 等用户说话
    Idle,
    /// 前缀已固定，等模型吐第一帧
    Thinking { prefix: u64 },
    /// 模型要看工具结果
    Acting { call: String },
    /// 结束或被中断
    Done { interrupted: bool },
}

/// 工具由客户端实现，内核只认名字和 JSON 参数。
pub trait Tool {
    fn name(&self) -> &str;
    fn run(&mut self, args: &str) -> String;
}
