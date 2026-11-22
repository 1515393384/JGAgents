use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{DefaultBodyLimit, Extension, Multipart, Path, Query, Request, State};
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::core::error::EngineError;
use crate::core::ids::{encode_hex, UserId};
use crate::core::traits::{Clock, ConvStore, DocStore, HealthProbe, LedgerStore, SessionStore, UserStore};
use crate::db::pool::run_blocking;
use crate::doc::diff::compute_doc_diff;
use crate::doc::model::{
    DocDetail, DocDiffReport, DocNodeRecord, DocRecord, DocRevisionMeta, DocSearchHit, ExportMode,
    PatchEditInput, MAX_DOCX_UPLOAD_BYTES,
};
use crate::doc::ooxml::{parse_docx_ast, patch_docx_zip, ParagraphPatch};
use crate::engine::user::admin::UserAdminService;
use crate::engine::user::gate::TokenGate;
use crate::engine::user::model::{
    CreateUserInput, UpdateUserInput, UserQuery, UserRecord, UserRole, UserSort, UserState,
};
use crate::engine::user::session::AuthSessionService;
use crate::engine::user::token::TokenPepper;

const SESSION_COOKIE_NAME: &str = "jg_session";
const MULTIPART_LIMIT_BYTES: usize = MAX_DOCX_UPLOAD_BYTES + 2 * 1024 * 1024;

pub struct AppState<U, S, D, V, L, H, C>
where
    U: UserStore,
    S: SessionStore,
    D: DocStore,
    V: ConvStore,
    L: LedgerStore,
    H: HealthProbe,
    C: Clock,
{
    pub users: Arc<U>,
    pub sessions: Arc<S>,
    pub docs: Arc<D>,
    pub convs: Arc<V>,
    pub ledger: Arc<L>,
    pub health: Arc<H>,
    pub clock: Arc<C>,
    pub pepper: Arc<TokenPepper>,
    pub conns: Arc<ConnRegistry>,
}

use crate::engine::server::conn::{ConnRegistry, MAX_CONNS};
use crate::engine::server::health::{build_health_snapshot, HealthSnapshot};

