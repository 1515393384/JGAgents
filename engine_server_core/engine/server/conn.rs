use std::collections::HashMap;
use std::sync::RwLock;

use crate::core::error::EngineError;

pub const MAX_CONNS: usize = 5000;

#[derive(Clone, Debug)]
pub struct ConnRecord {
    pub id: String,
    pub user_id: String,
    pub user_name: String,
    pub conv_id: String,
    pub ip: String,
    pub connected_at_ms: i64,
    pub inflight: bool,
}

#[derive(Clone, Debug)]
pub struct ConnSnapshot {
    pub id: String,
    pub user_id: String,
    pub user_name: String,
    pub conv_id: String,
    pub ip: String,
    pub connected_at_ms: i64,
    pub inflight: bool,
}

#[derive(Default)]
pub struct ConnRegistry {
    items: RwLock<HashMap<String, ConnRecord>>,
}

impl ConnRegistry {
    pub fn register(&self, record: ConnRecord) -> Result<(), EngineError> {
        let mut guard = self
            .items
            .write()
            .map_err(|_| EngineError::internal("conn registry poisoned"))?;
        if guard.len() >= MAX_CONNS && !guard.contains_key(&record.id) {
            return Err(EngineError::budget("max active connections reached"));
        }
        guard.insert(record.id.clone(), record);
        Ok(())
    }

    pub fn set_inflight(&self, id: &str, inflight: bool) -> Result<(), EngineError> {
        let mut guard = self
            .items
            .write()
            .map_err(|_| EngineError::internal("conn registry poisoned"))?;
        if let Some(item) = guard.get_mut(id) {
            item.inflight = inflight;
        }
        Ok(())
    }

    pub fn remove(&self, id: &str) -> Result<(), EngineError> {
        let mut guard = self
            .items
            .write()
            .map_err(|_| EngineError::internal("conn registry poisoned"))?;
        guard.remove(id);
        Ok(())
    }

    pub fn len(&self) -> Result<usize, EngineError> {
        let guard = self
            .items
            .read()
            .map_err(|_| EngineError::internal("conn registry poisoned"))?;
        Ok(guard.len())
    }

    pub fn snapshot(&self) -> Result<Vec<ConnSnapshot>, EngineError> {
        let guard = self
            .items
            .read()
            .map_err(|_| EngineError::internal("conn registry poisoned"))?;
        let mut list: Vec<&ConnRecord> = guard.values().collect();
        list.sort_by(|a, b| b.connected_at_ms.cmp(&a.connected_at_ms));
        Ok(list
            .into_iter()
            .map(|c| ConnSnapshot {
                id: c.id.clone(),
                user_id: c.user_id.clone(),
                user_name: c.user_name.clone(),
                conv_id: c.conv_id.clone(),
                ip: c.ip.clone(),
                connected_at_ms: c.connected_at_ms,
                inflight: c.inflight,
            })
            .collect())
    }
}
