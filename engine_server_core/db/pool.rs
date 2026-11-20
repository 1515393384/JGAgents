use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rusqlite::Connection;

use crate::core::error::EngineError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PoolHealth {
    pub writable: bool,
    pub journal_mode: String,
    pub path: String,
}

pub struct SqlitePool {
    path: PathBuf,
    in_memory: bool,
    writer: Arc<Mutex<Connection>>,
    reader: Arc<Mutex<Connection>>,
}

impl SqlitePool {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, EngineError> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|e| EngineError::internal(format!("create db dir: {e}")))?;
            }
        }

        let writer = Connection::open(&path)
            .map_err(|e| EngineError::internal(format!("open sqlite writer: {e}")))?;
        configure_writer(&writer)?;

        let reader = Connection::open(&path)
            .map_err(|e| EngineError::internal(format!("open sqlite reader: {e}")))?;
        configure_reader(&reader)?;

        Ok(Self {
            path,
            in_memory: false,
            writer: Arc::new(Mutex::new(writer)),
            reader: Arc::new(Mutex::new(reader)),
        })
    }

    pub fn open_in_memory() -> Result<Self, EngineError> {
        let writer = Connection::open_in_memory()
            .map_err(|e| EngineError::internal(format!("open memory sqlite: {e}")))?;
        configure_memory(&writer)?;
        let shared = Arc::new(Mutex::new(writer));
        Ok(Self {
            path: PathBuf::from(":memory:"),
            in_memory: true,
            writer: Arc::clone(&shared),
            reader: shared,
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
            .map_err(|_| EngineError::internal("sqlite writer lock poisoned"))?;
        f(&mut conn)
    }

    pub fn with_reader<T, F>(&self, f: F) -> Result<T, EngineError>
    where
        F: FnOnce(&Connection) -> Result<T, EngineError>,
    {
        let conn = self
            .reader
            .lock()
            .map_err(|_| EngineError::internal("sqlite reader lock poisoned"))?;
        f(&conn)
    }

    pub fn health_snapshot(&self) -> PoolHealth {
        let res = self.with_writer(|conn| {
            let mode: String = conn
                .pragma_query_value(None, "journal_mode", |r| r.get(0))
                .map_err(|e| EngineError::internal(format!("query journal_mode: {e}")))?;
            conn.execute_batch("BEGIN IMMEDIATE; ROLLBACK;")
                .map_err(|e| EngineError::internal(format!("write probe: {e}")))?;
            Ok(mode)
        });

        match res {
            Ok(journal_mode) => PoolHealth {
                writable: true,
                journal_mode,
                path: self.path.display().to_string(),
            },
            Err(_) => PoolHealth {
                writable: false,
                journal_mode: String::from("unknown"),
                path: self.path.display().to_string(),
            },
        }
    }
}

fn configure_writer(conn: &Connection) -> Result<(), EngineError> {
    conn.busy_timeout(Duration::from_millis(5000))
        .map_err(|e| EngineError::internal(format!("busy_timeout: {e}")))?;
    let _: String = conn
        .query_row("PRAGMA journal_mode = WAL", [], |r| r.get(0))
        .map_err(|e| EngineError::internal(format!("pragma journal_mode: {e}")))?;
    conn.pragma_update(None, "synchronous", "NORMAL")
        .map_err(|e| EngineError::internal(format!("pragma synchronous: {e}")))?;
    conn.pragma_update(None, "mmap_size", 67_108_864_i64)
        .map_err(|e| EngineError::internal(format!("pragma mmap_size: {e}")))?;
    conn.pragma_update(None, "cache_size", -16_000_i64)
        .map_err(|e| EngineError::internal(format!("pragma cache_size: {e}")))?;
    conn.pragma_update(None, "temp_store", "MEMORY")
        .map_err(|e| EngineError::internal(format!("pragma temp_store: {e}")))?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|e| EngineError::internal(format!("pragma foreign_keys: {e}")))?;
    Ok(())
}

fn configure_reader(conn: &Connection) -> Result<(), EngineError> {
    conn.busy_timeout(Duration::from_millis(5000))
        .map_err(|e| EngineError::internal(format!("reader busy_timeout: {e}")))?;
    conn.pragma_update(None, "query_only", "ON")
        .map_err(|e| EngineError::internal(format!("reader query_only: {e}")))?;
    conn.pragma_update(None, "cache_size", -16_000_i64)
        .map_err(|e| EngineError::internal(format!("reader cache_size: {e}")))?;
    conn.pragma_update(None, "temp_store", "MEMORY")
        .map_err(|e| EngineError::internal(format!("reader temp_store: {e}")))?;
    Ok(())
}

fn configure_memory(conn: &Connection) -> Result<(), EngineError> {
    conn.busy_timeout(Duration::from_millis(5000))
        .map_err(|e| EngineError::internal(format!("memory busy_timeout: {e}")))?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|e| EngineError::internal(format!("memory foreign_keys: {e}")))?;
    Ok(())
}

pub fn map_sqlite_err(err: rusqlite::Error, context: &str) -> EngineError {
    if let rusqlite::Error::SqliteFailure(ref f, _) = err {
        if f.code == rusqlite::ErrorCode::ConstraintViolation {
            return EngineError::conflict(format!("{context}: constraint conflict"));
        }
    }
    EngineError::internal(format!("{context}: {err}"))
}