impl<U, S, D, V, L, H, C> Clone for AppState<U, S, D, V, L, H, C>
where
    U: UserStore,
    S: SessionStore,
    D: DocStore,
    V: ConvStore,
    L: LedgerStore,
    H: HealthProbe,
    C: Clock,
{
    fn clone(&self) -> Self {
        Self {
            users: Arc::clone(&self.users),
            sessions: Arc::clone(&self.sessions),
            docs: Arc::clone(&self.docs),
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
struct AuthCredentialsReq {
    username: String,
    password: String,
    #[serde(default)]
    remember: bool,
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListDocsQuery {
    space_id: Option<String>,
}

#[derive(Deserialize)]
struct GetDocQuery {
    rev: Option<i64>,
}

#[derive(Deserialize)]
struct SearchDocQuery {
    q: Option<String>,
    limit: Option<usize>,
}

#[derive(Deserialize)]
struct DiffDocQuery {
    from: Option<i64>,
    to: Option<i64>,
}

#[derive(Deserialize)]
struct LockNodeReq {
    locked: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PatchDocReq {
    edits: Vec<PatchEditInput>,
    mode: Option<String>,
    note: Option<String>,
    #[serde(default)]
    force: bool,
}

#[derive(Deserialize)]
struct DownloadDocQuery {
    mode: Option<String>,
}

#[derive(Serialize)]
struct DocsListRes {
    docs: Vec<DocRecord>,
}

#[derive(Serialize)]
struct DocSearchRes {
    hits: Vec<DocSearchHit>,
}

#[derive(Serialize)]
struct LockNodeRes {
    node: DocNodeRecord,
}

#[derive(Serialize)]
struct PatchDocRes {
    detail: DocDetail,
    diff: Option<DocDiffReport>,
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

fn extract_session_cookie(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    for part in raw.split(';') {
        let trimmed = part.trim();
        if let Some(val) = trimmed.strip_prefix(&format!("{SESSION_COOKIE_NAME}=")) {
            if !val.trim().is_empty() {
                return Some(val.trim().to_string());
            }
        }
    }
    None
}

fn build_session_cookie_header(token: &str, max_age_secs: Option<i64>) -> String {
    let mut parts = vec![
        format!("{SESSION_COOKIE_NAME}={token}"),
        String::from("HttpOnly"),
        String::from("SameSite=Lax"),
        String::from("Path=/"),
    ];
    if let Some(max_age) = max_age_secs {
        parts.push(format!("Max-Age={}", max_age.max(0)));
    }
    parts.join("; ")
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
        .allow_credentials(true)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
}

pub fn build_router<U, S, D, V, L, H, C>(state: AppState<U, S, D, V, L, H, C>) -> Router
where
    U: UserStore + 'static,
    S: SessionStore + 'static,
    D: DocStore + 'static,
    V: ConvStore + 'static,
    L: LedgerStore + 'static,
    H: HealthProbe + 'static,
    C: Clock + 'static,
{
    let auth_routes = Router::new()
        .route("/login", post(auth_login::<U, S, D, V, L, H, C>))
        .route("/register", post(auth_register::<U, S, D, V, L, H, C>))
        .route("/me", get(auth_me::<U, S, D, V, L, H, C>))
        .route("/logout", post(auth_logout::<U, S, D, V, L, H, C>));

    let doc_routes = Router::new()
        .route("/", get(list_docs::<U, S, D, V, L, H, C>))
        .route(
            "/upload",
            post(upload_doc::<U, S, D, V, L, H, C>)
                .layer(DefaultBodyLimit::max(MULTIPART_LIMIT_BYTES)),
        )
        .route("/{id}", get(get_doc::<U, S, D, V, L, H, C>))
        .route(
            "/{id}/upload-revision",
            post(upload_doc_revision::<U, S, D, V, L, H, C>)
                .layer(DefaultBodyLimit::max(MULTIPART_LIMIT_BYTES)),
        )
        .route("/{id}/search", get(search_doc::<U, S, D, V, L, H, C>))
        .route("/{id}/diff", get(diff_doc::<U, S, D, V, L, H, C>))
        .route("/{id}/patch", post(patch_doc::<U, S, D, V, L, H, C>))
        .route(
            "/{id}/nodes/{anchor}/lock",
            post(lock_doc_node::<U, S, D, V, L, H, C>),
        )
        .route(
            "/{id}/revisions/{rev}/download",
            get(download_doc_revision::<U, S, D, V, L, H, C>),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_user_session_middleware::<U, S, D, V, L, H, C>,
        ));

    let admin_routes = Router::new()
        .route(
            "/users",
            get(list_users::<U, S, D, V, L, H, C>).post(create_user::<U, S, D, V, L, H, C>),
        )
        .route(
            "/users/batch-state",
            patch(batch_set_state::<U, S, D, V, L, H, C>),
        )
        .route(
            "/users/{id}",
            patch(update_user::<U, S, D, V, L, H, C>).delete(delete_user::<U, S, D, V, L, H, C>),
        )
        .route(
            "/users/{id}/state",
            patch(set_user_state::<U, S, D, V, L, H, C>),
        )
        .route(
            "/users/{id}/token",
            post(rotate_token::<U, S, D, V, L, H, C>),
        )
        .route("/metrics", get(user_metrics::<U, S, D, V, L, H, C>))
        .route("/overview", get(admin_overview::<U, S, D, V, L, H, C>))
        .route("/health", get(admin_health::<U, S, D, V, L, H, C>))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_admin_middleware::<U, S, D, V, L, H, C>,
        ));

    Router::new()
        .route("/healthz", get(healthz::<U, S, D, V, L, H, C>))
        .nest("/api/auth", auth_routes)
        .nest("/api/docs", doc_routes)
        .nest("/api/admin", admin_routes)
        .layer(build_cors_layer())
        .with_state(state)
}

async fn require_user_session_middleware<U, S, D, V, L, H, C>(
    State(state): State<AppState<U, S, D, V, L, H, C>>,
    mut req: Request,
    next: Next,
) -> Response
where
    U: UserStore + 'static,
    S: SessionStore + 'static,
    D: DocStore + 'static,
    V: ConvStore + 'static,
    L: LedgerStore + 'static,
    H: HealthProbe + 'static,
    C: Clock + 'static,
{
    let token = match extract_session_cookie(req.headers()) {
        Some(t) => t,
        None => return ApiError(EngineError::unauthorized("请先登录工作台")).into_response(),
    };

    let users = Arc::clone(&state.users);
    let sessions = Arc::clone(&state.sessions);
    let clock = Arc::clone(&state.clock);
    let pepper = Arc::clone(&state.pepper);

    match run_blocking(move || {
        let svc = AuthSessionService::new(
            users.as_ref(),
            sessions.as_ref(),
            clock.as_ref(),
            pepper.as_ref(),
        );
        svc.current_user(&token)
    })
    .await
    {
        Ok(user) => {
            req.extensions_mut().insert(user);
            next.run(req).await
        }
        Err(err) => ApiError(err).into_response(),
    }
}

async fn require_admin_middleware<U, S, D, V, L, H, C>(
    State(state): State<AppState<U, S, D, V, L, H, C>>,
    mut req: Request,
    next: Next,
) -> Response
where
    U: UserStore + 'static,
    S: SessionStore + 'static,
    D: DocStore + 'static,
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

async fn auth_login<U, S, D, V, L, H, C>(
    State(state): State<AppState<U, S, D, V, L, H, C>>,
    Json(req): Json<AuthCredentialsReq>,
) -> Result<Response, ApiError>
where
    U: UserStore + 'static,
    S: SessionStore + 'static,
    D: DocStore + 'static,
    V: ConvStore + 'static,
    L: LedgerStore + 'static,
    H: HealthProbe + 'static,
    C: Clock + 'static,
{
    let users = Arc::clone(&state.users);
    let sessions = Arc::clone(&state.sessions);
    let clock = Arc::clone(&state.clock);
    let pepper = Arc::clone(&state.pepper);

    let (user, issued) = run_blocking(move || {
        let svc = AuthSessionService::new(
            users.as_ref(),
            sessions.as_ref(),
            clock.as_ref(),
            pepper.as_ref(),
        );
        svc.login(&req.username, &req.password, req.remember)
    })
    .await?;

    let cookie = build_session_cookie_header(&issued.plain_token, issued.max_age_secs);
    let mut res = Json(UserRes { user: to_dto(user) }).into_response();
    if let Ok(val) = HeaderValue::from_str(&cookie) {
        res.headers_mut().insert(header::SET_COOKIE, val);
    }
    Ok(res)
}

async fn auth_register<U, S, D, V, L, H, C>(
    State(state): State<AppState<U, S, D, V, L, H, C>>,
    Json(req): Json<AuthCredentialsReq>,
) -> Result<Response, ApiError>
where
    U: UserStore + 'static,
    S: SessionStore + 'static,
    D: DocStore + 'static,
    V: ConvStore + 'static,
    L: LedgerStore + 'static,
    H: HealthProbe + 'static,
    C: Clock + 'static,
{
    let users = Arc::clone(&state.users);
    let sessions = Arc::clone(&state.sessions);
    let clock = Arc::clone(&state.clock);
    let pepper = Arc::clone(&state.pepper);

    let (user, issued) = run_blocking(move || {
        let svc = AuthSessionService::new(
            users.as_ref(),
            sessions.as_ref(),
            clock.as_ref(),
            pepper.as_ref(),
        );
        svc.register(&req.username, &req.password, req.remember)
    })
    .await?;

    let cookie = build_session_cookie_header(&issued.plain_token, issued.max_age_secs);
    let mut res = Json(UserRes { user: to_dto(user) }).into_response();
    if let Ok(val) = HeaderValue::from_str(&cookie) {
        res.headers_mut().insert(header::SET_COOKIE, val);
    }
    Ok(res)
}

async fn auth_me<U, S, D, V, L, H, C>(
    State(state): State<AppState<U, S, D, V, L, H, C>>,
    headers: HeaderMap,
) -> Result<Json<UserRes>, ApiError>
where
    U: UserStore + 'static,
    S: SessionStore + 'static,
    D: DocStore + 'static,
    V: ConvStore + 'static,
    L: LedgerStore + 'static,
    H: HealthProbe + 'static,
    C: Clock + 'static,
{
    let token = extract_session_cookie(&headers)
        .ok_or_else(|| EngineError::unauthorized("未登录"))?;

    let users = Arc::clone(&state.users);
    let sessions = Arc::clone(&state.sessions);
    let clock = Arc::clone(&state.clock);
    let pepper = Arc::clone(&state.pepper);

    let user = run_blocking(move || {
        let svc = AuthSessionService::new(
            users.as_ref(),
            sessions.as_ref(),
            clock.as_ref(),
            pepper.as_ref(),
        );
        svc.current_user(&token)
    })
    .await?;

    Ok(Json(UserRes { user: to_dto(user) }))
}

async fn auth_logout<U, S, D, V, L, H, C>(
    State(state): State<AppState<U, S, D, V, L, H, C>>,
    headers: HeaderMap,
) -> Result<Response, ApiError>
where
    U: UserStore + 'static,
    S: SessionStore + 'static,
    D: DocStore + 'static,
    V: ConvStore + 'static,
    L: LedgerStore + 'static,
    H: HealthProbe + 'static,
    C: Clock + 'static,
{
    if let Some(token) = extract_session_cookie(&headers) {
        let users = Arc::clone(&state.users);
        let sessions = Arc::clone(&state.sessions);
        let clock = Arc::clone(&state.clock);
        let pepper = Arc::clone(&state.pepper);

        let _ = run_blocking(move || {
            let svc = AuthSessionService::new(
                users.as_ref(),
                sessions.as_ref(),
                clock.as_ref(),
                pepper.as_ref(),
            );
            svc.logout(&token)
        })
        .await;
    }

    let clear_cookie = build_session_cookie_header("", Some(0));
    let mut res = Json(OkRes { ok: true }).into_response();
    if let Ok(val) = HeaderValue::from_str(&clear_cookie) {
        res.headers_mut().insert(header::SET_COOKIE, val);
    }
    Ok(res)
}

// ── Document Endpoints ──────────────────────────────────────────────────────

async fn list_docs<U, S, D, V, L, H, C>(
    State(state): State<AppState<U, S, D, V, L, H, C>>,
    Extension(user): Extension<UserRecord>,
    Query(q): Query<ListDocsQuery>,
) -> Result<Json<DocsListRes>, ApiError>
where
    U: UserStore + 'static,
    S: SessionStore + 'static,
    D: DocStore + 'static,
    V: ConvStore + 'static,
    L: LedgerStore + 'static,
    H: HealthProbe + 'static,
    C: Clock + 'static,
{
    let docs_store = Arc::clone(&state.docs);
    let docs = run_blocking(move || docs_store.list_docs(&user.id, q.space_id.as_deref())).await?;
    Ok(Json(DocsListRes { docs }))
}

async fn upload_doc<U, S, D, V, L, H, C>(
    State(state): State<AppState<U, S, D, V, L, H, C>>,
    Extension(user): Extension<UserRecord>,
    mut multipart: Multipart,
) -> Result<Json<DocDetail>, ApiError>
where
    U: UserStore + 'static,
    S: SessionStore + 'static,
    D: DocStore + 'static,
    V: ConvStore + 'static,
    L: LedgerStore + 'static,
    H: HealthProbe + 'static,
    C: Clock + 'static,
{
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut filename = String::from("未命名标书.docx");
    let mut space_id = String::from("presales");
    let mut title_opt: Option<String> = None;
    let mut note = String::from("初稿 .docx 入库");

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| EngineError::invalid_input("读取上传表单失败"))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                if let Some(fname) = field.file_name() {
                    let clean = fname.trim();
                    if !clean.is_empty() {
                        filename = sanitize_filename(clean);
                    }
                }
                let data = field
                    .bytes()
                    .await
                    .map_err(|_| EngineError::invalid_input("读取文档字节流失败"))?;
                file_bytes = Some(data.to_vec());
            }
            "spaceId" => {
                if let Ok(txt) = field.text().await {
                    let trimmed = txt.trim();
                    if !trimmed.is_empty() {
                        space_id = trimmed.to_string();
                    }
                }
            }
            "title" => {
                if let Ok(txt) = field.text().await {
                    let trimmed = txt.trim();
                    if !trimmed.is_empty() {
                        title_opt = Some(trimmed.to_string());
                    }
                }
            }
            "note" => {
                if let Ok(txt) = field.text().await {
                    let trimmed = txt.trim();
                    if !trimmed.is_empty() {
                        note = trimmed.to_string();
                    }
                }
            }
            _ => {}
        }
    }

    let bytes = file_bytes.ok_or_else(|| EngineError::invalid_input("请选择要上传的 .docx 文件"))?;
    let docs_store = Arc::clone(&state.docs);
    let clock = Arc::clone(&state.clock);

    let detail = run_blocking(move || {
        let parsed = parse_docx_ast(&bytes)?;
        let now = clock.now_ms();
        let doc_id = generate_doc_id()?;
        let title = title_opt.unwrap_or_else(|| strip_ext(&filename));

        let doc = DocRecord {
            id: doc_id.clone(),
            owner_user_id: user.id.clone(),
            space_id,
            title,
            filename,
            current_rev: 1,
            source_format: String::from("docx"),
            created_at_ms: now,
            updated_at_ms: now,
        };
        let rev = DocRevisionMeta {
            doc_id,
            rev: 1,
            note,
            author_name: user.name,
            export_mode: String::from("raw"),
            node_count: parsed.node_count,
            table_count: parsed.table_count,
            mandatory_count: parsed.mandatory_count,
            byte_size: bytes.len(),
            created_at_ms: now,
        };
        docs_store.create_doc(&doc, &rev, &bytes, &parsed)
    })
    .await?;

    Ok(Json(detail))
}

