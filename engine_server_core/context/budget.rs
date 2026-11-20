use crate::core::error::EngineError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TokenBudget {
    pub max_tokens: u32,
    pub reserve_output: u32,
}

impl TokenBudget {
    pub fn new(max_tokens: u32, reserve_output: u32) -> Result<Self, EngineError> {
        if reserve_output >= max_tokens {
            return Err(EngineError::budget("reserve_output must be smaller than max_tokens"));
        }
        Ok(Self {
            max_tokens,
            reserve_output,
        })
    }

    pub fn prompt_cap(&self) -> u32 {
        self.max_tokens.saturating_sub(self.reserve_output)
    }
}

pub fn estimate_tokens(text: &str) -> u32 {
    if text.is_empty() {
        return 0;
    }
    let mut ascii_bytes = 0u32;
    let mut non_ascii_chars = 0u32;
    for ch in text.chars() {
        if ch.is_ascii() {
            ascii_bytes = ascii_bytes.saturating_add(1);
        } else {
            non_ascii_chars = non_ascii_chars.saturating_add(1);
        }
    }
    let ascii_est = ascii_bytes.div_ceil(4);
    ascii_est.saturating_add(non_ascii_chars).max(1)
}
