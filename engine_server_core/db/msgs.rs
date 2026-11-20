use std::sync::Arc;

use rusqlite::TransactionBehavior;

use crate::core::error::EngineError;
use crate::core::ids::{ConvId, UserId};
use crate::core::traits::{ArchiveRecord, ConvStore};
use crate::db::pool::{map_sqlite_err, SqlitePool};

pub struct SqliteConvStore {
    pool: Arc<SqlitePool>,
}

impl SqliteConvStore {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

impl ConvStore for SqliteConvStore {
    fn append(
        &self,
        user: &UserId,
        conv: &ConvId,
        payload: &[u8],
        now_ms: i64,
    ) -> Result<u64, EngineError> {
        self.pool.with_writer(|conn| {
            let tx = conn
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|e| map_sqlite_err(e, "begin append msg"))?;
            let next_seq: i64 = tx
                .query_row(
                    "SELECT COALESCE(MAX(seq), 0) + 1 FROM msgs WHERE user_id = ?1 AND conv_id = ?2",
                    rusqlite::params![user.as_str(), conv.as_str()],
                    |row| row.get(0),
                )
                .map_err(|e| map_sqlite_err(e, "query next seq"))?;

            tx.execute(
                "INSERT INTO msgs (user_id, conv_id, seq, payload, created_at_ms) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![user.as_str(), conv.as_str(), next_seq, payload, now_ms],
            )
            .map_err(|e| map_sqlite_err(e, "insert msg"))?;

            tx.commit()
                .map_err(|e| map_sqlite_err(e, "commit append msg"))?;
            Ok(next_seq.max(0) as u64)
        })
    }

    fn load_since(
        &self,
        user: &UserId,
        conv: &ConvId,
        since_seq: u64,
    ) -> Result<Vec<(u64, Vec<u8>)>, EngineError> {
        self.pool.with_reader(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT seq, payload FROM msgs WHERE user_id = ?1 AND conv_id = ?2 AND seq > ?3 ORDER BY seq ASC",
                )
                .map_err(|e| map_sqlite_err(e, "prepare load_since"))?;

            let rows = stmt
                .query_map(
                    rusqlite::params![user.as_str(), conv.as_str(), since_seq as i64],
                    |row| {
                        let seq: i64 = row.get(0)?;
                        let payload: Vec<u8> = row.get(1)?;
                        Ok((seq.max(0) as u64, payload))
                    },
                )
                .map_err(|e| map_sqlite_err(e, "query load_since"))?;

            let mut out = Vec::new();
            for row in rows {
                out.push(row.map_err(|e| map_sqlite_err(e, "read msg row"))?);
            }
            Ok(out)
        })
    }

    fn list_archives(&self, limit: usize) -> Result<Vec<ArchiveRecord>, EngineError> {
        let cap = limit.clamp(1, 200) as i64;
        self.pool.with_reader(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT m.conv_id,
                            COALESCE(u.name, m.user_id) AS owner_name,
                            COALESCE(u.grp, '-') AS grp,
                            COUNT(*) AS msg_count,
                            COALESCE(SUM(LENGTH(m.payload)), 0) AS total_bytes,
                            MAX(m.created_at_ms) AS updated_at_ms
                     FROM msgs m
                     LEFT JOIN users u ON u.id = m.user_id
                     GROUP BY m.user_id, m.conv_id
                     ORDER BY updated_at_ms DESC
                     LIMIT ?1",
                )
                .map_err(|e| map_sqlite_err(e, "prepare list archives"))?;

            let rows = stmt
                .query_map([cap], |row| {
                    let conv_id: String = row.get(0)?;
                    let owner_name: String = row.get(1)?;
                    let group: String = row.get(2)?;
                    let messages: i64 = row.get(3)?;
                    let total_bytes: i64 = row.get(4)?;
                    let updated_at_ms: i64 = row.get(5)?;
                    Ok(ArchiveRecord {
                        conv_id,
                        owner_name,
                        group,
                        messages: messages.max(0) as u64,
                        prefix_version: 1,
                        bytes: total_bytes.max(0) as u64,
                        updated_at_ms,
                    })
                })
                .map_err(|e| map_sqlite_err(e, "query list archives"))?;

            let mut out = Vec::new();
            for row in rows {
                out.push(row.map_err(|e| map_sqlite_err(e, "read archive row"))?);
            }
            Ok(out)
        })
    }
}
