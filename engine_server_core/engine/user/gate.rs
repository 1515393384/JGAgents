use crate::core::error::EngineError;
use crate::core::traits::UserStore;
use crate::engine::user::admin::hash_token;
use crate::engine::user::model::{UserRecord, UserRole, UserState};

pub struct TokenGate<'a, S: UserStore> {
    store: &'a S,
}

impl<'a, S: UserStore> TokenGate<'a, S> {
    pub fn new(store: &'a S) -> Self {
        Self { store }
    }

    pub fn authenticate(&self, bearer_token: &str) -> Result<UserRecord, EngineError> {
        let token = bearer_token.trim();
        if token.is_empty() {
            return Err(EngineError::unauthorized("missing token"));
        }
        let hash = hash_token(token);
        let user = self
            .store
            .get_by_token_hash(&hash)?
            .ok_or_else(|| EngineError::unauthorized("invalid token"))?;

        if user.state != UserState::Active {
            return Err(EngineError::forbidden("user disabled"));
        }
        Ok(user)
    }

    pub fn require_admin(&self, bearer_token: &str) -> Result<UserRecord, EngineError> {
        let user = self.authenticate(bearer_token)?;
        if user.role != UserRole::Admin {
            return Err(EngineError::forbidden("admin role required"));
        }
        Ok(user)
    }

    pub fn can_open_stream(&self, bearer_token: &str) -> Result<UserRecord, EngineError> {
        let user = self.authenticate(bearer_token)?;
        if !user.quota.has_remaining() {
            return Err(EngineError::budget("quota exhausted"));
        }
        Ok(user)
    }
}
