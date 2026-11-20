use std::collections::HashSet;

use crate::core::error::EngineError;
use crate::core::ids::UserId;
use crate::core::traits::{Clock, UserStore};
use crate::engine::user::model::{
    CreateUserInput, UpdateUserInput, UserMetrics, UserPage, UserQuery, UserQuota, UserRecord,
    UserState,
};
use crate::engine::user::token::{generate_token, TokenPepper};

pub struct UserAdminService<'a, S: UserStore, C: Clock> {
    store: &'a S,
    clock: &'a C,
    pepper: &'a TokenPepper,
}

impl<'a, S: UserStore, C: Clock> UserAdminService<'a, S, C> {
    pub fn new(store: &'a S, clock: &'a C, pepper: &'a TokenPepper) -> Self {
        Self {
            store,
            clock,
            pepper,
        }
    }

    pub fn query_users(&self, query: &UserQuery) -> Result<UserPage, EngineError> {
        self.store.query(query)
    }

    pub fn get_user(&self, id: &UserId) -> Result<UserRecord, EngineError> {
        self.store
            .get_by_id(id)?
            .ok_or_else(|| EngineError::not_found("user not found"))
    }

    pub fn create_user(&self, input: CreateUserInput) -> Result<(UserRecord, String), EngineError> {
        let name = normalize_required(&input.name, "name")?;
        let group = normalize_required(&input.group, "group")?;
        let now = self.clock.now_ms();
        let id = UserId::generate()?;
        let (plain_token, token_hint) = generate_token()?;
        let token_hash = self.pepper.hash_token(&plain_token);

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
            token_hint,
            created_at_ms: now,
            last_active_ms: 0,
        };

        self.store.insert(&record)?;
        Ok((record, plain_token))
    }

    pub fn update_user(
        &self,
        id: &UserId,
        input: UpdateUserInput,
    ) -> Result<UserRecord, EngineError> {
        let normalized = UpdateUserInput {
            name: match input.name {
                Some(raw) => Some(normalize_required(&raw, "name")?),
                None => None,
            },
            group: match input.group {
                Some(raw) => Some(normalize_required(&raw, "group")?),
                None => None,
            },
            role: input.role,
            state: input.state,
            daily_requests: input.daily_requests,
            monthly_tokens: input.monthly_tokens,
        };
        self.store.update_atomic(id, &normalized)
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

    pub fn batch_set_state(&self, ids: &[String], state: UserState) -> Result<usize, EngineError> {
        let mut seen = HashSet::with_capacity(ids.len());
        let mut unique_ids = Vec::with_capacity(ids.len());
        for raw in ids {
            let trimmed = raw.trim();
            if !trimmed.is_empty() && seen.insert(trimmed.to_string()) {
                unique_ids.push(UserId::new(trimmed));
            }
        }
        if unique_ids.is_empty() {
            return Err(EngineError::invalid_input("ids cannot be empty"));
        }
        self.store.batch_set_state_atomic(&unique_ids, state)
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
        let (plain_token, token_hint) = generate_token()?;
        let token_hash = self.pepper.hash_token(&plain_token);
        let updated = self
            .store
            .rotate_token_atomic(id, &token_hash, &token_hint)?;
        Ok((updated, plain_token))
    }

    pub fn delete_user(&self, id: &UserId) -> Result<(), EngineError> {
        self.store.delete_atomic(id)
    }

    pub fn metrics(&self) -> Result<UserMetrics, EngineError> {
        self.store.metrics()
    }
}

fn normalize_required(raw: &str, field: &str) -> Result<String, EngineError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(EngineError::invalid_input(format!(
            "{field} cannot be empty"
        )));
    }
    if trimmed.chars().count() > 64 {
        return Err(EngineError::invalid_input(format!(
            "{field} exceeds 64 chars"
        )));
    }
    Ok(trimmed.to_string())
}
