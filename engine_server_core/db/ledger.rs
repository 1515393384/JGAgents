use std::collections::BTreeMap;
use std::sync::Arc;

use crate::core::error::EngineError;
use crate::core::ids::{ConvId, UserId};
use crate::core::traits::LedgerStore;
use crate::db::pool::{map_sqlite_err, SqlitePool};

const MS_PER_DAY: i64 = 86_400_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DailyTrendRow {
    pub day_ms: i64,
    pub requests: u64,
    pub hit_rate: u8,
    pub tokens_k: u64,
}

pub struct SqliteLedgerStore {
    pool: Arc<SqlitePool>,
}

impl SqliteLedgerStore {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    pub fn daily_trend(&self, now_ms: i64, days: usize) -> Result<Vec<DailyTrendRow>, EngineError> {
        let window_days = days.clamp(1, 30) as i64;
        let today_bucket = now_ms.max(0) / MS_PER_DAY;
        let start_bucket = (today_bucket - (window_days - 1)).max(0);
        let since_ms = start_bucket * MS_PER_DAY;

        self.pool.with_reader(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT created_at_ms / 86400000 AS day_bucket,
                            COUNT(*) AS req_count,
                            COALESCE(SUM(prompt_tokens), 0) AS sum_prompt,
                            COALESCE(SUM(cached_tokens), 0) AS sum_cached,
                            COALESCE(SUM(completion_tokens), 0) AS sum_completion
                     FROM ledger
                     WHERE created_at_ms >= ?1
                     GROUP BY day_bucket
                     ORDER BY day_bucket ASC",
                )
                .map_err(|e| map_sqlite_err(e, "prepare daily trend"))?;

            let mut buckets: BTreeMap<i64, DailyTrendRow> = BTreeMap::new();
            let rows = stmt
                .query_map([since_ms], |row| {
                    let bucket: i64 = row.get(0)?;
                    let req_count: i64 = row.get(1)?;
                    let sum_prompt: i64 = row.get(2)?;
                    let sum_cached: i64 = row.get(3)?;
                    let sum_completion: i64 = row.get(4)?;

                    let prompt_u = sum_prompt.max(0) as u64;
                    let cached_u = sum_cached.max(0) as u64;
                    let comp_u = sum_completion.max(0) as u64;
                    let hit_rate = if prompt_u > 0 {
                        ((cached_u.saturating_mul(100)) / prompt_u).min(100) as u8
                    } else {
                        0
                    };
                    let tokens_k = (prompt_u + comp_u) / 1000;

                    Ok((
                        bucket,
                        DailyTrendRow {
                            day_ms: bucket * MS_PER_DAY,
                            requests: req_count.max(0) as u64,
                            hit_rate,
                            tokens_k,
                        },
                    ))
                })
                .map_err(|e| map_sqlite_err(e, "query daily trend"))?;

            for row in rows {
                let (bucket, entry) = row.map_err(|e| map_sqlite_err(e, "read trend row"))?;
                buckets.insert(bucket, entry);
            }

            let mut series = Vec::with_capacity(window_days as usize);
            for offset in 0..window_days {
                let bucket = start_bucket + offset;
                let entry = buckets.remove(&bucket).unwrap_or(DailyTrendRow {
                    day_ms: bucket * MS_PER_DAY,
                    requests: 0,
                    hit_rate: 0,
                    tokens_k: 0,
                });
                series.push(entry);
            }
            Ok(series)
        })
    }
}

impl LedgerStore for SqliteLedgerStore {
    fn record_usage(
        &self,
        user: &UserId,
        conv: &ConvId,
        prompt_tokens: u64,
        cached_tokens: u64,
        completion_tokens: u64,
        now_ms: i64,
    ) -> Result<(), EngineError> {
        self.pool.with_writer(|conn| {
            let tx = conn
                .transaction()
                .map_err(|e| map_sqlite_err(e, "begin record_usage"))?;

            tx.execute(
                "INSERT INTO ledger (user_id, conv_id, prompt_tokens, cached_tokens, completion_tokens, created_at_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    user.as_str(),
                    conv.as_str(),
                    prompt_tokens as i64,
                    cached_tokens as i64,
                    completion_tokens as i64,
                    now_ms,
                ],
            )
            .map_err(|e| map_sqlite_err(e, "insert ledger"))?;

            let billed_tokens = prompt_tokens.saturating_add(completion_tokens) as i64;
            tx.execute(
                "UPDATE users
                 SET used_requests = used_requests + 1,
                     used_tokens = used_tokens + ?2,
                     last_active_ms = CASE WHEN ?3 > last_active_ms THEN ?3 ELSE last_active_ms END
                 WHERE id = ?1",
                rusqlite::params![user.as_str(), billed_tokens, now_ms],
            )
            .map_err(|e| map_sqlite_err(e, "update user usage"))?;

            tx.commit()
                .map_err(|e| map_sqlite_err(e, "commit record_usage"))?;
            Ok(())
        })
    }
}
