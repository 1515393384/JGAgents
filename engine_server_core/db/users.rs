use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use rusqlite::{OptionalExtension, Row};

use crate::core::error::EngineError;
use crate::core::ids::{TokenHash, UserId};
use crate::core::traits::UserStore;
use crate::db::pool::{map_sqlite_err, SqlitePool};
use crate::engine::user::model::{UserQuota, UserRecord, UserRole, UserState};

pub const SQL_LIST_USERS: &str =
    "SELECT id, name, grp, role, state, daily_requests, monthly_tokens, used_requests, used_tokens, token_hash, created_at_ms, last_active_ms FROM users ORDER BY created_at_ms DESC";

pub const SQL_GET_USER_BY_ID: &str =
    "SELECT id, name, grp, role, state, daily_requests, monthly_tokens, used_requests, used_tokens, token_hash, created_at_ms, last_active_ms FROM users WHERE id = ?1";

pub const SQL_GET_USER_BY_TOKEN_HASH: &str =
    "SELECT id, name, grp, role, state, daily_requests, monthly_tokens, used_requests, used_tokens, token_hash, created_at_ms, last_active_ms FROM users WHERE token_hash = ?1";

pub const SQL_INSERT_USER: &str =
    "INSERT INTO users (id, name, grp, role, state, daily_requests, monthly_tokens, used_requests, used_tokens, token_hash, created_at_ms, last_active_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)";

pub const SQL_UPDATE_USER: &str =
    "UPDATE users SET name = ?2, grp = ?3, role = ?4, state = ?5, daily_requests = ?6, monthly_tokens = ?7, used_requests = ?8, used_tokens = ?9, token_hash = ?10, last_active_ms = ?11 WHERE id = ?1";

pub const SQL_DELETE_USER: &str = "DELETE FROM users WHERE id = ?1";

pub struct SqliteUserStore {
    pool: Arc<SqlitePool>,
}

impl SqliteUserStore {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

fn row_to_user(row: &Row<'_>) -> rusqlite::Result<UserRecord> {
    let id: String = row.get(0)?;
    let name: String = row.get(1)?;
    let group: String = row.get(2)?;
    let role_raw: String = row.get(3)?;
    let state_raw: String = row.get(4)?;
    let daily_requests: i64 = row.get(5)?;
    let monthly_tokens: i64 = row.get(6)?;
    let used_requests: i64 = row.get(7)?;
    let used_tokens: i64 = row.get(8)?;
    let token_hash: String = row.get(9)?;
    let created_at_ms: i64 = row.get(10)?;
    let last_active_ms: i64 = row.get(11)?;

    let role = UserRole::parse(&role_raw).unwrap_or(UserRole::Member);
    let state = UserState::parse(&state_raw).unwrap_or(UserState::Disabled);

    Ok(UserRecord {
        id: UserId::new(id),
        name,
        group,
        role,
        state,
        quota: UserQuota {
            daily_requests: daily_requests.max(0) as u64,
            monthly_tokens: monthly_tokens.max(0) as u64,
            used_requests: used_requests.max(0) as u64,
            used_tokens: used_tokens.max(0) as u64,
        },
        token_hash: TokenHash::from_hex(token_hash),
        created_at_ms,
        last_active_ms,
    })
}

impl UserStore for SqliteUserStore {
    fn list_all(&self) -> Result<Vec<UserRecord>, EngineError> {
        self.pool.with_reader(|conn| {
            let mut stmt = conn
                .prepare(SQL_LIST_USERS)
                .map_err(|e| map_sqlite_err(e, "prepare list users"))?;
            let rows = stmt
                .query_map([], row_to_user)
                .map_err(|e| map_sqlite_err(e, "query list users"))?;
            let mut users = Vec::new();
            for row in rows {
                users.push(row.map_err(|e| map_sqlite_err(e, "read user row"))?);
            }
            Ok(users)
        })
    }

    fn get_by_id(&self, id: &UserId) -> Result<Option<UserRecord>, EngineError> {
        self.pool.with_reader(|conn| {
            conn.query_row(SQL_GET_USER_BY_ID, [id.as_str()], row_to_user)
                .optional()
                .map_err(|e| map_sqlite_err(e, "get user by id"))
        })
    }

