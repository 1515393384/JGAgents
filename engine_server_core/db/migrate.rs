use crate::core::error::EngineError;
use crate::db::pool::{map_sqlite_err, SqlitePool};

pub const CURRENT_SCHEMA_VERSION: i64 = 1;
const BASELINE_SCHEMA: &str = include_str!("schema.sql");

pub fn migrate(pool: &SqlitePool, now_ms: i64) -> Result<i64, EngineError> {
    pool.with_writer(|conn| {
        let tx = conn
            .transaction()
            .map_err(|e| map_sqlite_err(e, "begin migrate"))?;

        tx.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_ver (
                version INTEGER PRIMARY KEY,
                applied_at_ms INTEGER NOT NULL
            );",
        )
        .map_err(|e| map_sqlite_err(e, "ensure schema_ver"))?;

        let current: i64 = tx
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_ver",
                [],
                |row| row.get(0),
            )
            .map_err(|e| map_sqlite_err(e, "read schema_ver"))?;

        if current < 1 {
            tx.execute_batch(BASELINE_SCHEMA)
                .map_err(|e| map_sqlite_err(e, "apply v1 schema"))?;
            tx.execute(
                "INSERT OR REPLACE INTO schema_ver (version, applied_at_ms) VALUES (?1, ?2)",
                rusqlite::params![CURRENT_SCHEMA_VERSION, now_ms],
            )
            .map_err(|e| map_sqlite_err(e, "record schema_ver"))?;
        }

        tx.commit().map_err(|e| map_sqlite_err(e, "commit migrate"))?;

        let latest: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_ver",
                [],
                |row| row.get(0),
            )
            .map_err(|e| map_sqlite_err(e, "confirm schema_ver"))?;

        Ok(latest)
    })
}

pub fn current_version(pool: &SqlitePool) -> Result<i64, EngineError> {
    pool.with_reader(|conn| {
        conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_ver",
            [],
            |row| row.get(0),
        )
        .map_err(|e| map_sqlite_err(e, "query schema_ver"))
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::core::ids::{ConvId, UserId};
    use crate::core::traits::{Clock, ConvStore, LedgerStore, UserStore};
    use crate::db::ledger::SqliteLedgerStore;
    use crate::db::migrate::{current_version, migrate, CURRENT_SCHEMA_VERSION};
    use crate::db::msgs::SqliteConvStore;
    use crate::db::pool::SqlitePool;
    use crate::db::users::SqliteUserStore;
    use crate::engine::user::admin::UserAdminService;
    use crate::engine::user::bootstrap::ensure_bootstrap_admin;
    use crate::engine::user::model::{CreateUserInput, UserRole};

    struct FixedClock(i64);

    impl Clock for FixedClock {
        fn now_ms(&self) -> i64 {
            self.0
        }
    }

    #[test]
    fn sqlite_migrate_users_msgs_and_ledger_end_to_end() {
        let pool = Arc::new(SqlitePool::open_in_memory().expect("open mem sqlite"));
        let now = 1_763_600_000_000_i64;

        let v1 = migrate(&pool, now).expect("first migrate");
        let v2 = migrate(&pool, now + 1000).expect("idempotent migrate");
        assert_eq!(v1, CURRENT_SCHEMA_VERSION);
        assert_eq!(v2, CURRENT_SCHEMA_VERSION);
        assert_eq!(current_version(&pool).expect("ver"), CURRENT_SCHEMA_VERSION);

        let clock = FixedClock(now);
        let users = SqliteUserStore::new(Arc::clone(&pool));
        let boot = ensure_bootstrap_admin(&users, &clock).expect("bootstrap");
        assert!(boot.is_some());
        assert!(ensure_bootstrap_admin(&users, &clock)
            .expect("second bootstrap")
            .is_none());

        let svc = UserAdminService::new(&users, &clock);
        let (created, _) = svc
            .create_user(CreateUserInput {
                name: String::from("测试成员"),
                group: String::from("研发"),
                role: UserRole::Member,
                daily_requests: 500,
                monthly_tokens: 1_000_000,
            })
            .expect("create user");

        let convs = SqliteConvStore::new(Arc::clone(&pool));
        let conv_id = ConvId::new("conv_demo");
        let seq1 = convs
            .append(&created.id, &conv_id, b"hello", now)
            .expect("append 1");
        let seq2 = convs
            .append(&created.id, &conv_id, b"world", now + 500)
            .expect("append 2");
        assert_eq!((seq1, seq2), (1, 2));
        assert_eq!(convs.list_archives(10).expect("archives").len(), 1);

        let ledger = SqliteLedgerStore::new(Arc::clone(&pool));
        ledger
            .record_usage(&created.id, &conv_id, 1000, 800, 200, now)
            .expect("record usage");

        let reloaded = users
            .get_by_id(&UserId::new(created.id.as_str()))
            .expect("get")
            .expect("exists");
        assert_eq!(reloaded.quota.used_requests, 1);
        assert_eq!(reloaded.quota.used_tokens, 1200);

        let trend = ledger.daily_trend(now, 7).expect("trend");
        assert_eq!(trend.len(), 7);
        assert_eq!(trend[6].requests, 1);
        assert_eq!(trend[6].hit_rate, 80);
    }
}
