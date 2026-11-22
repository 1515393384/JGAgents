use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

use engine_server_core::core::traits::Clock;
use engine_server_core::db::docs::SqliteDocStore;
use engine_server_core::db::ledger::SqliteLedgerStore;
use engine_server_core::db::migrate::migrate;
use engine_server_core::db::msgs::SqliteConvStore;
use engine_server_core::db::pool::SqlitePool;
use engine_server_core::db::sessions::SqliteSessionStore;
use engine_server_core::db::users::SqliteUserStore;
use engine_server_core::engine::server::conn::ConnRegistry;
use engine_server_core::engine::server::http::{build_router, AppState};
use engine_server_core::engine::user::bootstrap::ensure_bootstrap_admin;
use engine_server_core::engine::user::token::TokenPepper;

struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> i64 {
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(dur) => dur.as_millis().min(i64::MAX as u128) as i64,
            Err(_) => 0,
        }
    }
}

fn resolve_sibling_path(env_key: &str, db_path: &str, filename: &str) -> PathBuf {
    if let Ok(explicit) = std::env::var(env_key) {
        let trimmed = explicit.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    let db_dir = Path::new(db_path)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    db_dir.join(filename)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    let clock = Arc::new(SystemClock);
    let db_path = std::env::var("DB_PATH").unwrap_or_else(|_| String::from("./data/engine.db"));
    let pepper_path = resolve_sibling_path("TOKEN_PEPPER_PATH", &db_path, ".token-pepper");
    let bootstrap_path =
        resolve_sibling_path("BOOTSTRAP_TOKEN_PATH", &db_path, ".bootstrap-admin-token");

    let pepper = Arc::new(TokenPepper::load_or_init(&pepper_path)?);
    let pool = Arc::new(SqlitePool::open(&db_path)?);
    let schema_ver = migrate(pool.as_ref(), clock.now_ms())?;
    tracing::info!(%db_path, schema_ver, "sqlite initialized");

    let users = Arc::new(SqliteUserStore::new(Arc::clone(&pool)));
    let sessions = Arc::new(SqliteSessionStore::new(Arc::clone(&pool)));
    let docs = Arc::new(SqliteDocStore::new(Arc::clone(&pool)));
    let convs = Arc::new(SqliteConvStore::new(Arc::clone(&pool)));
    let ledger = Arc::new(SqliteLedgerStore::new(Arc::clone(&pool)));
    let health = Arc::clone(&pool);
    let conns = Arc::new(ConnRegistry::default());

    if let Some(admin) = ensure_bootstrap_admin(
        users.as_ref(),
        clock.as_ref(),
        pepper.as_ref(),
        Some(&bootstrap_path),
    )? {
        tracing::info!(
            user_id = %admin.record.id,
            token_file = %bootstrap_path.display(),
            "bootstrap admin created; one-time token written to restricted file"
        );
    }

    let addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| String::from("0.0.0.0:3000"));
    let listener = TcpListener::bind(&addr).await?;
    tracing::info!(%addr, "engine_server_core listening");

    let app = build_router(AppState {
        users,
        sessions,
        docs,
        convs,
        ledger,
        health,
        clock,
        pepper,
        conns,
    });

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