async fn upload_doc_revision<U, S, D, V, L, H, C>(
    State(state): State<AppState<U, S, D, V, L, H, C>>,
    Extension(user): Extension<UserRecord>,
    Path(doc_id): Path<String>,
    mut multipart: Multipart,
) -> Result<Json<PatchDocRes>, ApiError>
where
    U: UserStore + 'static,
    S: SessionStore + 'static,
    D: DocStore + 'static,
    V: ConvStore + 'static,
    L: LedgerStore + 'static,
    H: HealthProbe + 'static,
    C: Clock + 'static,
{
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut note = String::from("回传修订版 .docx");

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| EngineError::invalid_input("读取上传表单失败"))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                let data = field
                    .bytes()
                    .await
                    .map_err(|_| EngineError::invalid_input("读取文档字节流失败"))?;
                file_bytes = Some(data.to_vec());
            }
            "note" => {
                if let Ok(txt) = field.text().await {
                    let trimmed = txt.trim();
                    if !trimmed.is_empty() {
                        note = trimmed.to_string();
                    }
                }
            }
            _ => {}
        }
    }

    let bytes = file_bytes.ok_or_else(|| EngineError::invalid_input("请选择修订版 .docx 文件"))?;
    let docs_store = Arc::clone(&state.docs);
    let clock = Arc::clone(&state.clock);

    let res = run_blocking(move || {
        let prev_detail = docs_store.get_doc_detail(&doc_id, &user.id, None)?;
        let prev_rev = prev_detail.doc.current_rev;
        let parsed = parse_docx_ast(&bytes)?;
        let now = clock.now_ms();

        let next_detail = docs_store.append_revision(
            &doc_id,
            &user.id,
            &note,
            &user.name,
            ExportMode::Raw,
            &bytes,
            &parsed,
            Some(prev_rev),
            now,
        )?;
        let diff = compute_doc_diff(
            &doc_id,
            prev_rev,
            next_detail.doc.current_rev,
            &prev_detail.nodes,
            &next_detail.nodes,
        );
        Ok(PatchDocRes {
            detail: next_detail,
            diff: Some(diff),
        })
    })
    .await?;

    Ok(Json(res))
}

