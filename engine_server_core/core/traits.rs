use crate::core::error::EngineError;
use crate::core::ids::{ConvId, EndpointId, TokenHash, UserId};
use crate::engine::user::model::{
    UpdateUserInput, UserMetrics, UserPage, UserQuery, UserRecord, UserState,
};

pub trait Clock: Send + Sync {
    fn now_ms(&self) -> i64;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchiveRecord {
    pub conv_id: String,
    pub owner_name: String,
    pub group: String,
    pub messages: u64,
    pub prefix_version: u32,
    pub bytes: u64,
    pub updated_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DailyTrendRow {
    pub day_ms: i64,
    pub requests: u64,
    pub prompt_tokens: u64,
    pub cached_tokens: u64,
    pub completion_tokens: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StorageHealth {
    pub persistent: bool,
    pub writable: bool,
    pub db_mode: String,
    pub schema_ver: i64,
    pub expected_schema_ver: i64,
}

pub trait HealthProbe: Send + Sync {
    fn probe_storage(&self) -> Result<StorageHealth, EngineError>;
}

pub trait UserStore: Send + Sync {
    fn is_empty(&self) -> Result<bool, EngineError>;
    fn query(&self, query: &UserQuery) -> Result<UserPage, EngineError>;
    fn metrics(&self) -> Result<UserMetrics, EngineError>;
    fn get_by_id(&self, id: &UserId) -> Result<Option<UserRecord>, EngineError>;
    fn get_by_token_hash(&self, hash: &TokenHash) -> Result<Option<UserRecord>, EngineError>;
    fn insert(&self, user: &UserRecord) -> Result<(), EngineError>;
    fn update_atomic(&self, id: &UserId, patch: &UpdateUserInput)
        -> Result<UserRecord, EngineError>;
    fn rotate_token_atomic(
        &self,
        id: &UserId,
        token_hash: &TokenHash,
        token_hint: &str,
    ) -> Result<UserRecord, EngineError>;
    fn batch_set_state_atomic(
        &self,
        ids: &[UserId],
        state: UserState,
    ) -> Result<usize, EngineError>;
    fn delete_atomic(&self, id: &UserId) -> Result<(), EngineError>;
}

pub trait ConvStore: Send + Sync {
    fn append(
        &self,
        user: &UserId,
        conv: &ConvId,
        payload: &[u8],
        now_ms: i64,
    ) -> Result<u64, EngineError>;
    fn load_since(
        &self,
        user: &UserId,
        conv: &ConvId,
        since_seq: u64,
    ) -> Result<Vec<(u64, Vec<u8>)>, EngineError>;
    fn list_archives(&self, limit: usize) -> Result<Vec<ArchiveRecord>, EngineError>;
}

pub trait LedgerStore: Send + Sync {
    fn record_usage(
        &self,
        user: &UserId,
        conv: &ConvId,
        prompt_tokens: u64,
        cached_tokens: u64,
        completion_tokens: u64,
        now_ms: i64,
    ) -> Result<(), EngineError>;
    fn daily_trend(&self, now_ms: i64, days: usize) -> Result<Vec<DailyTrendRow>, EngineError>;
}

pub trait Upstream: Send + Sync {
    fn dispatch(
        &self,
        endpoint: &EndpointId,
        body: &[u8],
    ) -> Result<Vec<u8>, EngineError>;
}
