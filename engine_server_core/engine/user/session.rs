use crate::core::error::EngineError;
use crate::core::ids::UserId;
use crate::core::traits::{Clock, SessionStore, UserStore};
use crate::engine::user::model::{SessionRecord, UserQuota, UserRecord, UserRole, UserState};
use crate::engine::user::password::{hash_password, verify_password};
use crate::engine::user::token::{generate_session_token, generate_token, TokenPepper};

pub const MIN_PASSWORD_CHARS: usize = 12;
pub const MAX_PASSWORD_CHARS: usize = 256;
pub const EPHEMERAL_SESSION_TTL_MS: i64 = 86_400_000;
pub const REMEMBER_SESSION_TTL_MS: i64 = 30 * 86_400_000;
pub const REMEMBER_MAX_AGE_SECS: i64 = 30 * 86_400;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IssuedSession {
    pub plain_token: String,
    pub max_age_secs: Option<i64>,
}

pub struct AuthSessionService<'a, U: UserStore, S: SessionStore, C: Clock> {
    users: &'a U,
    sessions: &'a S,
    clock: &'a C,
    pepper: &'a TokenPepper,
}

impl<'a, U: UserStore, S: SessionStore, C: Clock> AuthSessionService<'a, U, S, C> {
    pub fn new(
        users: &'a U,
        sessions: &'a S,
        clock: &'a C,
        pepper: &'a TokenPepper,
    ) -> Self {
        Self {
            users,
            sessions,
            clock,
            pepper,
        }
    }

    pub fn register(
        &self,
        username: &str,
        password: &str,
        remember: bool,
    ) -> Result<(UserRecord, IssuedSession), EngineError> {
        let name = validate_username(username)?;
        validate_new_password(password)?;

        let password_hash = hash_password(password)?;
        let now = self.clock.now_ms();
        let id = UserId::generate()?;
        let (api_token, token_hint) = generate_token()?;
        let token_hash = self.pepper.hash_token(&api_token);

        let record = UserRecord {
            id,
            name,
            group: String::from("个人"),
            role: UserRole::Member,
            state: UserState::Active,
            quota: UserQuota {
                daily_requests: 500,
                monthly_tokens: 2_000_000,
                used_requests: 0,
                used_tokens: 0,
            },
            password_hash,
            token_hash,
            token_hint,
            created_at_ms: now,
            last_active_ms: now,
        };

        self.users.register_member_atomic(&record)?;
        let issued = self.issue_session(&record.id, now, remember)?;
        Ok((record, issued))
    }

    pub fn login(
        &self,
        username: &str,
        password: &str,
        remember: bool,
    ) -> Result<(UserRecord, IssuedSession), EngineError> {
        let name = username.trim();
        if name.is_empty() || password.is_empty() {
            return Err(EngineError::unauthorized("用户名或密码不能为空"));
        }

        let candidates = self.users.find_by_name(name)?;
        let mut matched: Option<UserRecord> = None;
        for user in candidates {
            if !user.password_hash.is_empty() && verify_password(password, &user.password_hash) {
                matched = Some(user);
                break;
            }
        }

        let mut user =
            matched.ok_or_else(|| EngineError::unauthorized("用户名或密码错误"))?;

        if user.state != UserState::Active {
            return Err(EngineError::forbidden("账号已停用，请联系管理员"));
        }

        let now = self.clock.now_ms();
        self.users.touch_last_active(&user.id, now)?;
        user.last_active_ms = now;

        let issued = self.issue_session(&user.id, now, remember)?;
        Ok((user, issued))
    }

    pub fn current_user(&self, session_token: &str) -> Result<UserRecord, EngineError> {
        let token = session_token.trim();
        if token.is_empty() {
            return Err(EngineError::unauthorized("未登录"));
        }
        let token_hash = self.pepper.hash_session_token(token);
        let now = self.clock.now_ms();

        let sess = self
            .sessions
            .get_valid(&token_hash, now)?
            .ok_or_else(|| EngineError::unauthorized("登录已过期，请重新登录"))?;

        let Some(user) = self.users.get_by_id(&sess.user_id)? else {
            let _ = self.sessions.delete(&token_hash);
            return Err(EngineError::unauthorized("账号不存在"));
        };

        if user.state != UserState::Active {
            let _ = self.sessions.delete_for_user(&user.id);
            return Err(EngineError::forbidden("账号已停用，请联系管理员"));
        }

        Ok(user)
    }

    pub fn logout(&self, session_token: &str) -> Result<(), EngineError> {
        let token = session_token.trim();
        if token.is_empty() {
            return Ok(());
        }
        let token_hash = self.pepper.hash_session_token(token);
        self.sessions.delete(&token_hash)
    }

    fn issue_session(
        &self,
        user_id: &UserId,
        now_ms: i64,
        remember: bool,
    ) -> Result<IssuedSession, EngineError> {
        let plain_token = generate_session_token()?;
        let token_hash = self.pepper.hash_session_token(&plain_token);
        let ttl_ms = if remember {
            REMEMBER_SESSION_TTL_MS
        } else {
            EPHEMERAL_SESSION_TTL_MS
        };
        let record = SessionRecord {
            token_hash,
            user_id: user_id.clone(),
            created_at_ms: now_ms,
            expires_at_ms: now_ms.saturating_add(ttl_ms),
        };
        self.sessions.create(&record)?;
        Ok(IssuedSession {
            plain_token,
            max_age_secs: if remember {
                Some(REMEMBER_MAX_AGE_SECS)
            } else {
                None
            },
        })
    }
}