async fn get_doc<U, S, D, V, L, H, C>(
    State(state): State<AppState<U, S, D, V, L, H, C>>,
    Extension(user): Extension<UserRecord>,
    Path(doc_id): Path<String>,
    Query(q): Query<GetDocQuery>,
) -> Result<Json<DocDetail>, ApiError>
where
    U: UserStore + 'static,
    S: SessionStore + 'static,
    D: DocStore + 'static,
    V: ConvStore + 'static,
    L: LedgerStore + 'static,
    H: HealthProbe + 'static,
    C: Clock + 'static,
{
    let docs_store = Arc::clone(&state.docs);
    let detail = run_blocking(move || docs_store.get_doc_detail(&doc_id, &user.id, q.rev)).await?;
    Ok(Json(detail))
}

async fn search_doc<U, S, D, V, L, H, C>(
    State(state): State<AppState<U, S, D, V, L, H, C>>,
    Extension(user): Extension<UserRecord>,
    Path(doc_id): Path<String>,
    Query(q): Query<SearchDocQuery>,
) -> Result<Json<DocSearchRes>, ApiError>
where
    U: UserStore + 'static,
    S: SessionStore + 'static,
    D: DocStore + 'static,
    V: ConvStore + 'static,
    L: LedgerStore + 'static,
    H: HealthProbe + 'static,
    C: Clock + 'static,
{
    let query_str = q.q.unwrap_or_default();
    let limit = q.limit.unwrap_or(15);
    let docs_store = Arc::clone(&state.docs);
    let hits =
        run_blocking(move || docs_store.search_doc_fts(&doc_id, &user.id, &query_str, limit))
            .await?;
    Ok(Json(DocSearchRes { hits }))
}

