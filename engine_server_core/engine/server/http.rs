use std::sync::Arc;

use axum::extract::{Path, Query, Request, State};
use axum::http::{header, HeaderValue, Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::core::error::EngineError;
use crate::core::ids::UserId;
use crate::core::traits::{Clock, ConvStore, HealthProbe, LedgerStore, UserStore};
use crate::db::pool::run_blocking;
use crate::engine::user::model::UserRecord;
use crate::engine::server::conn::{ConnRegistry, MAX_CONNS};
use crate::engine::server::health::{build_health_snapshot, HealthSnapshot};
use crate::engine::user::admin::UserAdminService;
use crate::engine::user::gate::TokenGate;
use crate::engine::user::model::{
    CreateUserInput, UpdateUserInput, UserQuery, UserRole, UserSort, UserState,
};
use crate::engine::user::token::TokenPepper;

pub struct AppState<U, V, L, H, C>
where
    U: UserStore,
    V: ConvStore,
    L: LedgerStore,
    H: HealthProbe,
    C: Clock,
{
    pub users: Arc<U>,
    pub convs: Arc<V>,
    pub ledger: Arc<L>,
    pub health: Arc<H>,
    pub clock: Arc<C>,
    pub pepper: Arc<TokenPepper>,
    pub conns: Arc<ConnRegistry>,
}

impl<U, V, L, H, C> Clone for AppState<U, V, L, H, C>
where
    U: UserStore,
    V: ConvStore,
    L: LedgerStore,
    H: HealthProbe,
    C: Clock,
{
    fn clone(&self) -> Self {
        Self {
            users: Arc::clone(&self.users),
            convs: Arc::clone(&self.convs),
            ledger: Arc::clone(&self.ledger),
            health: Arc::clone(&self.health),
            clock: Arc::clone(&self.clock),
            pepper: Arc::clone(&self.pepper),
            conns: Arc::clone(&self.conns),
        }
    }
}

#[derive(Serialize)]
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
    token_hint: String,
    created_at_ms: i64,
    last_active_ms: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UsersListRes {
    users: Vec<UserDto>,
    total: usize,
    page: usize,
    page_size: usize,
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
struct BatchStateRes {
    updated: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MetricsRes {
    total_users: usize,
    active_users: usize,
    disabled_users: usize,
    admin_users: usize,
    active_admin_users: usize,
    total_monthly_quota: u64,
    total_used_tokens: u64,
    total_used_requests: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TrendDto {
    day_ms: i64,
    requests: u64,
    prompt_tokens: u64,
    cached_tokens: u64,
    completion_tokens: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConnectionDto {
    id: String,
    user_id: String,
    user_name: String,
    conv_id: String,
    ip: String,
    connected_at_ms: i64,
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
    bytes: u64,
    updated_at_ms: i64,
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
struct ListUsersQuery {
    search: Option<String>,
    state: Option<String>,
    role: Option<String>,
    group: Option<String>,
    sort: Option<String>,
    page: Option<usize>,
    page_size: Option<usize>,
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

fn to_dto(u: UserRecord) -> UserDto {
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
        token_hint: u.token_hint,
        created_at_ms: u.created_at_ms,
        last_active_ms: u.last_active_ms,
    }
}

fn build_cors_layer() -> CorsLayer {
    let mut origins: Vec<HeaderValue> = vec![
        HeaderValue::from_static("http://localhost:5173"),
        HeaderValue::from_static("http://127.0.0.1:5173"),
        HeaderValue::from_static("http://localhost:3000"),
        HeaderValue::from_static("http://127.0.0.1:3000"),
    ];

    if let Ok(extra) = std::env::var("CORS_ORIGINS") {
        for raw in extra.split(',') {
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed == "*" {
                continue;
            }
            match HeaderValue::from_str(trimmed) {
                Ok(val) => {
                    if !origins.contains(&val) {
                        origins.push(val);
                    }
                }
                Err(err) => {
                    tracing::warn!(origin = %trimmed, error = %err, "ignored invalid CORS_ORIGINS entry");
                }
            }
        }
    }

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
}

pub fn build_router<U, V, L, H, C>(state: AppState<U, V, L, H, C>) -> Router
where
    U: UserStore + 'static,
    V: ConvStore + 'static,
    L: LedgerStore + 'static,
    H: HealthProbe + 'static,
    C: Clock + 'static,
{
    let admin_routes = Router::new()
        .route(
            "/users",
            get(list_users::<U, V, L, H, C>).post(create_user::<U, V, L, H, C>),
        )
        .route("/users/batch-state", patch(batch_set_state::<U, V, L, H, C>))
        .route(
            "/users/{id}",
            patch(update_user::<U, V, L, H, C>).delete(delete_user::<U, V, L, H, C>),
        )
        .route("/users/{id}/state", patch(set_user_state::<U, V, L, H, C>))
        .route("/users/{id}/token", post(rotate_token::<U, V, L, H, C>))
        .route("/metrics", get(user_metrics::<U, V, L, H, C>))
        .route("/overview", get(admin_overview::<U, V, L, H, C>))
        .route("/health", get(admin_health::<U, V, L, H, C>))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_admin_middleware::<U, V, L, H, C>,
        ));

    Router::new()
        .route("/healthz", get(healthz::<U, V, L, H, C>))
        .nest("/api/admin", admin_routes)
        .layer(build_cors_layer())
        .with_state(state)
}

async fn require_admin_middleware<U, V, L, H, C>(
    State(state): State<AppState<U, V, L, H, C>>,
    mut req: Request,
    next: Next,
) -> Response
where
    U: UserStore + 'static,
    V: ConvStore + 'static,
    L: LedgerStore + 'static,
    H: HealthProbe + 'static,
    C: Clock + 'static,
{
    let bearer = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|raw| {
            let trimmed = raw.trim();
            trimmed
                .strip_prefix("Bearer ")
                .or_else(|| trimmed.strip_prefix("bearer "))
        })
        .map(str::to_string);

    let token = match bearer {
        Some(t) if !t.trim().is_empty() => t,
        _ => return ApiError(EngineError::unauthorized("missing bearer token")).into_response(),
    };

    let users = Arc::clone(&state.users);
    let pepper = Arc::clone(&state.pepper);
    match run_blocking(move || {
        let gate = TokenGate::new(users.as_ref(), pepper.as_ref());
        gate.require_admin(&token)
    })
    .await
    {
        Ok(admin_user) => {
            req.extensions_mut().insert(admin_user);
            next.run(req).await
        }
        Err(err) => ApiError(err).into_response(),
    }
}

async fn healthz<U, V, L, H, C>(
    State(state): State<AppState<U, V, L, H, C>>,
) -> Result<Json<HealthRes>, ApiError>
where
    U: UserStore + 'static,
    V: ConvStore + 'static,
    L: LedgerStore + 'static,
    H: HealthProbe + 'static,
    C: Clock + 'static,
{
    let users = Arc::clone(&state.users);
    let health = Arc::clone(&state.health);
    let conns = Arc::clone(&state.conns);

    let ok = run_blocking(move || {
        let storage = health.probe_storage()?;
        let metrics = users.metrics()?;
        let active_conns = conns.len()?;
        let snap = build_health_snapshot(&storage, metrics.active_admin_users, active_conns);
        Ok(snap.ok)
    })
    .await?;

    Ok(Json(HealthRes { ok, ver: 1 }))
}

async fn list_users<U, V, L, H, C>(
    State(state): State<AppState<U, V, L, H, C>>,
    Query(params): Query<ListUsersQuery>,
) -> Result<Json<UsersListRes>, ApiError>
where
    U: UserStore + 'static,
    V: ConvStore + 'static,
    L: LedgerStore + 'static,
    H: HealthProbe + 'static,
    C: Clock + 'static,
{
    let state_filter = match params.state.as_deref().map(str::trim).filter(|s| !s.is_empty() && *s != "all") {
        Some(raw) => Some(
            UserState::parse(raw).ok_or_else(|| EngineError::invalid_input("invalid state filter"))?,
        ),
        None => None,
    };
    let role_filter = match params.role.as_deref().map(str::trim).filter(|s| !s.is_empty() && *s != "all") {
        Some(raw) => Some(
            UserRole::parse(raw).ok_or_else(|| EngineError::invalid_input("invalid role filter"))?,
        ),
        None => None,
    };
    let sort = match params.sort.as_deref() {
        Some(raw) => UserSort::parse(raw).ok_or_else(|| EngineError::invalid_input("invalid sort"))?,
        None => UserSort::CreatedDesc,
    };

    let query = UserQuery {
        search: params.search,
        state: state_filter,
        role: role_filter,
        group: params.group,
        sort,
        page: params.page.unwrap_or(1),
        page_size: params.page_size.unwrap_or(50),
    };

    let users = Arc::clone(&state.users);
    let clock = Arc::clone(&state.clock);
    let pepper = Arc::clone(&state.pepper);
    let page = run_blocking(move || {
        let svc = UserAdminService::new(users.as_ref(), clock.as_ref(), pepper.as_ref());
        svc.query_users(&query)
    })
    .await?;

    Ok(Json(UsersListRes {
        users: page.items.into_iter().map(to_dto).collect(),
        total: page.total,
        page: page.page,
        page_size: page.page_size,
    }))
}

async fn create_user<U, V, L, H, C>(
    State(state): State<AppState<U, V, L, H, C>>,
    Json(req): Json<CreateUserReq>,
) -> Result<Json<UserWithTokenRes>, ApiError>
where
    U: UserStore + 'static,
    V: ConvStore + 'static,
    L: LedgerStore + 'static,
    H: HealthProbe + 'static,
    C: Clock + 'static,
{
    let role = UserRole::parse(&req.role)
        .ok_or_else(|| EngineError::invalid_input("invalid role"))?;

    let users = Arc::clone(&state.users);
    let clock = Arc::clone(&state.clock);
    let pepper = Arc::clone(&state.pepper);
    let (user, token) = run_blocking(move || {
        let svc = UserAdminService::new(users.as_ref(), clock.as_ref(), pepper.as_ref());
        svc.create_user(CreateUserInput {
            name: req.name,
            group: req.group,
            role,
            daily_requests: req.daily_requests,
            monthly_tokens: req.monthly_tokens,
        })
    })
    .await?;

    Ok(Json(UserWithTokenRes {
        user: to_dto(user),
        token,
    }))
}

async fn update_user<U, V, L, H, C>(
    State(state): State<AppState<U, V, L, H, C>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateUserReq>,
) -> Result<Json<UserRes>, ApiError>
where
    U: UserStore + 'static,
    V: ConvStore + 'static,
    L: LedgerStore + 'static,
    H: HealthProbe + 'static,
    C: Clock + 'static,
{
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

    let users = Arc::clone(&state.users);
    let clock = Arc::clone(&state.clock);
    let pepper = Arc::clone(&state.pepper);
    let user = run_blocking(move || {
        let svc = UserAdminService::new(users.as_ref(), clock.as_ref(), pepper.as_ref());
        svc.update_user(
            &UserId::new(id),
            UpdateUserInput {
                name: req.name,
                group: req.group,
                role,
                state: next_state,
                daily_requests: req.daily_requests,
                monthly_tokens: req.monthly_tokens,
            },
        )
    })
    .await?;

    Ok(Json(UserRes { user: to_dto(user) }))
}

async fn set_user_state<U, V, L, H, C>(
    State(state): State<AppState<U, V, L, H, C>>,
    Path(id): Path<String>,
    Json(req): Json<SetStateReq>,
) -> Result<Json<UserRes>, ApiError>
where
    U: UserStore + 'static,
    V: ConvStore + 'static,
    L: LedgerStore + 'static,
    H: HealthProbe + 'static,
    C: Clock + 'static,
{
    let next_state = UserState::parse(&req.state)
        .ok_or_else(|| EngineError::invalid_input("invalid state"))?;

    let users = Arc::clone(&state.users);
    let clock = Arc::clone(&state.clock);
    let pepper = Arc::clone(&state.pepper);
    let user = run_blocking(move || {
        let svc = UserAdminService::new(users.as_ref(), clock.as_ref(), pepper.as_ref());
        svc.set_state(&UserId::new(id), next_state)
    })
    .await?;

    Ok(Json(UserRes { user: to_dto(user) }))
}

async fn batch_set_state<U, V, L, H, C>(
    State(state): State<AppState<U, V, L, H, C>>,
    Json(req): Json<BatchStateReq>,
) -> Result<Json<BatchStateRes>, ApiError>
where
    U: UserStore + 'static,
    V: ConvStore + 'static,
    L: LedgerStore + 'static,
    H: HealthProbe + 'static,
    C: Clock + 'static,
{
    let next_state = UserState::parse(&req.state)
        .ok_or_else(|| EngineError::invalid_input("invalid state"))?;

    let users = Arc::clone(&state.users);
    let clock = Arc::clone(&state.clock);
    let pepper = Arc::clone(&state.pepper);
    let updated = run_blocking(move || {
        let svc = UserAdminService::new(users.as_ref(), clock.as_ref(), pepper.as_ref());
        svc.batch_set_state(&req.ids, next_state)
    })
    .await?;

    Ok(Json(BatchStateRes { updated }))
}

async fn rotate_token<U, V, L, H, C>(
    State(state): State<AppState<U, V, L, H, C>>,
    Path(id): Path<String>,
) -> Result<Json<UserWithTokenRes>, ApiError>
where
    U: UserStore + 'static,
    V: ConvStore + 'static,
    L: LedgerStore + 'static,
    H: HealthProbe + 'static,
    C: Clock + 'static,
{
    let users = Arc::clone(&state.users);
    let clock = Arc::clone(&state.clock);
    let pepper = Arc::clone(&state.pepper);
    let (user, token) = run_blocking(move || {
        let svc = UserAdminService::new(users.as_ref(), clock.as_ref(), pepper.as_ref());
        svc.rotate_token(&UserId::new(id))
    })
    .await?;

    Ok(Json(UserWithTokenRes {
        user: to_dto(user),
        token,
    }))
}

async fn delete_user<U, V, L, H, C>(
    State(state): State<AppState<U, V, L, H, C>>,
    Path(id): Path<String>,
) -> Result<Json<OkRes>, ApiError>
where
    U: UserStore + 'static,
    V: ConvStore + 'static,
    L: LedgerStore + 'static,
    H: HealthProbe + 'static,
    C: Clock + 'static,
{
    let users = Arc::clone(&state.users);
    let clock = Arc::clone(&state.clock);
    let pepper = Arc::clone(&state.pepper);
    run_blocking(move || {
        let svc = UserAdminService::new(users.as_ref(), clock.as_ref(), pepper.as_ref());
        svc.delete_user(&UserId::new(id))
    })
    .await?;

    Ok(Json(OkRes { ok: true }))
}

async fn user_metrics<U, V, L, H, C>(
    State(state): State<AppState<U, V, L, H, C>>,
) -> Result<Json<MetricsRes>, ApiError>
where
    U: UserStore + 'static,
    V: ConvStore + 'static,
    L: LedgerStore + 'static,
    H: HealthProbe + 'static,
    C: Clock + 'static,
{
    let users = Arc::clone(&state.users);
    let clock = Arc::clone(&state.clock);
    let pepper = Arc::clone(&state.pepper);
    let m = run_blocking(move || {
        let svc = UserAdminService::new(users.as_ref(), clock.as_ref(), pepper.as_ref());
        svc.metrics()
    })
    .await?;

    Ok(Json(MetricsRes {
        total_users: m.total_users,
        active_users: m.active_users,
        disabled_users: m.disabled_users,
        admin_users: m.admin_users,
        active_admin_users: m.active_admin_users,
        total_monthly_quota: m.total_monthly_quota,
        total_used_tokens: m.total_used_tokens,
        total_used_requests: m.total_used_requests,
    }))
}

async fn admin_health<U, V, L, H, C>(
    State(state): State<AppState<U, V, L, H, C>>,
) -> Result<Json<HealthSnapshot>, ApiError>
where
    U: UserStore + 'static,
    V: ConvStore + 'static,
    L: LedgerStore + 'static,
    H: HealthProbe + 'static,
    C: Clock + 'static,
{
    let users = Arc::clone(&state.users);
    let health = Arc::clone(&state.health);
    let conns = Arc::clone(&state.conns);

    let snap = run_blocking(move || {
        let storage = health.probe_storage()?;
        let metrics = users.metrics()?;
        let active_conns = conns.len()?;
        Ok(build_health_snapshot(
            &storage,
            metrics.active_admin_users,
            active_conns,
        ))
    })
    .await?;

    Ok(Json(snap))
}

async fn admin_overview<U, V, L, H, C>(
    State(state): State<AppState<U, V, L, H, C>>,
) -> Result<Json<OverviewRes>, ApiError>
where
    U: UserStore + 'static,
    V: ConvStore + 'static,
    L: LedgerStore + 'static,
    H: HealthProbe + 'static,
    C: Clock + 'static,
{
    let users = Arc::clone(&state.users);
    let convs = Arc::clone(&state.convs);
    let ledger = Arc::clone(&state.ledger);
    let health = Arc::clone(&state.health);
    let clock = Arc::clone(&state.clock);
    let conns = Arc::clone(&state.conns);

    let res = run_blocking(move || {
        let now_ms = clock.now_ms();
        let m = users.metrics()?;
        let trend = ledger
            .daily_trend(now_ms, 7)?
            .into_iter()
            .map(|r| TrendDto {
                day_ms: r.day_ms,
                requests: r.requests,
                prompt_tokens: r.prompt_tokens,
                cached_tokens: r.cached_tokens,
                completion_tokens: r.completion_tokens,
            })
            .collect();

        let connections = conns
            .snapshot()?
            .into_iter()
            .map(|c| ConnectionDto {
                id: c.id,
                user_id: c.user_id,
                user_name: c.user_name,
                conv_id: c.conv_id,
                ip: c.ip,
                connected_at_ms: c.connected_at_ms,
                inflight: c.inflight,
            })
            .collect::<Vec<_>>();

        let archives = convs
            .list_archives(50)?
            .into_iter()
            .map(|a| ArchiveDto {
                conv_id: a.conv_id,
                owner_name: a.owner_name,
                group: a.group,
                messages: a.messages,
                prefix_version: a.prefix_version,
                bytes: a.bytes,
                updated_at_ms: a.updated_at_ms,
            })
            .collect();

        let storage = health.probe_storage()?;
        let health_snap = build_health_snapshot(&storage, m.active_admin_users, connections.len());

        Ok(OverviewRes {
            metrics: MetricsRes {
                total_users: m.total_users,
                active_users: m.active_users,
                disabled_users: m.disabled_users,
                admin_users: m.admin_users,
                active_admin_users: m.active_admin_users,
                total_monthly_quota: m.total_monthly_quota,
                total_used_tokens: m.total_used_tokens,
                total_used_requests: m.total_used_requests,
            },
            trend,
            connections,
            archives,
            health: health_snap,
            max_conns: MAX_CONNS,
        })
    })
    .await?;

    Ok(Json(res))
}
