use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use rusqlite::Connection;

use crate::core::error::EngineError;
use crate::core::traits::{HealthProbe, StorageHealth};
use crate::db::migrate::CURRENT_SCHEMA_VERSION;

const DEFAULT_READER_POOL_SIZE: usize = 4;

struct ReaderPool {
    conns: Mutex<Vec<Connection>>,
    available: Condvar,
}

impl ReaderPool {
    fn new(conns: Vec<Connection>) -> Self {
        Self {
            conns: Mutex::new(conns),
            available: Condvar::new(),
        }
    }

    fn checkout(&self) -> Result<ReaderGuard<'_>, EngineError> {
        let mut guard = self
            .conns
            .lock()
            .map_err(|_| EngineError::internal("sqlite reader pool poisoned"))?;
        loop {
            if let Some(conn) = guard.pop() {
                return Ok(ReaderGuard {
                    pool: self,
                    conn: Some(conn),
                });
            }
            guard = self
                .available
                .wait(guard)
                .map_err(|_| EngineError::internal("sqlite reader pool poisoned"))?;
        }
    }
}

struct ReaderGuard<'a> {
    pool: &'a ReaderPool,
    conn: Option<Connection>,
}

impl Drop for ReaderGuard<'_> {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            if let Ok(mut guard) = self.pool.conns.lock() {
                guard.push(conn);
                self.pool.available.notify_one();
            }
        }
    }
}

pub struct SqlitePool {
    path: PathBuf,
    in_memory: bool,
    writer: Arc<Mutex<Connection>>,
    readers: Option<ReaderPool>,
}

impl SqlitePool {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, EngineError> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|e| {
                    tracing::error!(dir = %parent.display(), error = %e, "failed creating db dir");
                    EngineError::internal("database initialization failed")
                })?;
            }
        }

        let writer = Connection::open(&path).map_err(|e| map_sqlite_err(e, "open writer"))?;
        configure_writer(&writer)?;

        let mut reader_conns = Vec::with_capacity(DEFAULT_READER_POOL_SIZE);
        for _ in 0..DEFAULT_READER_POOL_SIZE {
            let reader = Connection::open(&path).map_err(|e| map_sqlite_err(e, "open reader"))?;
            configure_reader(&reader)?;
            reader_conns.push(reader);
        }

        Ok(Self {
            path,
            in_memory: false,
            writer: Arc::new(Mutex::new(writer)),
            readers: Some(ReaderPool::new(reader_conns)),
        })
    }

    pub fn open_in_memory() -> Result<Self, EngineError> {
        let writer =
            Connection::open_in_memory().map_err(|e| map_sqlite_err(e, "open memory sqlite"))?;
        configure_memory(&writer)?;
        Ok(Self {
            path: PathBuf::from(":memory:"),
            in_memory: true,
            writer: Arc::new(Mutex::new(writer)),
            readers: None,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn is_in_memory(&self) -> bool {
        self.in_memory
    }

    pub fn with_writer<T, F>(&self, f: F) -> Result<T, EngineError>
    where
        F: FnOnce(&mut Connection) -> Result<T, EngineError>,
    {
        let mut conn = self
            .writer
            .lock()
            .map_err(|_| EngineError::internal("database busy"))?;
        f(&mut conn)
    }

    pub fn with_reader<T, F>(&self, f: F) -> Result<T, EngineError>
    where
        F: FnOnce(&Connection) -> Result<T, EngineError>,
    {
        if let Some(ref readers) = self.readers {
            let guard = readers.checkout()?;
            let conn = guard
                .conn
                .as_ref()
                .ok_or_else(|| EngineError::internal("missing reader connection"))?;
            return f(conn);
        }
        let conn = self
            .writer
            .lock()
            .map_err(|_| EngineError::internal("database busy"))?;
        f(&conn)
    }
}

impl HealthProbe for SqlitePool {
    fn probe_storage(&self) -> Result<StorageHealth, EngineError> {
        let (db_mode, writable) = self.with_writer(|conn| {
            let mode: String = conn
                .pragma_query_value(None, "journal_mode", |r| r.get(0))
                .map_err(|e| map_sqlite_err(e, "query journal_mode"))?;
            conn.execute_batch("BEGIN IMMEDIATE; ROLLBACK;")
                .map_err(|e| map_sqlite_err(e, "write probe"))?;
            Ok((mode, true))
        })?;

        let schema_ver: i64 = self.with_reader(|conn| {
            conn.query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_ver",
                [],
                |row| row.get(0),
            )
            .map_err(|e| map_sqlite_err(e, "probe schema_ver"))
        })?;

        Ok(StorageHealth {
            persistent: !self.in_memory,
            writable,
            db_mode,
            schema_ver,
            expected_schema_ver: CURRENT_SCHEMA_VERSION,
        })
    }
}

pub async fn run_blocking<T, F>(f: F) -> Result<T, EngineError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, EngineError> + Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "blocking storage task failed");
            EngineError::internal("storage operation failed")
        })?
}

fn configure_writer(conn: &Connection) -> Result<(), EngineError> {
    conn.busy_timeout(Duration::from_millis(5000))
        .map_err(|e| map_sqlite_err(e, "busy_timeout"))?;
    let _: String = conn
        .query_row("PRAGMA journal_mode = WAL", [], |r| r.get(0))
        .map_err(|e| map_sqlite_err(e, "pragma journal_mode"))?;
    conn.pragma_update(None, "synchronous", "NORMAL")
        .map_err(|e| map_sqlite_err(e, "pragma synchronous"))?;
    conn.pragma_update(None, "mmap_size", 67_108_864_i64)
        .map_err(|e| map_sqlite_err(e, "pragma mmap_size"))?;
    conn.pragma_update(None, "cache_size", -16_000_i64)
        .map_err(|e| map_sqlite_err(e, "pragma cache_size"))?;
    conn.pragma_update(None, "temp_store", "MEMORY")
        .map_err(|e| map_sqlite_err(e, "pragma temp_store"))?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|e| map_sqlite_err(e, "pragma foreign_keys"))?;
    Ok(())
}

fn configure_reader(conn: &Connection) -> Result<(), EngineError> {
    conn.busy_timeout(Duration::from_millis(5000))
        .map_err(|e| map_sqlite_err(e, "reader busy_timeout"))?;
    conn.pragma_update(None, "query_only", "ON")
        .map_err(|e| map_sqlite_err(e, "reader query_only"))?;
    conn.pragma_update(None, "cache_size", -16_000_i64)
        .map_err(|e| map_sqlite_err(e, "reader cache_size"))?;
    conn.pragma_update(None, "temp_store", "MEMORY")
        .map_err(|e| map_sqlite_err(e, "reader temp_store"))?;
    Ok(())
}

fn configure_memory(conn: &Connection) -> Result<(), EngineError> {
    conn.busy_timeout(Duration::from_millis(5000))
        .map_err(|e| map_sqlite_err(e, "memory busy_timeout"))?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|e| map_sqlite_err(e, "memory foreign_keys"))?;
    Ok(())
}

pub fn map_sqlite_err(err: rusqlite::Error, context: &str) -> EngineError {
    if let rusqlite::Error::SqliteFailure(ref f, _) = err {
        if f.code == rusqlite::ErrorCode::ConstraintViolation {
            return EngineError::conflict("resource already exists or violates constraint");
        }
    }
    tracing::error!(%context, error = %err, "sqlite operation failed");
    EngineError::internal("database operation failed")
}