async fn lock_doc_node<U, S, D, V, L, H, C>(
    State(state): State<AppState<U, S, D, V, L, H, C>>,
    Extension(user): Extension<UserRecord>,
    Path((doc_id, anchor)): Path<(String, String)>,
    Json(req): Json<LockNodeReq>,
) -> Result<Json<LockNodeRes>, ApiError>
where
    U: UserStore + 'static,
    S: SessionStore + 'static,
    D: DocStore + 'static,
    V: ConvStore + 'static,
    L: LedgerStore + 'static,
    H: HealthProbe + 'static,
    C: Clock + 'static,
{
    let docs_store = Arc::clone(&state.docs);
    let clock = Arc::clone(&state.clock);
    let node = run_blocking(move || {
        docs_store.set_node_locked(&doc_id, &user.id, &anchor, req.locked, clock.now_ms())
    })
    .await?;
    Ok(Json(LockNodeRes { node }))
}

async fn diff_doc<U, S, D, V, L, H, C>(
    State(state): State<AppState<U, S, D, V, L, H, C>>,
    Extension(user): Extension<UserRecord>,
    Path(doc_id): Path<String>,
    Query(q): Query<DiffDocQuery>,
) -> Result<Json<DocDiffReport>, ApiError>
where
    U: UserStore + 'static,
    S: SessionStore + 'static,
    D: DocStore + 'static,
    V: ConvStore + 'static,
    L: LedgerStore + 'static,
    H: HealthProbe + 'static,
    C: Clock + 'static,
{
    let docs_store = Arc::clone(&state.docs);
    let report = run_blocking(move || {
        let detail = docs_store.get_doc_detail(&doc_id, &user.id, None)?;
        let to_rev = q.to.unwrap_or(detail.doc.current_rev);
        let from_rev = q.from.unwrap_or_else(|| (to_rev - 1).max(1));
        let old_nodes = docs_store.get_revision_nodes(&doc_id, &user.id, from_rev)?;
        let new_nodes = docs_store.get_revision_nodes(&doc_id, &user.id, to_rev)?;
        Ok(compute_doc_diff(&doc_id, from_rev, to_rev, &old_nodes, &new_nodes))
    })
    .await?;

    Ok(Json(report))
}

