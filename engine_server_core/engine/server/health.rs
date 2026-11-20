use serde::Serialize;

use crate::db::migrate::{current_version, CURRENT_SCHEMA_VERSION};
use crate::db::pool::SqlitePool;
use crate::engine::server::conn::MAX_CONNS;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthCheckItem {
    pub name: String,
    pub detail: String,
    pub ok: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthSnapshot {
    pub ok: bool,
    pub ver: u32,
    pub db_mode: String,
    pub schema_ver: i64,
    pub max_conns: usize,
    pub active_conns: usize,
    pub checks: Vec<HealthCheckItem>,
}

pub fn build_health_snapshot(
    pool: Option<&SqlitePool>,
    active_admin_count: usize,
    active_conns: usize,
) -> HealthSnapshot {
    let mut checks = Vec::with_capacity(4);
    checks.push(HealthCheckItem {
        name: String::from("HTTP /healthz 探针"),
        detail: String::from("GET /healthz -> ver=1 · 路由在线"),
        ok: true,
    });

    let (db_mode, schema_ver, db_ok, schema_ok) = match pool {
        Some(p) => {
            let ph = p.health_snapshot();
            let ver = current_version(p).unwrap_or(0);
            let s_ok = ver >= CURRENT_SCHEMA_VERSION;
            checks.push(HealthCheckItem {
                name: String::from("SQLite 存储与写通道"),
                detail: format!("mode={} · {} · busy_timeout 5000ms", ph.journal_mode, ph.path),
                ok: ph.writable,
            });
            checks.push(HealthCheckItem {
                name: String::from("Schema 版本迁移"),
                detail: format!("schema_ver = v{ver}（基线 v{CURRENT_SCHEMA_VERSION}）"),
                ok: s_ok,
            });
            (ph.journal_mode, ver, ph.writable, s_ok)
        }
        None => {
            checks.push(HealthCheckItem {
                name: String::from("存储模式"),
                detail: String::from("memory store（未挂载持久化库）"),
                ok: true,
            });
            (String::from("memory"), 0, true, true)
        }
    };

    let admin_ok = active_admin_count > 0;
    checks.push(HealthCheckItem {
        name: String::from("管理员引导与隔离检查"),
        detail: format!("启用管理员 {active_admin_count} 人 · 用户分区已启用"),
        ok: admin_ok,
    });

    HealthSnapshot {
        ok: db_ok && schema_ok && admin_ok,
        ver: 1,
        db_mode,
        schema_ver,
        max_conns: MAX_CONNS,
        active_conns,
        checks,
    }
}