fn validate_username(raw: &str) -> Result<String, EngineError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(EngineError::invalid_input("用户名不能为空"));
    }
    if trimmed.chars().count() > 64 {
        return Err(EngineError::invalid_input("用户名不能超过 64 个字符"));
    }
    if trimmed.chars().any(|c| c.is_control()) {
        return Err(EngineError::invalid_input("用户名包含非法控制字符"));
    }
    Ok(trimmed.to_string())
}

fn validate_new_password(password: &str) -> Result<(), EngineError> {
    let len = password.chars().count();
    if len < MIN_PASSWORD_CHARS {
        return Err(EngineError::invalid_input("密码至少需要 12 位"));
    }
    if len > MAX_PASSWORD_CHARS {
        return Err(EngineError::invalid_input("密码长度超出限制"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicI64, Ordering};
    use std::sync::Arc;

    use crate::core::error::ErrorCode;
    use crate::core::traits::{Clock, UserStore};
    use crate::db::migrate::migrate;
    use crate::db::pool::SqlitePool;
    use crate::db::sessions::SqliteSessionStore;
    use crate::db::users::SqliteUserStore;
    use crate::engine::user::admin::UserAdminService;
    use crate::engine::user::model::{CreateUserInput, UserRole, UserState};
    use crate::engine::user::session::AuthSessionService;
    use crate::engine::user::token::TokenPepper;

    struct ManualClock(AtomicI64);

    impl Clock for ManualClock {
        fn now_ms(&self) -> i64 {
            self.0.load(Ordering::Relaxed)
        }
    }

    #[test]
    fn register_login_session_expiry_disable_and_logout_lifecycle() {
        let pool = Arc::new(SqlitePool::open_in_memory().expect("open mem"));
        migrate(&pool, 1_763_600_000_000).expect("migrate");

        let users = SqliteUserStore::new(Arc::clone(&pool));
        let sessions = SqliteSessionStore::new(Arc::clone(&pool));
        let clock = ManualClock(AtomicI64::new(1_763_600_000_000));
        let pepper = TokenPepper::ephemeral().expect("pepper");

        let auth = AuthSessionService::new(&users, &sessions, &clock, &pepper);

        // 短密码拒绝
        let short_err = auth
            .register("chen.mo", "short_pw", false)
            .expect_err("short password");
        assert_eq!(short_err.code, ErrorCode::InvalidInput);

        // 注册成功并立即签发 session
        let (created, reg_sess) = auth
            .register("chen.mo", "StrongPass_2026!", false)
            .expect("register");
        assert_eq!(created.name, "chen.mo");
        assert_eq!(created.role, UserRole::Member);
        assert!(reg_sess.plain_token.starts_with("sess_"));
        assert_eq!(reg_sess.max_age_secs, None);

        // 重复用户名拦截
        let dup = auth
            .register("chen.mo", "AnotherStrong_2026!", false)
            .expect_err("duplicate username");
        assert_eq!(dup.code, ErrorCode::Conflict);

        // 会话可读出当前用户
        let me = auth.current_user(&reg_sess.plain_token).expect("me");
        assert_eq!(me.id, created.id);

        // 错密码登录被拒
        let bad_pw = auth
            .login("chen.mo", "WrongPass_2026!", true)
            .expect_err("wrong password");
        assert_eq!(bad_pw.code, ErrorCode::Unauthorized);

        // 正确密码登录（remember = true）
        clock.0.fetch_add(10_000, Ordering::Relaxed);
        let (logged_in, rem_sess) = auth
            .login("chen.mo", "StrongPass_2026!", true)
            .expect("login");
        assert_eq!(logged_in.id, created.id);
        assert!(rem_sess.max_age_secs.is_some());

        // 登出后旧 session 失效
        auth.logout(&reg_sess.plain_token).expect("logout");
        assert_eq!(
            auth.current_user(&reg_sess.plain_token)
                .expect_err("logged out")
                .code,
            ErrorCode::Unauthorized
        );

        // 管理员禁用账号后，现存 session 立即失效且不可再登录
        let admin = UserAdminService::new(&users, &clock, &pepper);
        admin
            .set_state(&created.id, UserState::Disabled)
            .expect("disable user");
        assert_eq!(
            auth.current_user(&rem_sess.plain_token)
                .expect_err("disabled user session")
                .code,
            ErrorCode::Forbidden
        );
        assert_eq!(
            auth.login("chen.mo", "StrongPass_2026!", false)
                .expect_err("disabled login")
                .code,
            ErrorCode::Forbidden
        );

        // 纯管理员创建的无密码账号不能用空密码登录
        let (_api_only, api_tok) = admin
            .create_user(CreateUserInput {
                name: String::from("api-bot"),
                group: String::from("系统"),
                role: UserRole::Member,
                daily_requests: 100,
                monthly_tokens: 1000,
            })
            .expect("create api user");
        assert!(api_tok.starts_with("jg_"));
        assert_eq!(
            auth.login("api-bot", "", false)
                .expect_err("empty pw")
                .code,
            ErrorCode::Unauthorized
        );
        assert_eq!(
            auth.current_user(&api_tok)
                .expect_err("api token cannot be used as cookie session")
                .code,
            ErrorCode::Unauthorized
        );
        assert!(users.find_by_name("api-bot").expect("find").len() == 1);
    }
}