async fn patch_doc<U, S, D, V, L, H, C>(
    State(state): State<AppState<U, S, D, V, L, H, C>>,
    Extension(user): Extension<UserRecord>,
    Path(doc_id): Path<String>,
    Json(req): Json<PatchDocReq>,
) -> Result<Json<PatchDocRes>, ApiError>
where
    U: UserStore + 'static,
    S: SessionStore + 'static,
    D: DocStore + 'static,
    V: ConvStore + 'static,
    L: LedgerStore + 'static,
    H: HealthProbe + 'static,
    C: Clock + 'static,
{
    if req.edits.is_empty() {
        return Err(EngineError::invalid_input("请至少提供一条段落修订").into());
    }

    let mode = ExportMode::parse(req.mode.as_deref());
    let docs_store = Arc::clone(&state.docs);
    let clock = Arc::clone(&state.clock);

    let res = run_blocking(move || {
        let cur_detail = docs_store.get_doc_detail(&doc_id, &user.id, None)?;
        let cur_rev = cur_detail.doc.current_rev;
        let (_, _, baseline_blob) = docs_store.get_revision_blob(&doc_id, &user.id, cur_rev)?;

        let nodes_by_anchor: HashMap<&str, &DocNodeRecord> = cur_detail
            .nodes
            .iter()
            .filter(|n| !n.is_search_chunk)
            .map(|n| (n.anchor_id.as_str(), n))
            .collect();

        let now = clock.now_ms();
        let date_iso = format_iso_utc(now);
        let mut patches_by_loc: HashMap<String, ParagraphPatch> = HashMap::new();
        let mut locked_hits: Vec<String> = Vec::new();

        for edit in &req.edits {
            let node = nodes_by_anchor
                .get(edit.anchor_id.as_str())
                .ok_or_else(|| EngineError::not_found(format!("未找到锚点条款 {}", edit.anchor_id)))?;
            if node.locked && !req.force {
                locked_hits.push(node.anchor_id.clone());
                continue;
            }
            if !node.xml_locator.starts_with("p:") {
                return Err(EngineError::invalid_input("暂仅支持正文段落原位补丁"));
            }
            patches_by_loc.insert(
                node.xml_locator.clone(),
                ParagraphPatch {
                    before_text: node.text_content.clone(),
                    new_text: edit.new_text.clone(),
                    mode,
                    author: user.name.clone(),
                    date_iso: date_iso.clone(),
                },
            );
        }

        if !locked_hits.is_empty() {
            return Err(EngineError::conflict(format!(
                "以下条款已锁定，拒绝覆盖: {}",
                locked_hits.join(", ")
            )));
        }

        let patched_blob = patch_docx_zip(&baseline_blob, &patches_by_loc)?;
        let reparsed = parse_docx_ast(&patched_blob)?;
        let note = req
            .note
            .unwrap_or_else(|| format!("修订 {} 处（{}）", req.edits.len(), mode.as_str()));

        let next_detail = docs_store.append_revision(
            &doc_id,
            &user.id,
            &note,
            &user.name,
            mode,
            &patched_blob,
            &reparsed,
            Some(cur_rev),
            now,
        )?;
        let diff = compute_doc_diff(
            &doc_id,
            cur_rev,
            next_detail.doc.current_rev,
            &cur_detail.nodes,
            &next_detail.nodes,
        );

        Ok(PatchDocRes {
            detail: next_detail,
            diff: Some(diff),
        })
    })
    .await?;

    Ok(Json(res))
}

