use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;

use crate::core::error::EngineError;
use crate::core::ids::UserId;
use crate::core::traits::{Clock, UserStore};
use crate::engine::user::admin::UserAdminService;
use crate::engine::user::model::{CreateUserInput, UpdateUserInput, UserRecord, UserRole, UserState};

pub struct AppState<S: UserStore, C: Clock> {
    pub store: Arc<S>,
    pub clock: Arc<C>,
}

impl<S: UserStore, C: Clock> Clone for AppState<S, C> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            clock: Arc::clone(&self.clock),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HealthRes {
    ok: bool,
    ver: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UserDto {
    id: String,
    name: String,
    group: String,
    role: &'static str,
    state: &'static str,
    daily_requests: u64,
    monthly_tokens: u64,
    used_requests: u64,
    used_tokens: u64,
    token_fingerprint: String,
    created_at: String,
    last_active_at: String,
}

#[derive(Serialize)]
struct UsersListRes {
    users: Vec<UserDto>,
}

#[derive(Serialize)]
struct UserRes {
    user: UserDto,
}

#[derive(Serialize)]
struct UserWithTokenRes {
    user: UserDto,
    token: String,
}

#[derive(Serialize)]
struct OkRes {
    ok: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MetricsRes {
    total_users: usize,
    active_users: usize,
    disabled_users: usize,
    admin_users: usize,
    total_monthly_quota: u64,
    total_used_tokens: u64,
    total_used_requests: u64,
}

#[derive(Serialize)]
struct ErrorRes {
    code: &'static str,
    message: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateUserReq {
    name: String,
    group: String,
    role: String,
    daily_requests: u64,
    monthly_tokens: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateUserReq {
    name: Option<String>,
    group: Option<String>,
    role: Option<String>,
    state: Option<String>,
    daily_requests: Option<u64>,
    monthly_tokens: Option<u64>,
}

#[derive(Deserialize)]
struct SetStateReq {
    state: String,
}

struct ApiError(EngineError);

impl From<EngineError> for ApiError {
    fn from(value: EngineError) -> Self {
        Self(value)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.0.http_status())
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let body = Json(ErrorRes {
            code: self.0.as_str(),
            message: self.0.message,
        });
        (status, body).into_response()
    }
}

fn format_date(ms: i64) -> String {
    if ms <= 0 {
        return String::from("1970-01-01");
    }
    let days = ms / 86_400_000;
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = y + if m <= 2 { 1 } else { 0 };
    format!("{year:04}-{m:02}-{d:02}")
}

fn to_dto(u: UserRecord) -> UserDto {
    let last_active_at = if u.last_active_ms <= 0 {
        String::from("尚未活跃")
    } else {
        format_date(u.last_active_ms)
    };
    UserDto {
        id: u.id.as_str().to_string(),
        name: u.name,
        group: u.group,
        role: u.role.as_str(),
        state: u.state.as_str(),
        daily_requests: u.quota.daily_requests,
        monthly_tokens: u.quota.monthly_tokens,
        used_requests: u.quota.used_requests,
        used_tokens: u.quota.used_tokens,
        token_fingerprint: u.token_hash.fingerprint().to_string(),
        created_at: format_date(u.created_at_ms),
        last_active_at,
    }
}

pub fn build_router<S, C>(state: AppState<S, C>) -> Router
where
    S: UserStore + 'static,
    C: Clock + 'static,
{
    Router::new()
        .route("/healthz", get(healthz))
        .route("/api/admin/users", get(list_users::<S, C>).post(create_user::<S, C>))
        .route(
            "/api/admin/users/{id}",
            patch(update_user::<S, C>).delete(delete_user::<S, C>),
        )
        .route("/api/admin/users/{id}/state", patch(set_user_state::<S, C>))
        .route("/api/admin/users/{id}/token", post(rotate_token::<S, C>))
        .route("/api/admin/metrics", get(user_metrics::<S, C>))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn healthz() -> Json<HealthRes> {
    Json(HealthRes { ok: true, ver: 1 })
}

async fn list_users<S: UserStore, C: Clock>(
    State(state): State<AppState<S, C>>,
) -> Result<Json<UsersListRes>, ApiError> {
    let svc = UserAdminService::new(state.store.as_ref(), state.clock.as_ref());
    let users = svc.list_users()?.into_iter().map(to_dto).collect();
    Ok(Json(UsersListRes { users }))
}

async fn create_user<S: UserStore, C: Clock>(
    State(state): State<AppState<S, C>>,
    Json(req): Json<CreateUserReq>,
) -> Result<Json<UserWithTokenRes>, ApiError> {
    let role = UserRole::parse(&req.role)
        .ok_or_else(|| EngineError::invalid_input("invalid role"))?;
    let svc = UserAdminService::new(state.store.as_ref(), state.clock.as_ref());
    let (user, token) = svc.create_user(CreateUserInput {
        name: req.name,
        group: req.group,
        role,
        daily_requests: req.daily_requests,
        monthly_tokens: req.monthly_tokens,
    })?;
    Ok(Json(UserWithTokenRes {
        user: to_dto(user),
        token,
    }))
}

async fn update_user<S: UserStore, C: Clock>(
    State(state): State<AppState<S, C>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateUserReq>,
) -> Result<Json<UserRes>, ApiError> {
    let role = match req.role {
        Some(raw) => Some(
            UserRole::parse(&raw).ok_or_else(|| EngineError::invalid_input("invalid role"))?,
        ),
        None => None,
    };
    let next_state = match req.state {
        Some(raw) => Some(
            UserState::parse(&raw).ok_or_else(|| EngineError::invalid_input("invalid state"))?,
        ),
        None => None,
    };
    let svc = UserAdminService::new(state.store.as_ref(), state.clock.as_ref());
    let user = svc.update_user(
        &UserId::new(id),
        UpdateUserInput {
            name: req.name,
            group: req.group,
            role,
            state: next_state,
            daily_requests: req.daily_requests,
            monthly_tokens: req.monthly_tokens,
        },
    )?;
    Ok(Json(UserRes { user: to_dto(user) }))
}

async fn set_user_state<S: UserStore, C: Clock>(
    State(state): State<AppState<S, C>>,
    Path(id): Path<String>,
    Json(req): Json<SetStateReq>,
) -> Result<Json<UserRes>, ApiError> {
    let next_state = UserState::parse(&req.state)
        .ok_or_else(|| EngineError::invalid_input("invalid state"))?;
    let svc = UserAdminService::new(state.store.as_ref(), state.clock.as_ref());
    let user = svc.set_state(&UserId::new(id), next_state)?;
    Ok(Json(UserRes { user: to_dto(user) }))
}

async fn rotate_token<S: UserStore, C: Clock>(
    State(state): State<AppState<S, C>>,
    Path(id): Path<String>,
) -> Result<Json<UserWithTokenRes>, ApiError> {
    let svc = UserAdminService::new(state.store.as_ref(), state.clock.as_ref());
    let (user, token) = svc.rotate_token(&UserId::new(id))?;
    Ok(Json(UserWithTokenRes {
        user: to_dto(user),
        token,
    }))
}

async fn delete_user<S: UserStore, C: Clock>(
    State(state): State<AppState<S, C>>,
    Path(id): Path<String>,
) -> Result<Json<OkRes>, ApiError> {
    let svc = UserAdminService::new(state.store.as_ref(), state.clock.as_ref());
    svc.delete_user(&UserId::new(id))?;
    Ok(Json(OkRes { ok: true }))
}

async fn user_metrics<S: UserStore, C: Clock>(
    State(state): State<AppState<S, C>>,
) -> Result<Json<MetricsRes>, ApiError> {
    let svc = UserAdminService::new(state.store.as_ref(), state.clock.as_ref());
    let m = svc.metrics()?;
    Ok(Json(MetricsRes {
        total_users: m.total_users,
        active_users: m.active_users,
        disabled_users: m.disabled_users,
        admin_users: m.admin_users,
        total_monthly_quota: m.total_monthly_quota,
        total_used_tokens: m.total_used_tokens,
        total_used_requests: m.total_used_requests,
    }))
}
