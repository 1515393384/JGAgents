use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use rusqlite::OptionalExtension;

use crate::core::error::EngineError;
use crate::core::ids::{TokenHash, UserId};
use crate::core::traits::SessionStore;
use crate::db::pool::{map_sqlite_err, SqlitePool};
use crate::engine::user::model::SessionRecord;

pub struct SqliteSessionStore {
    pool: Arc<SqlitePool>,
}

impl SqliteSessionStore {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

impl SessionStore for SqliteSessionStore {
    fn create(&self, session: &SessionRecord) -> Result<(), EngineError> {
        self.pool.with_writer(|conn| {
            conn.execute(
                "DELETE FROM sessions WHERE expires_at_ms <= ?1",
                rusqlite::params![session.created_at_ms],
            )
            .map_err(|e| map_sqlite_err(e, "prune expired sessions"))?;

            conn.execute(
                "INSERT OR REPLACE INTO sessions (token_hash, user_id, created_at_ms, expires_at_ms)
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![
                    session.token_hash.as_str(),
                    session.user_id.as_str(),
                    session.created_at_ms,
                    session.expires_at_ms,
                ],
            )
            .map_err(|e| map_sqlite_err(e, "insert session"))?;
            Ok(())
        })
    }

    fn get_valid(
        &self,
        token_hash: &TokenHash,
        now_ms: i64,
    ) -> Result<Option<SessionRecord>, EngineError> {
        self.pool.with_reader(|conn| {
            conn.query_row(
                "SELECT token_hash, user_id, created_at_ms, expires_at_ms
                 FROM sessions
                 WHERE token_hash = ?1 AND expires_at_ms > ?2",
                rusqlite::params![token_hash.as_str(), now_ms],
                |row| {
                    let th: String = row.get(0)?;
                    let uid: String = row.get(1)?;
                    let created_at_ms: i64 = row.get(2)?;
                    let expires_at_ms: i64 = row.get(3)?;
                    Ok(SessionRecord {
                        token_hash: TokenHash::from_hex(th),
                        user_id: UserId::new(uid),
                        created_at_ms,
                        expires_at_ms,
                    })
                },
            )
            .optional()
            .map_err(|e| map_sqlite_err(e, "get valid session"))
        })
    }

    fn delete(&self, token_hash: &TokenHash) -> Result<(), EngineError> {
        self.pool.with_writer(|conn| {
            conn.execute(
                "DELETE FROM sessions WHERE token_hash = ?1",
                [token_hash.as_str()],
            )
            .map_err(|e| map_sqlite_err(e, "delete session"))?;
            Ok(())
        })
    }

    fn delete_for_user(&self, user_id: &UserId) -> Result<(), EngineError> {
        self.pool.with_writer(|conn| {
            conn.execute(
                "DELETE FROM sessions WHERE user_id = ?1",
                [user_id.as_str()],
            )
            .map_err(|e| map_sqlite_err(e, "delete user sessions"))?;
            Ok(())
        })
    }
}

#[derive(Default)]
pub struct InMemorySessionStore {
    rows: RwLock<HashMap<TokenHash, SessionRecord>>,
}

impl InMemorySessionStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SessionStore for InMemorySessionStore {
    fn create(&self, session: &SessionRecord) -> Result<(), EngineError> {
        let mut guard = self
            .rows
            .write()
            .map_err(|_| EngineError::internal("session store lock poisoned"))?;
        guard.retain(|_, s| s.expires_at_ms > session.created_at_ms);
        guard.insert(session.token_hash.clone(), session.clone());
        Ok(())
    }

    fn get_valid(
        &self,
        token_hash: &TokenHash,
        now_ms: i64,
    ) -> Result<Option<SessionRecord>, EngineError> {
        let guard = self
            .rows
            .read()
            .map_err(|_| EngineError::internal("session store lock poisoned"))?;
        Ok(guard
            .get(token_hash)
            .filter(|s| s.expires_at_ms > now_ms)
            .cloned())
    }

    fn delete(&self, token_hash: &TokenHash) -> Result<(), EngineError> {
        let mut guard = self
            .rows
            .write()
            .map_err(|_| EngineError::internal("session store lock poisoned"))?;
        guard.remove(token_hash);
        Ok(())
    }

    fn delete_for_user(&self, user_id: &UserId) -> Result<(), EngineError> {
        let mut guard = self
            .rows
            .write()
            .map_err(|_| EngineError::internal("session store lock poisoned"))?;
        guard.retain(|_, s| &s.user_id != user_id);
        Ok(())
    }
}
