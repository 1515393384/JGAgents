use std::collections::HashMap;
use std::sync::RwLock;

use crate::core::error::EngineError;
use crate::core::ids::{TokenHash, UserId};
use crate::core::traits::UserStore;
use crate::engine::user::model::UserRecord;

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
