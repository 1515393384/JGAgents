use rusqlite::TransactionBehavior;

use crate::core::error::EngineError;
use crate::db::pool::{map_sqlite_err, SqlitePool};

pub const CURRENT_SCHEMA_VERSION: i64 = 2;
const BASELINE_SCHEMA: &str = include_str!("schema.sql");

pub fn migrate(pool: &SqlitePool, now_ms: i64) -> Result<i64, EngineError> {
    pool.with_writer(|conn| {
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
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
                .map_err(|e| map_sqlite_err(e, "apply baseline schema"))?;
            tx.execute(
                "INSERT OR REPLACE INTO schema_ver (version, applied_at_ms) VALUES (?1, ?2)",
                rusqlite::params![CURRENT_SCHEMA_VERSION, now_ms],
            )
            .map_err(|e| map_sqlite_err(e, "record schema_ver"))?;
        } else if current < 2 {
            tx.execute_batch(
                "ALTER TABLE users ADD COLUMN token_hint TEXT NOT NULL DEFAULT '';
                 CREATE INDEX IF NOT EXISTS idx_users_state_role ON users (state, role);
                 CREATE INDEX IF NOT EXISTS idx_users_created ON users (created_at_ms DESC);
                 CREATE INDEX IF NOT EXISTS idx_msgs_conv_time ON msgs (user_id, conv_id, created_at_ms DESC);
                 CREATE INDEX IF NOT EXISTS idx_ledger_time ON ledger (created_at_ms);",
            )
            .map_err(|e| map_sqlite_err(e, "migrate v1 to v2"))?;
            tx.execute(
                "INSERT OR REPLACE INTO schema_ver (version, applied_at_ms) VALUES (?1, ?2)",
                rusqlite::params![CURRENT_SCHEMA_VERSION, now_ms],
            )
            .map_err(|e| map_sqlite_err(e, "record v2 schema_ver"))?;
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
    use std::fs;
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
    use crate::engine::user::token::TokenPepper;

    struct FixedClock(i64);

    impl Clock for FixedClock {
        fn now_ms(&self) -> i64 {
            self.0
        }
    }

    #[test]
    fn sqlite_migrate_bootstrap_file_msgs_and_ledger_end_to_end() {
        let pool = Arc::new(SqlitePool::open_in_memory().expect("open mem sqlite"));
        let now = 1_763_600_000_000_i64;

        let v1 = migrate(&pool, now).expect("first migrate");
        let v2 = migrate(&pool, now + 1000).expect("idempotent migrate");
        assert_eq!(v1, CURRENT_SCHEMA_VERSION);
        assert_eq!(v2, CURRENT_SCHEMA_VERSION);
        assert_eq!(current_version(&pool).expect("ver"), CURRENT_SCHEMA_VERSION);

        let clock = FixedClock(now);
        let pepper = TokenPepper::ephemeral().expect("pepper");
        let users = SqliteUserStore::new(Arc::clone(&pool));

        let tmp_dir = std::env::temp_dir().join(format!(
            "jiagou_boot_{}_{}",
            std::process::id(),
            now
        ));
        let token_file = tmp_dir.join(".bootstrap-admin-token");

        let boot = ensure_bootstrap_admin(&users, &clock, &pepper, Some(&token_file))
            .expect("bootstrap")
            .expect("should create admin");
        let written = fs::read_to_string(&token_file).expect("read token file");
        assert_eq!(written.trim(), boot.one_time_token);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&token_file)
                .expect("meta")
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o600);
        }

        let _ = fs::remove_dir_all(&tmp_dir);

        assert!(ensure_bootstrap_admin(&users, &clock, &pepper, None)
            .expect("second bootstrap")
            .is_none());

        let svc = UserAdminService::new(&users, &clock, &pepper);
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

        let archives = convs.list_archives(10).expect("archives");
        assert_eq!(archives.len(), 1);
        assert_eq!(archives[0].bytes, 10);
        assert_eq!(archives[0].updated_at_ms, now + 500);

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
        assert_eq!(trend[6].prompt_tokens, 1000);
        assert_eq!(trend[6].cached_tokens, 800);
        assert_eq!(trend[6].completion_tokens, 200);
    }

    #[test]
    fn sqlite_upgrades_v1_schema_to_v2_preserving_data() {
        let pool = Arc::new(SqlitePool::open_in_memory().expect("open mem"));
        pool.with_writer(|conn| {
            conn.execute_batch(
                "CREATE TABLE schema_ver (version INTEGER PRIMARY KEY, applied_at_ms INTEGER NOT NULL);
                 INSERT INTO schema_ver (version, applied_at_ms) VALUES (1, 100);
                 CREATE TABLE users (
                     id TEXT PRIMARY KEY, name TEXT NOT NULL, grp TEXT NOT NULL,
                     role TEXT NOT NULL, state TEXT NOT NULL,
                     daily_requests INTEGER NOT NULL DEFAULT 0, monthly_tokens INTEGER NOT NULL DEFAULT 0,
                     used_requests INTEGER NOT NULL DEFAULT 0, used_tokens INTEGER NOT NULL DEFAULT 0,
                     token_hash TEXT NOT NULL UNIQUE, created_at_ms INTEGER NOT NULL, last_active_ms INTEGER NOT NULL DEFAULT 0
                 );
                 INSERT INTO users (id, name, grp, role, state, token_hash, created_at_ms)
                 VALUES ('u_legacy', '老用户', '系统', 'admin', 'active', 'deadbeef', 100);",
            )
            .expect("seed v1");
            Ok(())
        })
        .expect("setup v1");

        let ver = migrate(&pool, 200).expect("upgrade v1->v2");
        assert_eq!(ver, CURRENT_SCHEMA_VERSION);

        let store = SqliteUserStore::new(Arc::clone(&pool));
        let user = store
            .get_by_id(&UserId::new("u_legacy"))
            .expect("query")
            .expect("exists");
        assert_eq!(user.name, "老用户");
        assert_eq!(user.token_hint, "");
    }
}
