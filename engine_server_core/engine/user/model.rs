use crate::core::ids::{TokenHash, UserId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UserRole {
    Admin,
    Member,
}

impl UserRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::Member => "member",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "admin" => Some(Self::Admin),
            "member" => Some(Self::Member),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UserState {
    Active,
    Disabled,
}

impl UserState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Disabled => "disabled",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "active" => Some(Self::Active),
            "disabled" => Some(Self::Disabled),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UserQuota {
    pub daily_requests: u64,
    pub monthly_tokens: u64,
    pub used_requests: u64,
    pub used_tokens: u64,
}

impl UserQuota {
    pub fn has_remaining(&self) -> bool {
        let req_ok = self.daily_requests == 0 || self.used_requests < self.daily_requests;
        let tok_ok = self.monthly_tokens == 0 || self.used_tokens < self.monthly_tokens;
        req_ok && tok_ok
    }

    pub fn usage_ratio(&self) -> f64 {
        if self.monthly_tokens == 0 {
            return 0.0;
        }
        (self.used_tokens as f64 / self.monthly_tokens as f64).min(1.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserRecord {
    pub id: UserId,
    pub name: String,
    pub group: String,
    pub role: UserRole,
    pub state: UserState,
    pub quota: UserQuota,
    pub token_hash: TokenHash,
    pub created_at_ms: i64,
    pub last_active_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateUserInput {
    pub name: String,
    pub group: String,
    pub role: UserRole,
    pub daily_requests: u64,
    pub monthly_tokens: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateUserInput {
    pub name: Option<String>,
    pub group: Option<String>,
    pub role: Option<UserRole>,
    pub state: Option<UserState>,
    pub daily_requests: Option<u64>,
    pub monthly_tokens: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserMetrics {
    pub total_users: usize,
    pub active_users: usize,
    pub disabled_users: usize,
    pub admin_users: usize,
    pub total_monthly_quota: u64,
    pub total_used_tokens: u64,
    pub total_used_requests: u64,
}