async fn download_doc_revision<U, S, D, V, L, H, C>(
    State(state): State<AppState<U, S, D, V, L, H, C>>,
    Extension(user): Extension<UserRecord>,
    Path((doc_id, rev)): Path<(String, i64)>,
    Query(q): Query<DownloadDocQuery>,
) -> Result<Response, ApiError>
where
    U: UserStore + 'static,
    S: SessionStore + 'static,
    D: DocStore + 'static,
    V: ConvStore + 'static,
    L: LedgerStore + 'static,
    H: HealthProbe + 'static,
    C: Clock + 'static,
{
    let mode = ExportMode::parse(q.mode.as_deref());
    let docs_store = Arc::clone(&state.docs);
    let clock = Arc::clone(&state.clock);

    let (filename, out_bytes) = run_blocking(move || {
        let (doc, rev_meta, rev_blob) = docs_store.get_revision_blob(&doc_id, &user.id, rev)?;
        let bytes = if mode == ExportMode::Raw || rev <= 1 {
            rev_blob
        } else if rev_meta.export_mode == mode.as_str() {
            rev_blob
        } else {
            // Re-synthesize requested mode (redline or clean) from rev-1 baseline cleanly
            let (_, _, base_blob) = docs_store.get_revision_blob(&doc_id, &user.id, rev - 1)?;
            let old_nodes = docs_store.get_revision_nodes(&doc_id, &user.id, rev - 1)?;
            let new_nodes = docs_store.get_revision_nodes(&doc_id, &user.id, rev)?;
            let diff = compute_doc_diff(&doc_id, rev - 1, rev, &old_nodes, &new_nodes);

            let new_by_anchor: HashMap<&str, &DocNodeRecord> = new_nodes
                .iter()
                .filter(|n| !n.is_search_chunk)
                .map(|n| (n.anchor_id.as_str(), n))
                .collect();
            let old_primary: Vec<&DocNodeRecord> = old_nodes
                .iter()
                .filter(|n| !n.is_search_chunk)
                .collect();
            let old_by_anchor: HashMap<&str, &DocNodeRecord> = old_primary
                .iter()
                .map(|n| (n.anchor_id.as_str(), *n))
                .collect();

            let date_iso = format_iso_utc(clock.now_ms());
            let mut patches = HashMap::new();
            for blk in diff.blocks {
                if blk.added_chars == 0 && blk.deleted_chars == 0 {
                    continue;
                }
                if let Some(target) = new_by_anchor.get(blk.anchor_id.as_str()) {
                    let base_opt = old_by_anchor
                        .get(blk.anchor_id.as_str())
                        .copied()
                        .or_else(|| {
                            target.explicit_anchor.as_ref().and_then(|exp| {
                                old_primary
                                    .iter()
                                    .find(|o| o.explicit_anchor.as_ref() == Some(exp))
                                    .copied()
                            })
                        })
                        .or_else(|| {
                            old_primary
                                .iter()
                                .find(|o| o.text_content == blk.before_text)
                                .copied()
                        });
                    if let Some(base_n) = base_opt {
                        if base_n.xml_locator.starts_with("p:") {
                            patches.insert(
                                base_n.xml_locator.clone(),
                                ParagraphPatch {
                                    before_text: blk.before_text,
                                    new_text: target.text_content.clone(),
                                    mode,
                                    author: rev_meta.author_name.clone(),
                                    date_iso: date_iso.clone(),
                                },
                            );
                        }
                    }
                }
            }
            if patches.is_empty() {
                rev_blob
            } else {
                patch_docx_zip(&base_blob, &patches)?
            }
        };

        let base_name = strip_ext(&doc.filename);
        let suffix = match mode {
            ExportMode::Redline => format!("_v{rev}_修订红线版.docx"),
            ExportMode::Clean => format!("_v{rev}_净稿版.docx"),
            ExportMode::Raw => format!("_v{rev}.docx"),
        };
        Ok((format!("{base_name}{suffix}"), bytes))
    })
    .await?;

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        ),
    );
    let encoded_name = percent_encode_utf8(&filename);
    let disp = format!("attachment; filename=\"document.docx\"; filename*=UTF-8''{encoded_name}");
    if let Ok(val) = HeaderValue::from_str(&disp) {
        headers.insert(header::CONTENT_DISPOSITION, val);
    }
    Ok((StatusCode::OK, headers, out_bytes).into_response())
}

// ── Health & Admin Endpoints ────────────────────────────────────────────────

async fn healthz<U, S, D, V, L, H, C>(
    State(state): State<AppState<U, S, D, V, L, H, C>>,
) -> Result<Json<HealthRes>, ApiError>
where
    U: UserStore + 'static,
    S: SessionStore + 'static,
    D: DocStore + 'static,
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