    fn get_by_token_hash(&self, hash: &TokenHash) -> Result<Option<UserRecord>, EngineError> {
        self.pool.with_reader(|conn| {
            conn.query_row(SQL_GET_USER_BY_TOKEN_HASH, [hash.as_str()], row_to_user)
                .optional()
                .map_err(|e| map_sqlite_err(e, "get user by token"))
        })
    }

    fn insert(&self, user: &UserRecord) -> Result<(), EngineError> {
        self.pool.with_writer(|conn| {
            conn.execute(
                SQL_INSERT_USER,
                rusqlite::params![
                    user.id.as_str(),
                    user.name,
                    user.group,
                    user.role.as_str(),
                    user.state.as_str(),
                    user.quota.daily_requests as i64,
                    user.quota.monthly_tokens as i64,
                    user.quota.used_requests as i64,
                    user.quota.used_tokens as i64,
                    user.token_hash.as_str(),
                    user.created_at_ms,
                    user.last_active_ms,
                ],
            )
            .map_err(|e| map_sqlite_err(e, "insert user"))?;
            Ok(())
        })
    }

    fn update(&self, user: &UserRecord) -> Result<(), EngineError> {
        self.pool.with_writer(|conn| {
            let changed = conn
                .execute(
                    SQL_UPDATE_USER,
                    rusqlite::params![
                        user.id.as_str(),
                        user.name,
                        user.group,
                        user.role.as_str(),
                        user.state.as_str(),
                        user.quota.daily_requests as i64,
                        user.quota.monthly_tokens as i64,
                        user.quota.used_requests as i64,
                        user.quota.used_tokens as i64,
                        user.token_hash.as_str(),
                        user.last_active_ms,
                    ],
                )
                .map_err(|e| map_sqlite_err(e, "update user"))?;
            if changed == 0 {
                return Err(EngineError::not_found("user not found"));
            }
            Ok(())
        })
    }

    fn delete(&self, id: &UserId) -> Result<bool, EngineError> {
        self.pool.with_writer(|conn| {
            let changed = conn
                .execute(SQL_DELETE_USER, [id.as_str()])
                .map_err(|e| map_sqlite_err(e, "delete user"))?;
            Ok(changed > 0)
        })
    }
}

pub struct InMemoryUserStore {
    rows: RwLock<HashMap<UserId, UserRecord>>,
}

impl InMemoryUserStore {
    pub fn new(seed: Vec<UserRecord>) -> Self {
        let mut map = HashMap::with_capacity(seed.len());
        for user in seed {
            map.insert(user.id.clone(), user);
        }
        Self {
            rows: RwLock::new(map),
        }
    }
}

impl Default for InMemoryUserStore {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl UserStore for InMemoryUserStore {
    fn list_all(&self) -> Result<Vec<UserRecord>, EngineError> {
        let guard = self
            .rows
            .read()
            .map_err(|_| EngineError::internal("user store lock poisoned"))?;
        Ok(guard.values().cloned().collect())
    }

    fn get_by_id(&self, id: &UserId) -> Result<Option<UserRecord>, EngineError> {
        let guard = self
            .rows
            .read()
            .map_err(|_| EngineError::internal("user store lock poisoned"))?;
        Ok(guard.get(id).cloned())
    }

    fn get_by_token_hash(&self, hash: &TokenHash) -> Result<Option<UserRecord>, EngineError> {
        let guard = self
            .rows
            .read()
            .map_err(|_| EngineError::internal("user store lock poisoned"))?;
        Ok(guard.values().find(|u| &u.token_hash == hash).cloned())
    }

    fn insert(&self, user: &UserRecord) -> Result<(), EngineError> {
        let mut guard = self
            .rows
            .write()
            .map_err(|_| EngineError::internal("user store lock poisoned"))?;
        if guard.contains_key(&user.id) {
            return Err(EngineError::conflict("duplicate user id"));
        }
        guard.insert(user.id.clone(), user.clone());
        Ok(())
    }

    fn update(&self, user: &UserRecord) -> Result<(), EngineError> {
        let mut guard = self
            .rows
            .write()
            .map_err(|_| EngineError::internal("user store lock poisoned"))?;
        if !guard.contains_key(&user.id) {
            return Err(EngineError::not_found("user not found"));
        }
        guard.insert(user.id.clone(), user.clone());
        Ok(())
    }

    fn delete(&self, id: &UserId) -> Result<bool, EngineError> {
        let mut guard = self
            .rows
            .write()
            .map_err(|_| EngineError::internal("user store lock poisoned"))?;
        Ok(guard.remove(id).is_some())
    }
}
