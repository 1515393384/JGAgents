use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use rusqlite::types::Value;
use rusqlite::{OptionalExtension, Row, TransactionBehavior};

use crate::core::error::EngineError;
use crate::core::ids::{TokenHash, UserId};
use crate::core::traits::UserStore;
use crate::db::pool::{map_sqlite_err, SqlitePool};
use crate::engine::user::model::{
    UpdateUserInput, UserMetrics, UserPage, UserQuery, UserQuota, UserRecord, UserRole, UserSort,
    UserState,
};

const USER_COLUMNS: &str = "id, name, grp, role, state, daily_requests, monthly_tokens, used_requests, used_tokens, token_hash, token_hint, created_at_ms, last_active_ms";

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
    let token_hint: String = row.get(10)?;
    let created_at_ms: i64 = row.get(11)?;
    let last_active_ms: i64 = row.get(12)?;

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
        token_hint,
        created_at_ms,
        last_active_ms,
    })
}

fn escape_like(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if matches!(ch, '\\' | '%' | '_') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

fn count_active_admins_tx(tx: &rusqlite::Transaction<'_>) -> Result<i64, EngineError> {
    tx.query_row(
        "SELECT COUNT(*) FROM users WHERE role = 'admin' AND state = 'active'",
        [],
        |row| row.get(0),
    )
    .map_err(|e| map_sqlite_err(e, "count active admins"))
}

impl UserStore for SqliteUserStore {
    fn is_empty(&self) -> Result<bool, EngineError> {
        self.pool.with_reader(|conn| {
            let empty: i64 = conn
                .query_row("SELECT NOT EXISTS(SELECT 1 FROM users)", [], |row| row.get(0))
                .map_err(|e| map_sqlite_err(e, "check users empty"))?;
            Ok(empty != 0)
        })
    }

    fn query(&self, query: &UserQuery) -> Result<UserPage, EngineError> {
        let page = query.page.max(1);
        let page_size = query.page_size.clamp(1, 200);
        let offset = (page - 1).saturating_mul(page_size);

        let mut conditions: Vec<String> = Vec::new();
        let mut params: Vec<Value> = Vec::new();

        if let Some(state) = query.state {
            params.push(Value::Text(state.as_str().to_string()));
            conditions.push(format!("state = ?{}", params.len()));
        }
        if let Some(role) = query.role {
            params.push(Value::Text(role.as_str().to_string()));
            conditions.push(format!("role = ?{}", params.len()));
        }
        if let Some(ref group) = query.group {
            let trimmed = group.trim();
            if !trimmed.is_empty() && trimmed != "all" {
                params.push(Value::Text(trimmed.to_string()));
                conditions.push(format!("grp = ?{}", params.len()));
            }
        }
        if let Some(ref search) = query.search {
            let trimmed = search.trim();
            if !trimmed.is_empty() {
                let pattern = format!("%{}%", escape_like(trimmed));
                params.push(Value::Text(pattern));
                let idx = params.len();
                conditions.push(format!(
                    "(name LIKE ?{idx} ESCAPE '\\' OR id LIKE ?{idx} ESCAPE '\\' OR grp LIKE ?{idx} ESCAPE '\\')"
                ));
            }
        }

        let where_sql = if conditions.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", conditions.join(" AND "))
        };

        let order_sql = match query.sort {
            UserSort::CreatedDesc => "ORDER BY created_at_ms DESC, id ASC",
            UserSort::UsageDesc => {
                "ORDER BY CASE WHEN monthly_tokens > 0 THEN CAST(used_tokens AS REAL) / CAST(monthly_tokens AS REAL) ELSE 0.0 END DESC, used_tokens DESC, created_at_ms DESC"
            }
            UserSort::RequestsDesc => "ORDER BY used_requests DESC, created_at_ms DESC",
        };

        self.pool.with_reader(|conn| {
            let count_sql = format!("SELECT COUNT(*) FROM users{where_sql}");
            let total: i64 = conn
                .query_row(
                    &count_sql,
                    rusqlite::params_from_iter(params.iter()),
                    |row| row.get(0),
                )
                .map_err(|e| map_sqlite_err(e, "count queried users"))?;

            let mut list_params = params;
            list_params.push(Value::Integer(page_size as i64));
            let limit_idx = list_params.len();
            list_params.push(Value::Integer(offset as i64));
            let offset_idx = list_params.len();

            let list_sql = format!(
                "SELECT {USER_COLUMNS} FROM users{where_sql} {order_sql} LIMIT ?{limit_idx} OFFSET ?{offset_idx}"
            );
            let mut stmt = conn
                .prepare(&list_sql)
                .map_err(|e| map_sqlite_err(e, "prepare query users"))?;
            let rows = stmt
                .query_map(rusqlite::params_from_iter(list_params.iter()), row_to_user)
                .map_err(|e| map_sqlite_err(e, "execute query users"))?;

            let mut items = Vec::with_capacity(page_size.min(total.max(0) as usize));
            for row in rows {
                items.push(row.map_err(|e| map_sqlite_err(e, "read queried user row"))?);
            }

            Ok(UserPage {
                items,
                total: total.max(0) as usize,
                page,
                page_size,
            })
        })
    }

    fn metrics(&self) -> Result<UserMetrics, EngineError> {
        self.pool.with_reader(|conn| {
            conn.query_row(
                "SELECT
                    COUNT(*),
                    COALESCE(SUM(CASE WHEN state = 'active' THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN state = 'disabled' THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN role = 'admin' THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN role = 'admin' AND state = 'active' THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(monthly_tokens), 0),
                    COALESCE(SUM(used_tokens), 0),
                    COALESCE(SUM(used_requests), 0)
                 FROM users",
                [],
                |row| {
                    let total_users: i64 = row.get(0)?;
                    let active_users: i64 = row.get(1)?;
                    let disabled_users: i64 = row.get(2)?;
                    let admin_users: i64 = row.get(3)?;
                    let active_admin_users: i64 = row.get(4)?;
                    let total_monthly_quota: i64 = row.get(5)?;
                    let total_used_tokens: i64 = row.get(6)?;
                    let total_used_requests: i64 = row.get(7)?;
                    Ok(UserMetrics {
                        total_users: total_users.max(0) as usize,
                        active_users: active_users.max(0) as usize,
                        disabled_users: disabled_users.max(0) as usize,
                        admin_users: admin_users.max(0) as usize,
                        active_admin_users: active_admin_users.max(0) as usize,
                        total_monthly_quota: total_monthly_quota.max(0) as u64,
                        total_used_tokens: total_used_tokens.max(0) as u64,
                        total_used_requests: total_used_requests.max(0) as u64,
                    })
                },
            )
            .map_err(|e| map_sqlite_err(e, "aggregate user metrics"))
        })
    }

    fn get_by_id(&self, id: &UserId) -> Result<Option<UserRecord>, EngineError> {
        self.pool.with_reader(|conn| {
            let sql = format!("SELECT {USER_COLUMNS} FROM users WHERE id = ?1");
            conn.query_row(&sql, [id.as_str()], row_to_user)
                .optional()
                .map_err(|e| map_sqlite_err(e, "get user by id"))
        })
    }

    fn get_by_token_hash(&self, hash: &TokenHash) -> Result<Option<UserRecord>, EngineError> {
        self.pool.with_reader(|conn| {
            let sql = format!("SELECT {USER_COLUMNS} FROM users WHERE token_hash = ?1");
            conn.query_row(&sql, [hash.as_str()], row_to_user)
                .optional()
                .map_err(|e| map_sqlite_err(e, "get user by token"))
        })
    }

    fn insert(&self, user: &UserRecord) -> Result<(), EngineError> {
        self.pool.with_writer(|conn| {
            conn.execute(
                "INSERT INTO users (id, name, grp, role, state, daily_requests, monthly_tokens, used_requests, used_tokens, token_hash, token_hint, created_at_ms, last_active_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
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
                    user.token_hint,
                    user.created_at_ms,
                    user.last_active_ms,
                ],
            )
            .map_err(|e| map_sqlite_err(e, "insert user"))?;
            Ok(())
        })
    }

    fn update_atomic(
        &self,
        id: &UserId,
        patch: &UpdateUserInput,
    ) -> Result<UserRecord, EngineError> {
        self.pool.with_writer(|conn| {
            let tx = conn
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|e| map_sqlite_err(e, "begin update user"))?;

            let get_sql = format!("SELECT {USER_COLUMNS} FROM users WHERE id = ?1");
            let mut user = tx
                .query_row(&get_sql, [id.as_str()], row_to_user)
                .optional()
                .map_err(|e| map_sqlite_err(e, "load user in update tx"))?
                .ok_or_else(|| EngineError::not_found("user not found"))?;

            let was_active_admin =
                user.role == UserRole::Admin && user.state == UserState::Active;
            let next_role = patch.role.unwrap_or(user.role);
            let next_state = patch.state.unwrap_or(user.state);

            if was_active_admin
                && (next_role != UserRole::Admin || next_state != UserState::Active)
                && count_active_admins_tx(&tx)? <= 1
            {
                return Err(EngineError::conflict(
                    "cannot disable or demote last active admin",
                ));
            }

            if let Some(ref name) = patch.name {
                user.name = name.clone();
            }
            if let Some(ref group) = patch.group {
                user.group = group.clone();
            }
            user.role = next_role;
            user.state = next_state;
            if let Some(daily_requests) = patch.daily_requests {
                user.quota.daily_requests = daily_requests;
            }
            if let Some(monthly_tokens) = patch.monthly_tokens {
                user.quota.monthly_tokens = monthly_tokens;
            }

            tx.execute(
                "UPDATE users
                 SET name = ?2, grp = ?3, role = ?4, state = ?5, daily_requests = ?6, monthly_tokens = ?7
                 WHERE id = ?1",
                rusqlite::params![
                    user.id.as_str(),
                    user.name,
                    user.group,
                    user.role.as_str(),
                    user.state.as_str(),
                    user.quota.daily_requests as i64,
                    user.quota.monthly_tokens as i64,
                ],
            )
            .map_err(|e| map_sqlite_err(e, "update user row"))?;

            tx.commit().map_err(|e| map_sqlite_err(e, "commit update user"))?;
            Ok(user)
        })
    }

    fn rotate_token_atomic(
        &self,
        id: &UserId,
        token_hash: &TokenHash,
        token_hint: &str,
    ) -> Result<UserRecord, EngineError> {
        self.pool.with_writer(|conn| {
            let tx = conn
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|e| map_sqlite_err(e, "begin rotate token"))?;

            let changed = tx
                .execute(
                    "UPDATE users SET token_hash = ?2, token_hint = ?3 WHERE id = ?1",
                    rusqlite::params![id.as_str(), token_hash.as_str(), token_hint],
                )
                .map_err(|e| map_sqlite_err(e, "rotate token"))?;
            if changed == 0 {
                return Err(EngineError::not_found("user not found"));
            }

            let get_sql = format!("SELECT {USER_COLUMNS} FROM users WHERE id = ?1");
            let updated = tx
                .query_row(&get_sql, [id.as_str()], row_to_user)
                .map_err(|e| map_sqlite_err(e, "reload rotated user"))?;

            tx.commit().map_err(|e| map_sqlite_err(e, "commit rotate token"))?;
            Ok(updated)
        })
    }

    fn batch_set_state_atomic(
        &self,
        ids: &[UserId],
        state: UserState,
    ) -> Result<usize, EngineError> {
        if ids.is_empty() {
            return Ok(0);
        }

        self.pool.with_writer(|conn| {
            let tx = conn
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|e| map_sqlite_err(e, "begin batch set state"))?;

            if state == UserState::Disabled {
                let total_active_admins = count_active_admins_tx(&tx)?;
                let placeholders: String = (1..=ids.len())
                    .map(|idx| format!("?{idx}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                let count_sql = format!(
                    "SELECT COUNT(*) FROM users WHERE role = 'admin' AND state = 'active' AND id IN ({placeholders})"
                );
                let id_params: Vec<Value> =
                    ids.iter().map(|id| Value::Text(id.as_str().to_string())).collect();
                let target_active_admins: i64 = tx
                    .query_row(
                        &count_sql,
                        rusqlite::params_from_iter(id_params.iter()),
                        |row| row.get(0),
                    )
                    .map_err(|e| map_sqlite_err(e, "count batch active admins"))?;

                if target_active_admins > 0 && total_active_admins <= target_active_admins {
                    return Err(EngineError::conflict(
                        "cannot disable all active admins",
                    ));
                }
            }

            let placeholders: String = (2..=ids.len() + 1)
                .map(|idx| format!("?{idx}"))
                .collect::<Vec<_>>()
                .join(", ");
            let update_sql =
                format!("UPDATE users SET state = ?1 WHERE id IN ({placeholders})");

            let mut params = Vec::with_capacity(ids.len() + 1);
            params.push(Value::Text(state.as_str().to_string()));
            for id in ids {
                params.push(Value::Text(id.as_str().to_string()));
            }

            let changed = tx
                .execute(&update_sql, rusqlite::params_from_iter(params.iter()))
                .map_err(|e| map_sqlite_err(e, "execute batch set state"))?;

            tx.commit()
                .map_err(|e| map_sqlite_err(e, "commit batch set state"))?;
            Ok(changed)
        })
    }

    fn delete_atomic(&self, id: &UserId) -> Result<(), EngineError> {
        self.pool.with_writer(|conn| {
            let tx = conn
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|e| map_sqlite_err(e, "begin delete user"))?;

            let role_state: Option<(String, String)> = tx
                .query_row(
                    "SELECT role, state FROM users WHERE id = ?1",
                    [id.as_str()],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .map_err(|e| map_sqlite_err(e, "check user before delete"))?;

            let (role_raw, state_raw) =
                role_state.ok_or_else(|| EngineError::not_found("user not found"))?;
            if UserRole::parse(&role_raw) == Some(UserRole::Admin)
                && UserState::parse(&state_raw) == Some(UserState::Active)
                && count_active_admins_tx(&tx)? <= 1
            {
                return Err(EngineError::conflict("cannot delete last active admin"));
            }

            tx.execute("DELETE FROM users WHERE id = ?1", [id.as_str()])
                .map_err(|e| map_sqlite_err(e, "delete user"))?;

            tx.commit().map_err(|e| map_sqlite_err(e, "commit delete user"))?;
            Ok(())
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
    fn is_empty(&self) -> Result<bool, EngineError> {
        let guard = self
            .rows
            .read()
            .map_err(|_| EngineError::internal("user store lock poisoned"))?;
        Ok(guard.is_empty())
    }

    fn query(&self, query: &UserQuery) -> Result<UserPage, EngineError> {
        let guard = self
            .rows
            .read()
            .map_err(|_| EngineError::internal("user store lock poisoned"))?;

        let search_lower = query
            .search
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_ascii_lowercase);
        let group_filter = query
            .group
            .as_deref()
            .map(str::trim)
            .filter(|g| !g.is_empty() && *g != "all");

        let mut list: Vec<UserRecord> = guard
            .values()
            .filter(|u| {
                if let Some(state) = query.state {
                    if u.state != state {
                        return false;
                    }
                }
                if let Some(role) = query.role {
                    if u.role != role {
                        return false;
                    }
                }
                if let Some(grp) = group_filter {
                    if u.group != grp {
                        return false;
                    }
                }
                if let Some(ref q) = search_lower {
                    if !(u.name.to_ascii_lowercase().contains(q)
                        || u.id.as_str().to_ascii_lowercase().contains(q)
                        || u.group.to_ascii_lowercase().contains(q))
                    {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        match query.sort {
            UserSort::CreatedDesc => list.sort_by(|a, b| {
                b.created_at_ms
                    .cmp(&a.created_at_ms)
                    .then_with(|| a.id.cmp(&b.id))
            }),
            UserSort::UsageDesc => list.sort_by(|a, b| {
                b.quota
                    .usage_ratio()
                    .total_cmp(&a.quota.usage_ratio())
                    .then_with(|| b.quota.used_tokens.cmp(&a.quota.used_tokens))
                    .then_with(|| b.created_at_ms.cmp(&a.created_at_ms))
            }),
            UserSort::RequestsDesc => list.sort_by(|a, b| {
                b.quota
                    .used_requests
                    .cmp(&a.quota.used_requests)
                    .then_with(|| b.created_at_ms.cmp(&a.created_at_ms))
            }),
        }

        let total = list.len();
        let page = query.page.max(1);
        let page_size = query.page_size.clamp(1, 200);
        let start = (page - 1).saturating_mul(page_size).min(total);
        let end = start.saturating_add(page_size).min(total);

        Ok(UserPage {
            items: list[start..end].to_vec(),
            total,
            page,
            page_size,
        })
    }

    fn metrics(&self) -> Result<UserMetrics, EngineError> {
        let guard = self
            .rows
            .read()
            .map_err(|_| EngineError::internal("user store lock poisoned"))?;
        let mut metrics = UserMetrics {
            total_users: guard.len(),
            active_users: 0,
            disabled_users: 0,
            admin_users: 0,
            active_admin_users: 0,
            total_monthly_quota: 0,
            total_used_tokens: 0,
            total_used_requests: 0,
        };
        for u in guard.values() {
            match u.state {
                UserState::Active => metrics.active_users += 1,
                UserState::Disabled => metrics.disabled_users += 1,
            }
            if u.role == UserRole::Admin {
                metrics.admin_users += 1;
                if u.state == UserState::Active {
                    metrics.active_admin_users += 1;
                }
            }
            metrics.total_monthly_quota += u.quota.monthly_tokens;
            metrics.total_used_tokens += u.quota.used_tokens;
            metrics.total_used_requests += u.quota.used_requests;
        }
        Ok(metrics)
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
        if guard.contains_key(&user.id)
            || guard
                .values()
                .any(|u| u.name == user.name && u.group == user.group)
        {
            return Err(EngineError::conflict(
                "resource already exists or violates constraint",
            ));
        }
        guard.insert(user.id.clone(), user.clone());
        Ok(())
    }

    fn update_atomic(
        &self,
        id: &UserId,
        patch: &UpdateUserInput,
    ) -> Result<UserRecord, EngineError> {
        let mut guard = self
            .rows
            .write()
            .map_err(|_| EngineError::internal("user store lock poisoned"))?;
        let current = guard
            .get(id)
            .cloned()
            .ok_or_else(|| EngineError::not_found("user not found"))?;

        let next_name = patch.name.clone().unwrap_or_else(|| current.name.clone());
        let next_group = patch.group.clone().unwrap_or_else(|| current.group.clone());
        if guard
            .values()
            .any(|u| &u.id != id && u.name == next_name && u.group == next_group)
        {
            return Err(EngineError::conflict(
                "resource already exists or violates constraint",
            ));
        }

        let next_role = patch.role.unwrap_or(current.role);
        let next_state = patch.state.unwrap_or(current.state);
        let was_active_admin =
            current.role == UserRole::Admin && current.state == UserState::Active;
        if was_active_admin && (next_role != UserRole::Admin || next_state != UserState::Active) {
            let active_admins = guard
                .values()
                .filter(|u| u.role == UserRole::Admin && u.state == UserState::Active)
                .count();
            if active_admins <= 1 {
                return Err(EngineError::conflict(
                    "cannot disable or demote last active admin",
                ));
            }
        }

        let mut updated = current;
        updated.name = next_name;
        updated.group = next_group;
        updated.role = next_role;
        updated.state = next_state;
        if let Some(daily) = patch.daily_requests {
            updated.quota.daily_requests = daily;
        }
        if let Some(monthly) = patch.monthly_tokens {
            updated.quota.monthly_tokens = monthly;
        }
        guard.insert(id.clone(), updated.clone());
        Ok(updated)
    }

    fn rotate_token_atomic(
        &self,
        id: &UserId,
        token_hash: &TokenHash,
        token_hint: &str,
    ) -> Result<UserRecord, EngineError> {
        let mut guard = self
            .rows
            .write()
            .map_err(|_| EngineError::internal("user store lock poisoned"))?;
        let user = guard
            .get_mut(id)
            .ok_or_else(|| EngineError::not_found("user not found"))?;
        user.token_hash = token_hash.clone();
        user.token_hint = token_hint.to_string();
        Ok(user.clone())
    }

    fn batch_set_state_atomic(
        &self,
        ids: &[UserId],
        state: UserState,
    ) -> Result<usize, EngineError> {
        let mut guard = self
            .rows
            .write()
            .map_err(|_| EngineError::internal("user store lock poisoned"))?;
        let id_set: HashSet<&UserId> = ids.iter().collect();

        if state == UserState::Disabled {
            let remaining = guard
                .values()
                .filter(|u| {
                    u.role == UserRole::Admin
                        && u.state == UserState::Active
                        && !id_set.contains(&u.id)
                })
                .count();
            if remaining == 0 {
                return Err(EngineError::conflict(
                    "cannot disable all active admins",
                ));
            }
        }

        let mut changed = 0;
        for (id, user) in guard.iter_mut() {
            if id_set.contains(id) {
                user.state = state;
                changed += 1;
            }
        }
        Ok(changed)
    }

    fn delete_atomic(&self, id: &UserId) -> Result<(), EngineError> {
        let mut guard = self
            .rows
            .write()
            .map_err(|_| EngineError::internal("user store lock poisoned"))?;
        let target = guard
            .get(id)
            .ok_or_else(|| EngineError::not_found("user not found"))?;
        if target.role == UserRole::Admin && target.state == UserState::Active {
            let active_admins = guard
                .values()
                .filter(|u| u.role == UserRole::Admin && u.state == UserState::Active)
                .count();
            if active_admins <= 1 {
                return Err(EngineError::conflict("cannot delete last active admin"));
            }
        }
        guard.remove(id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::core::error::ErrorCode;
    use crate::core::traits::{Clock, UserStore};
    use crate::db::migrate::migrate;
    use crate::db::pool::SqlitePool;
    use crate::db::users::SqliteUserStore;
    use crate::engine::user::admin::UserAdminService;
    use crate::engine::user::model::{
        CreateUserInput, UpdateUserInput, UserQuery, UserRole, UserSort, UserState,
    };
    use crate::engine::user::token::TokenPepper;

    struct SeqClock(std::sync::atomic::AtomicI64);

    impl Clock for SeqClock {
        fn now_ms(&self) -> i64 {
            self.0.fetch_add(100, std::sync::atomic::Ordering::Relaxed)
        }
    }

    fn setup_store() -> (Arc<SqlitePool>, SqliteUserStore, SeqClock, TokenPepper) {
        let pool = Arc::new(SqlitePool::open_in_memory().expect("open mem"));
        migrate(&pool, 1_763_600_000_000).expect("migrate");
        let store = SqliteUserStore::new(Arc::clone(&pool));
        let clock = SeqClock(std::sync::atomic::AtomicI64::new(1_763_600_001_000));
        let pepper = TokenPepper::ephemeral().expect("pepper");
        (pool, store, clock, pepper)
    }

    #[test]
    fn sqlite_users_crud_pagination_and_no_id_collision_on_delete_recreate() {
        let (_pool, store, clock, pepper) = setup_store();
        let svc = UserAdminService::new(&store, &clock, &pepper);

        let (admin, admin_tok) = svc
            .create_user(CreateUserInput {
                name: String::from("主管理员"),
                group: String::from("系统"),
                role: UserRole::Admin,
                daily_requests: 0,
                monthly_tokens: 0,
            })
            .expect("create admin");
        assert!(admin_tok.starts_with("jg_"));
        assert!(admin.token_hint.starts_with("jg_"));

        let (m1, m1_tok) = svc
            .create_user(CreateUserInput {
                name: String::from("100%_测试"),
                group: String::from("研发"),
                role: UserRole::Member,
                daily_requests: 500,
                monthly_tokens: 2_000_000,
            })
            .expect("create m1");

        // 同组同名冲突拦截
        let dup = svc.create_user(CreateUserInput {
            name: String::from("100%_测试"),
            group: String::from("研发"),
            role: UserRole::Member,
            daily_requests: 100,
            monthly_tokens: 100,
        });
        assert_eq!(dup.expect_err("dup conflict").code, ErrorCode::Conflict);

        // 轮换凭证并验证旧哈希失效、新哈希命中
        let (rotated, new_tok) = svc.rotate_token(&m1.id).expect("rotate");
        assert_ne!(rotated.token_hash, m1.token_hash);
        assert_ne!(new_tok, m1_tok);
        assert!(store
            .get_by_token_hash(&pepper.hash_token(&m1_tok))
            .expect("old lookup")
            .is_none());
        assert_eq!(
            store
                .get_by_token_hash(&pepper.hash_token(&new_tok))
                .expect("new lookup")
                .expect("found")
                .id,
            m1.id
        );

        // 删除后重建，CSPRNG ID 不碰撞
        svc.delete_user(&m1.id).expect("delete m1");
        let (m1_reborn, _) = svc
            .create_user(CreateUserInput {
                name: String::from("100%_测试"),
                group: String::from("研发"),
                role: UserRole::Member,
                daily_requests: 500,
                monthly_tokens: 2_000_000,
            })
            .expect("recreate");
        assert_ne!(m1_reborn.id, m1.id);

        let (_m2, _) = svc
            .create_user(CreateUserInput {
                name: String::from("普通成员B"),
                group: String::from("售前"),
                role: UserRole::Member,
                daily_requests: 200,
                monthly_tokens: 1_000_000,
            })
            .expect("create m2");

        // SQL LIKE 通配符安全转义搜索
        let search_pct = svc
            .query_users(&UserQuery {
                search: Some(String::from("100%_")),
                ..Default::default()
            })
            .expect("search literal %_");
        assert_eq!(search_pct.total, 1);
        assert_eq!(search_pct.items[0].id, m1_reborn.id);

        // 分页校验
        let p1 = svc
            .query_users(&UserQuery {
                page: 1,
                page_size: 2,
                sort: UserSort::CreatedDesc,
                ..Default::default()
            })
            .expect("page 1");
        let p2 = svc
            .query_users(&UserQuery {
                page: 2,
                page_size: 2,
                sort: UserSort::CreatedDesc,
                ..Default::default()
            })
            .expect("page 2");
        assert_eq!(p1.total, 3);
        assert_eq!(p1.items.len(), 2);
        assert_eq!(p2.items.len(), 1);

        let metrics = svc.metrics().expect("metrics");
        assert_eq!(metrics.total_users, 3);
        assert_eq!(metrics.active_users, 3);
        assert_eq!(metrics.active_admin_users, 1);
        assert_eq!(metrics.total_monthly_quota, 3_000_000);
        assert!(svc.delete_user(&admin.id).is_err());
    }

    #[test]
    fn last_admin_protection_holds_under_concurrent_updates_and_batch_disable() {
        let tmp = std::env::temp_dir().join(format!(
            "jiagou_concurrent_{}.db",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&tmp);
        let pool = Arc::new(SqlitePool::open(&tmp).expect("open file sqlite"));
        migrate(&pool, 1_763_600_000_000).expect("migrate");

        let store = Arc::new(SqliteUserStore::new(Arc::clone(&pool)));
        let clock = SeqClock(std::sync::atomic::AtomicI64::new(1_763_600_001_000));
        let pepper = TokenPepper::ephemeral().expect("pepper");
        let svc = UserAdminService::new(store.as_ref(), &clock, &pepper);

        let (a1, _) = svc
            .create_user(CreateUserInput {
                name: String::from("Admin1"),
                group: String::from("Sys"),
                role: UserRole::Admin,
                daily_requests: 0,
                monthly_tokens: 0,
            })
            .expect("a1");
        let (a2, _) = svc
            .create_user(CreateUserInput {
                name: String::from("Admin2"),
                group: String::from("Sys"),
                role: UserRole::Admin,
                daily_requests: 0,
                monthly_tokens: 0,
            })
            .expect("a2");

        // 批量试图同时停用全部两个管理员必须失败且保持原子回滚
        let batch_err = svc
            .batch_set_state(
                &[a1.id.as_str().to_string(), a2.id.as_str().to_string()],
                UserState::Disabled,
            )
            .expect_err("cannot disable all active admins");
        assert_eq!(batch_err.code, ErrorCode::Conflict);

        // 两个线程同时各自禁用 a1 / a2，BEGIN IMMEDIATE 确保只能成功 1 个
        let s1 = Arc::clone(&store);
        let s2 = Arc::clone(&store);
        let id1 = a1.id.clone();
        let id2 = a2.id.clone();

        let (r1, r2) = std::thread::scope(|scope| {
            let h1 = scope.spawn(move || {
                s1.update_atomic(
                    &id1,
                    &UpdateUserInput {
                        name: None,
                        group: None,
                        role: None,
                        state: Some(UserState::Disabled),
                        daily_requests: None,
                        monthly_tokens: None,
                    },
                )
            });
            let h2 = scope.spawn(move || {
                s2.update_atomic(
                    &id2,
                    &UpdateUserInput {
                        name: None,
                        group: None,
                        role: None,
                        state: Some(UserState::Disabled),
                        daily_requests: None,
                        monthly_tokens: None,
                    },
                )
            });
            (h1.join().expect("t1"), h2.join().expect("t2"))
        });

        assert!(r1.is_ok() ^ r2.is_ok(), "exactly one disable should succeed");
        let metrics = store.metrics().expect("metrics");
        assert_eq!(metrics.active_admin_users, 1);

        drop(store);
        drop(pool);
        let _ = std::fs::remove_file(&tmp);
        let _ = std::fs::remove_file(tmp.with_extension("db-wal"));
        let _ = std::fs::remove_file(tmp.with_extension("db-shm"));
    }
}
