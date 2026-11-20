use crate::core::error::EngineError;
use crate::core::ids::{TokenHash, UserId};
use crate::core::traits::{Clock, UserStore};
use crate::engine::user::model::{
    CreateUserInput, UpdateUserInput, UserMetrics, UserQuota, UserRecord, UserRole, UserState,
};

pub struct UserAdminService<'a, S: UserStore, C: Clock> {
    store: &'a S,
    clock: &'a C,
}

impl<'a, S: UserStore, C: Clock> UserAdminService<'a, S, C> {
    pub fn new(store: &'a S, clock: &'a C) -> Self {
        Self { store, clock }
    }

    pub fn list_users(&self) -> Result<Vec<UserRecord>, EngineError> {
        let mut users = self.store.list_all()?;
        users.sort_by(|a, b| b.created_at_ms.cmp(&a.created_at_ms));
        Ok(users)
    }

    pub fn get_user(&self, id: &UserId) -> Result<UserRecord, EngineError> {
        self.store
            .get_by_id(id)?
            .ok_or_else(|| EngineError::not_found("user not found"))
    }

    pub fn create_user(&self, input: CreateUserInput) -> Result<(UserRecord, String), EngineError> {
        let name = normalize_required(&input.name, "name")?;
        let group = normalize_required(&input.group, "group")?;
        let existing = self.store.list_all()?;
        if existing.iter().any(|u| u.name == name && u.group == group) {
            return Err(EngineError::conflict("user already exists in group"));
        }

        let now = self.clock.now_ms();
        let seq = existing.len() as u64 + 1;
        let id = UserId::new(format!("u_{:08x}{:04x}", now.max(0) as u64, seq));
        let plain_token = generate_token(id.as_str(), now);
        let token_hash = hash_token(&plain_token);

        let record = UserRecord {
            id,
            name,
            group,
            role: input.role,
            state: UserState::Active,
            quota: UserQuota {
                daily_requests: input.daily_requests,
                monthly_tokens: input.monthly_tokens,
                used_requests: 0,
                used_tokens: 0,
            },
            token_hash,
            created_at_ms: now,
            last_active_ms: 0,
        };

        self.store.insert(&record)?;
        Ok((record, plain_token))
    }

    pub fn update_user(&self, id: &UserId, input: UpdateUserInput) -> Result<UserRecord, EngineError> {
        let mut user = self.get_user(id)?;
        let all = self.store.list_all()?;

        if let Some(name) = input.name {
            user.name = normalize_required(&name, "name")?;
        }
        if let Some(group) = input.group {
            user.group = normalize_required(&group, "group")?;
        }

        let next_role = input.role.unwrap_or(user.role);
        let next_state = input.state.unwrap_or(user.state);
        if is_last_active_admin(&all, &user.id)
            && (next_role != UserRole::Admin || next_state != UserState::Active)
        {
            return Err(EngineError::conflict("cannot disable or demote last active admin"));
        }

        user.role = next_role;
        user.state = next_state;

        if let Some(daily_requests) = input.daily_requests {
            user.quota.daily_requests = daily_requests;
        }
        if let Some(monthly_tokens) = input.monthly_tokens {
            user.quota.monthly_tokens = monthly_tokens;
        }

        self.store.update(&user)?;
        Ok(user)
    }

    pub fn set_state(&self, id: &UserId, state: UserState) -> Result<UserRecord, EngineError> {
        self.update_user(
            id,
            UpdateUserInput {
                name: None,
                group: None,
                role: None,
                state: Some(state),
                daily_requests: None,
                monthly_tokens: None,
            },
        )
    }

    pub fn update_quota(
        &self,
        id: &UserId,
        daily_requests: u64,
        monthly_tokens: u64,
    ) -> Result<UserRecord, EngineError> {
        self.update_user(
            id,
            UpdateUserInput {
                name: None,
                group: None,
                role: None,
                state: None,
                daily_requests: Some(daily_requests),
                monthly_tokens: Some(monthly_tokens),
            },
        )
    }

    pub fn rotate_token(&self, id: &UserId) -> Result<(UserRecord, String), EngineError> {
        let mut user = self.get_user(id)?;
        let now = self.clock.now_ms();
        let plain_token = generate_token(user.id.as_str(), now);
        user.token_hash = hash_token(&plain_token);
        self.store.update(&user)?;
        Ok((user, plain_token))
    }

    pub fn delete_user(&self, id: &UserId) -> Result<(), EngineError> {
        let all = self.store.list_all()?;
        if !all.iter().any(|u| &u.id == id) {
            return Err(EngineError::not_found("user not found"));
        }
        if is_last_active_admin(&all, id) {
            return Err(EngineError::conflict("cannot delete last active admin"));
        }
        let deleted = self.store.delete(id)?;
        if !deleted {
            return Err(EngineError::not_found("user not found"));
        }
        Ok(())
    }

    pub fn metrics(&self) -> Result<UserMetrics, EngineError> {
        let all = self.store.list_all()?;
        let mut metrics = UserMetrics {
            total_users: all.len(),
            active_users: 0,
            disabled_users: 0,
            admin_users: 0,
            total_monthly_quota: 0,
            total_used_tokens: 0,
            total_used_requests: 0,
        };
        for u in &all {
            match u.state {
                UserState::Active => metrics.active_users += 1,
                UserState::Disabled => metrics.disabled_users += 1,
            }
            if u.role == UserRole::Admin {
                metrics.admin_users += 1;
            }
            metrics.total_monthly_quota += u.quota.monthly_tokens;
            metrics.total_used_tokens += u.quota.used_tokens;
            metrics.total_used_requests += u.quota.used_requests;
        }
        Ok(metrics)
    }
}

fn normalize_required(raw: &str, field: &str) -> Result<String, EngineError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(EngineError::invalid_input(format!("{field} cannot be empty")));
    }
    if trimmed.chars().count() > 64 {
        return Err(EngineError::invalid_input(format!("{field} exceeds 64 chars")));
    }
    Ok(trimmed.to_string())
}

fn is_last_active_admin(all: &[UserRecord], target: &UserId) -> bool {
    let active_admins: Vec<&UserRecord> = all
        .iter()
        .filter(|u| u.role == UserRole::Admin && u.state == UserState::Active)
        .collect();
    active_admins.len() == 1 && &active_admins[0].id == target
}

pub fn hash_token(plain: &str) -> TokenHash {
    let mut h1: u64 = 0xcbf29ce484222325;
    let mut h2: u64 = 0x84222325cbf29ce4;
    for &b in plain.as_bytes() {
        h1 ^= b as u64;
        h1 = h1.wrapping_mul(0x100000001b3);
        h2 = h2.wrapping_mul(0x9e3779b97f4a7c15) ^ (b as u64);
    }
    TokenHash::from_hex(format!("{:016x}{:016x}", h1, h2))
}

fn generate_token(user_id: &str, now_ms: i64) -> String {
    let seed = hash_token(&format!("{user_id}:{now_ms}"));
    format!("jg_{}", seed.as_str())
}
