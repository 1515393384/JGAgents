use crate::core::error::EngineError;
use crate::core::ids::{ConvId, EndpointId, TokenHash, UserId};
use crate::engine::user::model::UserRecord;

pub trait Clock: Send + Sync {
    fn now_ms(&self) -> i64;
}

pub trait UserStore: Send + Sync {
    fn list_all(&self) -> Result<Vec<UserRecord>, EngineError>;
    fn get_by_id(&self, id: &UserId) -> Result<Option<UserRecord>, EngineError>;
    fn get_by_token_hash(&self, hash: &TokenHash) -> Result<Option<UserRecord>, EngineError>;
    fn insert(&self, user: &UserRecord) -> Result<(), EngineError>;
    fn update(&self, user: &UserRecord) -> Result<(), EngineError>;
    fn delete(&self, id: &UserId) -> Result<bool, EngineError>;
}

pub trait ConvStore: Send + Sync {
    fn append(&self, user: &UserId, conv: &ConvId, payload: &[u8], now_ms: i64) -> Result<u64, EngineError>;
    fn load_since(&self, user: &UserId, conv: &ConvId, since_seq: u64) -> Result<Vec<(u64, Vec<u8>)>, EngineError>;
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
}

pub trait Upstream: Send + Sync {
    fn dispatch(
        &self,
        endpoint: &EndpointId,
        body: &[u8],
    ) -> Result<Vec<u8>, EngineError>;
}
