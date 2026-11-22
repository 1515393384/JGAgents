use serde::{Deserialize, Serialize};

use crate::core::ids::UserId;

pub const MAX_DOCX_UPLOAD_BYTES: usize = 50 * 1024 * 1024;
pub const MAX_ZIP_ENTRIES: usize = 4096;
pub const MAX_XML_UNCOMPRESSED_BYTES: u64 = 32 * 1024 * 1024;
pub const MAX_TOTAL_UNCOMPRESSED_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Heading,
    Paragraph,
    ListItem,
    Table,
    TableChunk,
}

impl NodeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Heading => "heading",
            Self::Paragraph => "paragraph",
            Self::ListItem => "list_item",
            Self::Table => "table",
            Self::TableChunk => "table_chunk",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim() {
            "heading" => Some(Self::Heading),
            "paragraph" => Some(Self::Paragraph),
            "list_item" => Some(Self::ListItem),
            "table" => Some(Self::Table),
            "table_chunk" => Some(Self::TableChunk),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportMode {
    Redline,
    Clean,
    Raw,
}

impl ExportMode {
    pub fn parse(raw: Option<&str>) -> Self {
        match raw.unwrap_or("clean").trim().to_ascii_lowercase().as_str() {
            "redline" | "track" | "track_changes" => Self::Redline,
            "raw" => Self::Raw,
            _ => Self::Clean,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Redline => "redline",
            Self::Clean => "clean",
            Self::Raw => "raw",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocRecord {
    pub id: String,
    pub owner_user_id: UserId,
    pub space_id: String,
    pub title: String,
    pub filename: String,
    pub current_rev: i64,
    pub source_format: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocRevisionMeta {
    pub doc_id: String,
    pub rev: i64,
    pub note: String,
    pub author_name: String,
    pub export_mode: String,
    pub node_count: usize,
    pub table_count: usize,
    pub mandatory_count: usize,
    pub byte_size: usize,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocNodeRecord {
    pub ord: usize,
    pub anchor_id: String,
    pub parent_anchor_id: Option<String>,
    pub explicit_anchor: Option<String>,
    pub xml_locator: String,
    pub node_kind: NodeKind,
    pub heading_path: String,
    pub text_content: String,
    pub text_hash: String,
    pub row_keys: Vec<String>,
    pub mandatory_tags: Vec<String>,
    pub locked: bool,
    pub token_estimate: usize,
    pub is_search_chunk: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocRefRecord {
    pub source_anchor: String,
    pub target_hint: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocDetail {
    pub doc: DocRecord,
    pub revisions: Vec<DocRevisionMeta>,
    pub nodes: Vec<DocNodeRecord>,
    pub refs: Vec<DocRefRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocSearchHit {
    pub hit_anchor_id: String,
    pub hit_text: String,
    pub heading_path: String,
    pub parent_node: Option<DocNodeRecord>,
    pub related_refs: Vec<DocRefRecord>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffKind {
    Equal,
    Modified,
    Inserted,
    Deleted,
    Moved,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpanOp {
    Equal,
    Insert,
    Delete,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffSpan {
    pub op: SpanOp,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableRowDiff {
    pub row_key: String,
    pub op: SpanOp,
    pub before_row: Option<String>,
    pub after_row: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffBlock {
    pub anchor_id: String,
    pub heading_path: String,
    pub node_kind: NodeKind,
    pub diff_kind: DiffKind,
    pub before_text: String,
    pub after_text: String,
    pub spans: Vec<DiffSpan>,
    pub added_chars: usize,
    pub deleted_chars: usize,
    pub locked: bool,
    pub table_row_diffs: Vec<TableRowDiff>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocDiffReport {
    pub doc_id: String,
    pub from_rev: i64,
    pub to_rev: i64,
    pub added_chars: usize,
    pub deleted_chars: usize,
    pub changed_blocks: usize,
    pub locked_conflicts: Vec<String>,
    pub blocks: Vec<DiffBlock>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchEditInput {
    pub anchor_id: String,
    pub new_text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedDocx {
    pub nodes: Vec<DocNodeRecord>,
    pub refs: Vec<DocRefRecord>,
    pub node_count: usize,
    pub table_count: usize,
    pub mandatory_count: usize,
    pub has_revisions: bool,
}
