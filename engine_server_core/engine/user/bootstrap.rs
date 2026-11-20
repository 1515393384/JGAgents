use crate::core::error::EngineError;
use crate::core::traits::{Clock, UserStore};
use crate::engine::user::admin::UserAdminService;
use crate::engine::user::model::{CreateUserInput, UserRecord, UserRole};

pub struct BootstrapAdmin {
    pub record: UserRecord,
    pub one_time_token: String,
}

pub fn ensure_bootstrap_admin<S: UserStore, C: Clock>(
    store: &S,
    clock: &C,
) -> Result<Option<BootstrapAdmin>, EngineError> {
    let existing = store.list_all()?;
    if !existing.is_empty() {
        return Ok(None);
    }

    let svc = UserAdminService::new(store, clock);
    let (record, one_time_token) = svc.create_user(CreateUserInput {
        name: String::from("admin"),
        group: String::from("system"),
        role: UserRole::Admin,
        daily_requests: 0,
        monthly_tokens: 0,
    })?;

    Ok(Some(BootstrapAdmin {
        record,
        one_time_token,
    }))
}
