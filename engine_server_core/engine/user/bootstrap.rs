use std::path::Path;

use crate::core::error::EngineError;
use crate::core::traits::{Clock, UserStore};
use crate::engine::user::admin::UserAdminService;
use crate::engine::user::model::{CreateUserInput, UserRecord, UserRole};
use crate::engine::user::token::{write_restricted_file, TokenPepper};

pub struct BootstrapAdmin {
    pub record: UserRecord,
    pub one_time_token: String,
}

pub fn ensure_bootstrap_admin<S: UserStore, C: Clock>(
    store: &S,
    clock: &C,
    pepper: &TokenPepper,
    token_output_path: Option<&Path>,
) -> Result<Option<BootstrapAdmin>, EngineError> {
    if !store.is_empty()? {
        return Ok(None);
    }

    let svc = UserAdminService::new(store, clock, pepper);
    let (record, one_time_token) = svc.create_user(CreateUserInput {
        name: String::from("admin"),
        group: String::from("system"),
        role: UserRole::Admin,
        daily_requests: 0,
        monthly_tokens: 0,
    })?;

    if let Some(path) = token_output_path {
        write_restricted_file(path, format!("{one_time_token}\n").as_bytes())?;
    }

    Ok(Some(BootstrapAdmin {
        record,
        one_time_token,
    }))
}
