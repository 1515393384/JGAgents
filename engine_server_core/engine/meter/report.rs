use crate::core::traits::DailyTrendRow;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MeterTotals {
    pub requests: u64,
    pub prompt_tokens: u64,
    pub cached_tokens: u64,
    pub completion_tokens: u64,
}

pub fn summarize_trend(rows: &[DailyTrendRow]) -> MeterTotals {
    let mut totals = MeterTotals::default();
    for r in rows {
        totals.requests = totals.requests.saturating_add(r.requests);
        totals.prompt_tokens = totals.prompt_tokens.saturating_add(r.prompt_tokens);
        totals.cached_tokens = totals.cached_tokens.saturating_add(r.cached_tokens);
        totals.completion_tokens = totals.completion_tokens.saturating_add(r.completion_tokens);
    }
    totals
}
