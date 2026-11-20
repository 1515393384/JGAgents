#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CtxRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CtxMsg {
    pub seq: u64,
    pub role: CtxRole,
    pub content: String,
    pub token_estimate: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CtxPrefix {
    pub version: u32,
    pub messages: Vec<CtxMsg>,
}

impl CtxPrefix {
    pub fn total_tokens(&self) -> u32 {
        self.messages
            .iter()
            .fold(0u32, |acc, m| acc.saturating_add(m.token_estimate))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssembledWindow {
    pub prefix_version: u32,
    pub messages: Vec<CtxMsg>,
    pub estimated_tokens: u32,
    pub trimmed_count: usize,
}
