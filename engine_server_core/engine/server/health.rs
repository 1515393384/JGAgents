use serde::Serialize;

use crate::core::traits::StorageHealth;
use crate::engine::server::conn::MAX_CONNS;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthCheckItem {
    pub key: &'static str,
    pub ok: bool,
    pub value: String,
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
    pub active_admins: usize,
    pub checks: Vec<HealthCheckItem>,
}

pub fn build_health_snapshot(
    storage: &StorageHealth,
    active_admin_count: usize,
    active_conns: usize,
) -> HealthSnapshot {
    let schema_ok = storage.schema_ver >= storage.expected_schema_ver;
    let admin_ok = active_admin_count > 0;
    let conn_ok = active_conns <= MAX_CONNS;

    let checks = vec![
        HealthCheckItem {
            key: "storage_write",
            ok: storage.writable,
            value: storage.db_mode.clone(),
        },
        HealthCheckItem {
            key: "schema_version",
            ok: schema_ok,
            value: format!("v{}/v{}", storage.schema_ver, storage.expected_schema_ver),
        },
        HealthCheckItem {
            key: "active_admins",
            ok: admin_ok,
            value: active_admin_count.to_string(),
        },
        HealthCheckItem {
            key: "conn_capacity",
            ok: conn_ok,
            value: format!("{active_conns}/{MAX_CONNS}"),
        },
    ];

    HealthSnapshot {
        ok: storage.writable && schema_ok && admin_ok && conn_ok,
        ver: 1,
        db_mode: storage.db_mode.clone(),
        schema_ver: storage.schema_ver,
        max_conns: MAX_CONNS,
        active_conns,
        active_admins: active_admin_count,
        checks,
    }
}
