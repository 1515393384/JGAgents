use std::collections::HashSet;
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
use crate::db::ledger::SqliteLedgerStore;
use crate::db::msgs::SqliteConvStore;
use crate::db::pool::SqlitePool;
use crate::engine::server::conn::{ConnRegistry, MAX_CONNS};
use crate::engine::server::health::{build_health_snapshot, HealthSnapshot};
use crate::engine::user::admin::UserAdminService;
use crate::engine::user::model::{
    CreateUserInput, UpdateUserInput, UserRecord, UserRole, UserState,
};

pub struct AppState<S: UserStore, C: Clock> {
    pub store: Arc<S>,
    pub clock: Arc<C>,
    pub pool: Option<Arc<SqlitePool>>,
    pub convs: Option<Arc<SqliteConvStore>>,
    pub ledger: Option<Arc<SqliteLedgerStore>>,
    pub conns: Arc<ConnRegistry>,
}

impl<S: UserStore, C: Clock> AppState<S, C> {
    pub fn in_memory(store: Arc<S>, clock: Arc<C>) -> Self {
        Self {
            store,
            clock,
            pool: None,
            convs: None,
            ledger: None,
            conns: Arc::new(ConnRegistry::default()),
        }
    }
}

impl<S: UserStore, C: Clock> Clone for AppState<S, C> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            clock: Arc::clone(&self.clock),
            pool: self.pool.clone(),
            convs: self.convs.clone(),
            ledger: self.ledger.clone(),
            conns: Arc::clone(&self.conns),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HealthRes {
    ok: bool,
    ver: u32,
    db_mode: String,
    schema_ver: i64,
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
#[serde(rename_all = "camelCase")]
struct TrendDto {
    day: String,
    requests: u64,
    hit_rate: u8,
    tokens_k: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConnectionDto {
    id: String,
    user_id: String,
    user_name: String,
    conv_id: String,
    ip: String,
    uptime: String,
    inflight: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ArchiveDto {
    conv_id: String,
    owner_name: String,
    group: String,
    messages: u64,
    prefix_version: u32,
    bytes_kb: u64,
    updated_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OverviewRes {
    metrics: MetricsRes,
    trend: Vec<TrendDto>,
    connections: Vec<ConnectionDto>,
    archives: Vec<ArchiveDto>,
    health: HealthSnapshot,
    max_conns: usize,
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

#[derive(Deserialize)]
struct BatchStateReq {
    ids: Vec<String>,
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

fn civil_from_ms(ms: i64) -> (i64, u64, u64) {
    if ms <= 0 {
        return (1970, 1, 1);
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
    (year, m, d)
}

fn format_date(ms: i64) -> String {
    let (year, m, d) = civil_from_ms(ms);
    format!("{year:04}-{m:02}-{d:02}")
}

fn format_month_day(ms: i64) -> String {
    let (_, m, d) = civil_from_ms(ms);
    format!("{m:02}/{d:02}")
}

fn format_relative(ms: i64, now_ms: i64, empty_label: &str) -> String {
    if ms <= 0 {
        return String::from(empty_label);
    }
    let diff_ms = (now_ms - ms).max(0);
    let mins = diff_ms / 60_000;
    if mins < 1 {
        return String::from("刚刚");
    }
    if mins < 60 {
        return format!("{mins} 分钟前");
    }
    let hours = mins / 60;
    if hours < 24 {
        return format!("{hours} 小时前");
    }
    format_date(ms)
}

fn to_dto(u: UserRecord, now_ms: i64) -> UserDto {
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
        last_active_at: format_relative(u.last_active_ms, now_ms, "尚未活跃"),
    }
}

pub fn build_router<S, C>(state: AppState<S, C>) -> Router
where
    S: UserStore + 'static,
    C: Clock + 'static,
{
    Router::new()
        .route("/healthz", get(healthz::<S, C>))
        .route(
            "/api/admin/users",
            get(list_users::<S, C>).post(create_user::<S, C>),
        )
        .route(
            "/api/admin/users/batch-state",
            patch(batch_set_state::<S, C>),
        )
        .route(
            "/api/admin/users/{id}",
            patch(update_user::<S, C>).delete(delete_user::<S, C>),
        )
        .route("/api/admin/users/{id}/state", patch(set_user_state::<S, C>))
        .route("/api/admin/users/{id}/token", post(rotate_token::<S, C>))
        .route("/api/admin/metrics", get(user_metrics::<S, C>))
        .route("/api/admin/overview", get(admin_overview::<S, C>))
        .route("/api/admin/health", get(admin_health::<S, C>))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn healthz<S: UserStore, C: Clock>(
    State(state): State<AppState<S, C>>,
) -> Result<Json<HealthRes>, ApiError> {
    let users = state.store.list_all()?;
    let active_admins = users
        .iter()
        .filter(|u| u.role == UserRole::Admin && u.state == UserState::Active)
        .count();
    let active_conns = state.conns.len().unwrap_or(0);
    let snap = build_health_snapshot(state.pool.as_deref(), active_admins, active_conns);
    Ok(Json(HealthRes {
        ok: snap.ok,
        ver: snap.ver,
        db_mode: snap.db_mode,
        schema_ver: snap.schema_ver,
    }))
}

async fn list_users<S: UserStore, C: Clock>(
    State(state): State<AppState<S, C>>,
) -> Result<Json<UsersListRes>, ApiError> {
    let now_ms = state.clock.now_ms();
    let svc = UserAdminService::new(state.store.as_ref(), state.clock.as_ref());
    let users = svc
        .list_users()?
        .into_iter()
        .map(|u| to_dto(u, now_ms))
        .collect();
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
    let now_ms = state.clock.now_ms();
    Ok(Json(UserWithTokenRes {
        user: to_dto(user, now_ms),
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
    let now_ms = state.clock.now_ms();
    Ok(Json(UserRes {
        user: to_dto(user, now_ms),
    }))
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
    let now_ms = state.clock.now_ms();
    Ok(Json(UserRes {
        user: to_dto(user, now_ms),
    }))
}

async fn batch_set_state<S: UserStore, C: Clock>(
    State(state): State<AppState<S, C>>,
    Json(req): Json<BatchStateReq>,
) -> Result<Json<UsersListRes>, ApiError> {
    let next_state = UserState::parse(&req.state)
        .ok_or_else(|| EngineError::invalid_input("invalid state"))?;
    let id_set: HashSet<String> = req.ids.into_iter().collect();
    let svc = UserAdminService::new(state.store.as_ref(), state.clock.as_ref());
    let all = svc.list_users()?;

    if next_state == UserState::Disabled {
        let remaining_admins = all
            .iter()
            .filter(|u| {
                u.role == UserRole::Admin
                    && u.state == UserState::Active
                    && !id_set.contains(u.id.as_str())
            })
            .count();
        if remaining_admins == 0 {
            return Err(
                EngineError::conflict("cannot disable all active admins").into(),
            );
        }
    }

    for id in &id_set {
        if all.iter().any(|u| u.id.as_str() == id) {
            svc.set_state(&UserId::new(id.clone()), next_state)?;
        }
    }

    let now_ms = state.clock.now_ms();
    let users = svc
        .list_users()?
        .into_iter()
        .map(|u| to_dto(u, now_ms))
        .collect();
    Ok(Json(UsersListRes { users }))
}

async fn rotate_token<S: UserStore, C: Clock>(
    State(state): State<AppState<S, C>>,
    Path(id): Path<String>,
) -> Result<Json<UserWithTokenRes>, ApiError> {
    let svc = UserAdminService::new(state.store.as_ref(), state.clock.as_ref());
    let (user, token) = svc.rotate_token(&UserId::new(id))?;
    let now_ms = state.clock.now_ms();
    Ok(Json(UserWithTokenRes {
        user: to_dto(user, now_ms),
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

async fn admin_health<S: UserStore, C: Clock>(
    State(state): State<AppState<S, C>>,
) -> Result<Json<HealthSnapshot>, ApiError> {
    let users = state.store.list_all()?;
    let active_admins = users
        .iter()
        .filter(|u| u.role == UserRole::Admin && u.state == UserState::Active)
        .count();
    let active_conns = state.conns.len()?;
    Ok(Json(build_health_snapshot(
        state.pool.as_deref(),
        active_admins,
        active_conns,
    )))
}

async fn admin_overview<S: UserStore, C: Clock>(
    State(state): State<AppState<S, C>>,
) -> Result<Json<OverviewRes>, ApiError> {
    let now_ms = state.clock.now_ms();
    let svc = UserAdminService::new(state.store.as_ref(), state.clock.as_ref());
    let m = svc.metrics()?;
    let users = svc.list_users()?;
    let active_admins = users
        .iter()
        .filter(|u| u.role == UserRole::Admin && u.state == UserState::Active)
        .count();

    let trend = match state.ledger.as_ref() {
        Some(ledger) => ledger
            .daily_trend(now_ms, 7)?
            .into_iter()
            .map(|r| TrendDto {
                day: format_month_day(r.day_ms),
                requests: r.requests,
                hit_rate: r.hit_rate,
                tokens_k: r.tokens_k,
            })
            .collect(),
        None => Vec::new(),
    };

    let connections = state
        .conns
        .snapshot(now_ms)?
        .into_iter()
        .map(|c| ConnectionDto {
            id: c.id,
            user_id: c.user_id,
            user_name: c.user_name,
            conv_id: c.conv_id,
            ip: c.ip,
            uptime: c.uptime,
            inflight: c.inflight,
        })
        .collect::<Vec<_>>();

    let archives = match state.convs.as_ref() {
        Some(convs) => convs
            .list_archives(50)?
            .into_iter()
            .map(|a| ArchiveDto {
                conv_id: a.conv_id,
                owner_name: a.owner_name,
                group: a.group,
                messages: a.messages,
                prefix_version: a.prefix_version,
                bytes_kb: a.bytes_kb,
                updated_at: format_relative(a.updated_at_ms, now_ms, "暂无更新"),
            })
            .collect(),
        None => Vec::new(),
    };

    let health = build_health_snapshot(
        state.pool.as_deref(),
        active_admins,
        connections.len(),
    );

    Ok(Json(OverviewRes {
        metrics: MetricsRes {
            total_users: m.total_users,
            active_users: m.active_users,
            disabled_users: m.disabled_users,
            admin_users: m.admin_users,
            total_monthly_quota: m.total_monthly_quota,
            total_used_tokens: m.total_used_tokens,
            total_used_requests: m.total_used_requests,
        },
        trend,
        connections,
        archives,
        health,
        max_conns: MAX_CONNS,
    }))
}
