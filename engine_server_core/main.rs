use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

use engine_server_core::core::traits::Clock;
use engine_server_core::db::ledger::SqliteLedgerStore;
use engine_server_core::db::migrate::migrate;
use engine_server_core::db::msgs::SqliteConvStore;
use engine_server_core::db::pool::SqlitePool;
use engine_server_core::db::users::SqliteUserStore;
use engine_server_core::engine::server::conn::ConnRegistry;
use engine_server_core::engine::server::http::{build_router, AppState};
use engine_server_core::engine::user::bootstrap::ensure_bootstrap_admin;

struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> i64 {
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(dur) => dur.as_millis().min(i64::MAX as u128) as i64,
            Err(_) => 0,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    let clock = Arc::new(SystemClock);
    let db_path = std::env::var("DB_PATH").unwrap_or_else(|_| String::from("./data/engine.db"));
    let pool = Arc::new(SqlitePool::open(&db_path)?);
    let schema_ver = migrate(pool.as_ref(), clock.now_ms())?;
    tracing::info!(%db_path, schema_ver, "sqlite initialized");

    let store = Arc::new(SqliteUserStore::new(Arc::clone(&pool)));
    let convs = Arc::new(SqliteConvStore::new(Arc::clone(&pool)));
    let ledger = Arc::new(SqliteLedgerStore::new(Arc::clone(&pool)));
    let conns = Arc::new(ConnRegistry::default());

    if let Some(admin) = ensure_bootstrap_admin(store.as_ref(), clock.as_ref())? {
        tracing::info!(
            user_id = %admin.record.id,
            token = %admin.one_time_token,
            "bootstrap admin created"
        );
    }

    let addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| String::from("0.0.0.0:3000"));
    let listener = TcpListener::bind(&addr).await?;
    tracing::info!(%addr, "engine_server_core listening");

    let app = build_router(AppState {
        store,
        clock,
        pool: Some(pool),
        convs: Some(convs),
        ledger: Some(ledger),
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
