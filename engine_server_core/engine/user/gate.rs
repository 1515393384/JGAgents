use crate::core::error::EngineError;
use crate::core::traits::UserStore;
use crate::engine::user::model::{UserRecord, UserRole, UserState};
use crate::engine::user::token::TokenPepper;

pub struct TokenGate<'a, S: UserStore> {
    store: &'a S,
    pepper: &'a TokenPepper,
}

impl<'a, S: UserStore> TokenGate<'a, S> {
    pub fn new(store: &'a S, pepper: &'a TokenPepper) -> Self {
        Self { store, pepper }
    }

    pub fn authenticate(&self, bearer_token: &str) -> Result<UserRecord, EngineError> {
        let token = bearer_token.trim();
        if token.is_empty() {
            return Err(EngineError::unauthorized("missing token"));
        }
        let hash = self.pepper.hash_token(token);
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

#[cfg(test)]
mod tests {
    use crate::core::error::ErrorCode;
    use crate::core::ids::UserId;
    use crate::db::users::InMemoryUserStore;
    use crate::engine::user::gate::TokenGate;
    use crate::engine::user::model::{UserQuota, UserRecord, UserRole, UserState};
    use crate::engine::user::token::TokenPepper;

    fn sample_user(
        id: &str,
        role: UserRole,
        state: UserState,
        used_requests: u64,
        token_plain: &str,
        pepper: &TokenPepper,
    ) -> UserRecord {
        UserRecord {
            id: UserId::new(id),
            name: id.to_string(),
            group: String::from("dev"),
            role,
            state,
            quota: UserQuota {
                daily_requests: 10,
                monthly_tokens: 1000,
                used_requests,
                used_tokens: 0,
            },
            token_hash: pepper.hash_token(token_plain),
            token_hint: String::from("jg_test"),
            created_at_ms: 100,
            last_active_ms: 0,
        }
    }

    #[test]
    fn token_gate_enforces_auth_roles_state_and_quota() {
        let pepper = TokenPepper::ephemeral().expect("pepper");
        let other_pepper = TokenPepper::ephemeral().expect("other pepper");

        let admin = sample_user("u_adm", UserRole::Admin, UserState::Active, 0, "tok_adm", &pepper);
        let member = sample_user("u_mem", UserRole::Member, UserState::Active, 10, "tok_mem", &pepper);
        let disabled = sample_user("u_dis", UserRole::Admin, UserState::Disabled, 0, "tok_dis", &pepper);

        let store = InMemoryUserStore::new(vec![admin, member, disabled]);
        let gate = TokenGate::new(&store, &pepper);

        assert_eq!(gate.authenticate("").expect_err("empty").code, ErrorCode::Unauthorized);
        assert_eq!(gate.authenticate("tok_wrong").expect_err("wrong").code, ErrorCode::Unauthorized);

        // 更换 pepper 后同明文失效
        let wrong_gate = TokenGate::new(&store, &other_pepper);
        assert_eq!(
            wrong_gate.authenticate("tok_adm").expect_err("pepper mismatch").code,
            ErrorCode::Unauthorized
        );

        assert_eq!(gate.require_admin("tok_dis").expect_err("disabled").code, ErrorCode::Forbidden);
        assert_eq!(gate.require_admin("tok_mem").expect_err("member").code, ErrorCode::Forbidden);
        assert_eq!(gate.require_admin("tok_adm").expect("admin ok").id.as_str(), "u_adm");

        assert_eq!(gate.can_open_stream("tok_mem").expect_err("quota").code, ErrorCode::Budget);
        assert!(gate.can_open_stream("tok_adm").is_ok());
    }
}
