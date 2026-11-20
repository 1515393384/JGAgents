#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CacheHitStats {
    pub prompt_tokens: u64,
    pub cached_tokens: u64,
}

impl CacheHitStats {
    pub fn ratio(&self) -> f64 {
        if self.prompt_tokens == 0 {
            return 0.0;
        }
        (self.cached_tokens as f64 / self.prompt_tokens as f64).min(1.0)
    }
}
