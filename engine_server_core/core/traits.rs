use crate::core::error::EngineError;
use crate::core::ids::{ConvId, EndpointId, TokenHash, UserId};
use crate::doc::model::{
    DocDetail, DocNodeRecord, DocRecord, DocRevisionMeta, DocSearchHit, ExportMode, ParsedDocx,
};
use crate::engine::user::model::{
    SessionRecord, UpdateUserInput, UserMetrics, UserPage, UserQuery, UserRecord, UserState,
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
    fn find_by_name(&self, name: &str) -> Result<Vec<UserRecord>, EngineError>;
    fn get_by_token_hash(&self, hash: &TokenHash) -> Result<Option<UserRecord>, EngineError>;
    fn insert(&self, user: &UserRecord) -> Result<(), EngineError>;
    fn register_member_atomic(&self, user: &UserRecord) -> Result<(), EngineError>;
    fn touch_last_active(&self, id: &UserId, now_ms: i64) -> Result<(), EngineError>;
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

pub trait SessionStore: Send + Sync {
    fn create(&self, session: &SessionRecord) -> Result<(), EngineError>;
    fn get_valid(&self, token_hash: &TokenHash, now_ms: i64)
        -> Result<Option<SessionRecord>, EngineError>;
    fn delete(&self, token_hash: &TokenHash) -> Result<(), EngineError>;
    fn delete_for_user(&self, user_id: &UserId) -> Result<(), EngineError>;
}

pub trait DocStore: Send + Sync {
    fn create_doc(
        &self,
        doc: &DocRecord,
        rev: &DocRevisionMeta,
        docx_blob: &[u8],
        parsed: &ParsedDocx,
    ) -> Result<DocDetail, EngineError>;
    #[allow(clippy::too_many_arguments)]
    fn append_revision(
        &self,
        doc_id: &str,
        owner_id: &UserId,
        note: &str,
        author_name: &str,
        export_mode: ExportMode,
        docx_blob: &[u8],
        parsed: &ParsedDocx,
        carry_locks_from_rev: Option<i64>,
        now_ms: i64,
    ) -> Result<DocDetail, EngineError>;
    fn list_docs(
        &self,
        owner_id: &UserId,
        space_id: Option<&str>,
    ) -> Result<Vec<DocRecord>, EngineError>;
    fn get_doc_detail(
        &self,
        doc_id: &str,
        owner_id: &UserId,
        rev: Option<i64>,
    ) -> Result<DocDetail, EngineError>;
    fn get_revision_blob(
        &self,
        doc_id: &str,
        owner_id: &UserId,
        rev: i64,
    ) -> Result<(DocRecord, DocRevisionMeta, Vec<u8>), EngineError>;
    fn get_revision_nodes(
        &self,
        doc_id: &str,
        owner_id: &UserId,
        rev: i64,
    ) -> Result<Vec<DocNodeRecord>, EngineError>;
    fn set_node_locked(
        &self,
        doc_id: &str,
        owner_id: &UserId,
        anchor_id: &str,
        locked: bool,
        now_ms: i64,
    ) -> Result<DocNodeRecord, EngineError>;
    fn search_doc_fts(
        &self,
        doc_id: &str,
        owner_id: &UserId,
        query: &str,
        limit: usize,
    ) -> Result<Vec<DocSearchHit>, EngineError>;
    /// 取某版本的渲染切片。`from_seq`/`to_seq` 按块序裁剪，缺省从头取一页量。
    fn get_render_slice(
        &self,
        doc_id: &str,
        owner_id: &UserId,
        rev: Option<i64>,
        from_seq: Option<usize>,
        to_seq: Option<usize>,
    ) -> Result<crate::doc::render::RenderSlice, EngineError>;
    /// 取某版本 docx 包内的图片。`rev` 缺省取当前版本。
    fn get_doc_media(
        &self,
        doc_id: &str,
        owner_id: &UserId,
        rev: Option<i64>,
        path: &str,
    ) -> Result<Option<crate::doc::media::MediaEntry>, EngineError>;
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
