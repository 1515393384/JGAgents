use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

use engine_server_core::core::traits::Clock;
use engine_server_core::db::users::InMemoryUserStore;
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

    let store = Arc::new(InMemoryUserStore::default());
    let clock = Arc::new(SystemClock);

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

    let app = build_router(AppState { store, clock });
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