async fn list_users<U, S, D, V, L, H, C>(
    State(state): State<AppState<U, S, D, V, L, H, C>>,
    Query(params): Query<ListUsersQuery>,
) -> Result<Json<UsersListRes>, ApiError>
where
    U: UserStore + 'static,
    S: SessionStore + 'static,
    D: DocStore + 'static,
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

async fn create_user<U, S, D, V, L, H, C>(
    State(state): State<AppState<U, S, D, V, L, H, C>>,
    Json(req): Json<CreateUserReq>,
) -> Result<Json<UserWithTokenRes>, ApiError>
where
    U: UserStore + 'static,
    S: SessionStore + 'static,
    D: DocStore + 'static,
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

async fn update_user<U, S, D, V, L, H, C>(
    State(state): State<AppState<U, S, D, V, L, H, C>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateUserReq>,
) -> Result<Json<UserRes>, ApiError>
where
    U: UserStore + 'static,
    S: SessionStore + 'static,
    D: DocStore + 'static,
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

async fn set_user_state<U, S, D, V, L, H, C>(
    State(state): State<AppState<U, S, D, V, L, H, C>>,
    Path(id): Path<String>,
    Json(req): Json<SetStateReq>,
) -> Result<Json<UserRes>, ApiError>
where
    U: UserStore + 'static,
    S: SessionStore + 'static,
    D: DocStore + 'static,
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

async fn batch_set_state<U, S, D, V, L, H, C>(
    State(state): State<AppState<U, S, D, V, L, H, C>>,
    Json(req): Json<BatchStateReq>,
) -> Result<Json<BatchStateRes>, ApiError>
where
    U: UserStore + 'static,
    S: SessionStore + 'static,
    D: DocStore + 'static,
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

async fn rotate_token<U, S, D, V, L, H, C>(
    State(state): State<AppState<U, S, D, V, L, H, C>>,
    Path(id): Path<String>,
) -> Result<Json<UserWithTokenRes>, ApiError>
where
    U: UserStore + 'static,
    S: SessionStore + 'static,
    D: DocStore + 'static,
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

async fn delete_user<U, S, D, V, L, H, C>(
    State(state): State<AppState<U, S, D, V, L, H, C>>,
    Path(id): Path<String>,
) -> Result<Json<OkRes>, ApiError>
where
    U: UserStore + 'static,
    S: SessionStore + 'static,
    D: DocStore + 'static,
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

async fn user_metrics<U, S, D, V, L, H, C>(
    State(state): State<AppState<U, S, D, V, L, H, C>>,
) -> Result<Json<MetricsRes>, ApiError>
where
    U: UserStore + 'static,
    S: SessionStore + 'static,
    D: DocStore + 'static,
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

async fn admin_health<U, S, D, V, L, H, C>(
    State(state): State<AppState<U, S, D, V, L, H, C>>,
) -> Result<Json<HealthSnapshot>, ApiError>
where
    U: UserStore + 'static,
    S: SessionStore + 'static,
    D: DocStore + 'static,
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

async fn admin_overview<U, S, D, V, L, H, C>(
    State(state): State<AppState<U, S, D, V, L, H, C>>,
) -> Result<Json<OverviewRes>, ApiError>
where
    U: UserStore + 'static,
    S: SessionStore + 'static,
    D: DocStore + 'static,
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

// ── Small Helpers ───────────────────────────────────────────────────────────

fn generate_doc_id() -> Result<String, EngineError> {
    let mut bytes = [0u8; 10];
    getrandom::fill(&mut bytes).map_err(|_| EngineError::internal("生成文档 ID 失败"))?;
    Ok(format!("doc_{}", encode_hex(&bytes)))
}

fn sanitize_filename(raw: &str) -> String {
    let base = raw
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("document.docx")
        .trim();
    let cleaned: String = base
        .chars()
        .filter(|c| !c.is_control() && *c != '"' && *c != ';')
        .collect();
    if cleaned.is_empty() {
        String::from("document.docx")
    } else {
        cleaned
    }
}

fn strip_ext(filename: &str) -> String {
    if let Some(stem) = filename.strip_suffix(".docx").or_else(|| filename.strip_suffix(".DOCX")) {
        if !stem.is_empty() {
            return stem.to_string();
        }
    }
    filename.to_string()
}

fn percent_encode_utf8(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() * 3);
    for b in raw.as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.') {
            out.push(*b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

fn format_iso_utc(ms: i64) -> String {
    let secs = (ms / 1000).max(0);
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let hh = rem / 3_600;
    let mm = (rem % 3_600) / 60;
    let ss = rem % 60;

    // Howard Hinnant's civil_from_days algorithm (epoch 1970-01-01)
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if m <= 2 { 1 } else { 0 };

    format!("{year:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}
