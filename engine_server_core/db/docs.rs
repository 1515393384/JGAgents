use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use rusqlite::{OptionalExtension, Row, Transaction, TransactionBehavior};

use crate::core::error::EngineError;
use crate::core::ids::UserId;
use crate::core::traits::DocStore;
use crate::db::pool::{map_sqlite_err, SqlitePool};
use crate::doc::model::{
    DocDetail, DocNodeRecord, DocRecord, DocRefRecord, DocRevisionMeta, DocSearchHit, ExportMode,
    NodeKind, ParsedDocx,
};

pub struct SqliteDocStore {
    pool: Arc<SqlitePool>,
}

impl SqliteDocStore {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

fn row_to_doc(row: &Row<'_>) -> rusqlite::Result<DocRecord> {
    let id: String = row.get(0)?;
    let owner: String = row.get(1)?;
    let space_id: String = row.get(2)?;
    let title: String = row.get(3)?;
    let filename: String = row.get(4)?;
    let current_rev: i64 = row.get(5)?;
    let source_format: String = row.get(6)?;
    let created_at_ms: i64 = row.get(7)?;
    let updated_at_ms: i64 = row.get(8)?;
    Ok(DocRecord {
        id,
        owner_user_id: UserId::new(owner),
        space_id,
        title,
        filename,
        current_rev,
        source_format,
        created_at_ms,
        updated_at_ms,
    })
}

fn row_to_rev(row: &Row<'_>) -> rusqlite::Result<DocRevisionMeta> {
    let doc_id: String = row.get(0)?;
    let rev: i64 = row.get(1)?;
    let note: String = row.get(2)?;
    let author_name: String = row.get(3)?;
    let export_mode: String = row.get(4)?;
    let node_count: i64 = row.get(5)?;
    let table_count: i64 = row.get(6)?;
    let mandatory_count: i64 = row.get(7)?;
    let byte_size: i64 = row.get(8)?;
    let created_at_ms: i64 = row.get(9)?;
    Ok(DocRevisionMeta {
        doc_id,
        rev,
        note,
        author_name,
        export_mode,
        node_count: node_count.max(0) as usize,
        table_count: table_count.max(0) as usize,
        mandatory_count: mandatory_count.max(0) as usize,
        byte_size: byte_size.max(0) as usize,
        created_at_ms,
    })
}

fn row_to_node(row: &Row<'_>) -> rusqlite::Result<DocNodeRecord> {
    let ord: i64 = row.get(0)?;
    let anchor_id: String = row.get(1)?;
    let parent_anchor_id: Option<String> = row.get(2)?;
    let explicit_anchor: Option<String> = row.get(3)?;
    let xml_locator: String = row.get(4)?;
    let node_kind_raw: String = row.get(5)?;
    let heading_path: String = row.get(6)?;
    let text_content: String = row.get(7)?;
    let text_hash: String = row.get(8)?;
    let row_keys_raw: String = row.get(9)?;
    let mandatory_raw: String = row.get(10)?;
    let locked_i: i64 = row.get(11)?;
    let token_est: i64 = row.get(12)?;
    let is_chunk_i: i64 = row.get(13)?;

    let row_keys: Vec<String> = serde_json::from_str(&row_keys_raw).unwrap_or_default();
    let mandatory_tags: Vec<String> = serde_json::from_str(&mandatory_raw).unwrap_or_default();
    let node_kind = NodeKind::parse(&node_kind_raw).unwrap_or(NodeKind::Paragraph);

    Ok(DocNodeRecord {
        ord: ord.max(0) as usize,
        anchor_id,
        parent_anchor_id,
        explicit_anchor,
        xml_locator,
        node_kind,
        heading_path,
        text_content,
        text_hash,
        row_keys,
        mandatory_tags,
        locked: locked_i != 0,
        token_estimate: token_est.max(0) as usize,
        is_search_chunk: is_chunk_i != 0,
    })
}

fn insert_nodes_and_fts(
    tx: &Transaction<'_>,
    doc_id: &str,
    rev: i64,
    parsed: &ParsedDocx,
    locked_anchors: &HashSet<String>,
) -> Result<(), EngineError> {
    {
        let mut node_stmt = tx
            .prepare(
                "INSERT INTO doc_nodes (
                    doc_id, rev, ord, anchor_id, parent_anchor_id, explicit_anchor,
                    xml_locator, node_kind, heading_path, text_content, text_hash,
                    row_keys_json, mandatory_tags, locked, token_estimate, is_search_chunk
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            )
            .map_err(|e| map_sqlite_err(e, "prepare insert doc_nodes"))?;

        let mut fts_stmt = tx
            .prepare(
                "INSERT INTO doc_fts (doc_id, rev, anchor_id, parent_anchor_id, heading_path, text_content)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )
            .map_err(|e| map_sqlite_err(e, "prepare insert doc_fts"))?;

        for n in &parsed.nodes {
            let row_keys_json = serde_json::to_string(&n.row_keys).unwrap_or_else(|_| "[]".into());
            let mandatory_json =
                serde_json::to_string(&n.mandatory_tags).unwrap_or_else(|_| "[]".into());
            let locked = if n.locked
                || locked_anchors.contains(&n.anchor_id)
                || n.explicit_anchor
                    .as_ref()
                    .map(|ea| locked_anchors.contains(ea))
                    .unwrap_or(false)
            {
                1i64
            } else {
                0i64
            };

            node_stmt
                .execute(rusqlite::params![
                    doc_id,
                    rev,
                    n.ord as i64,
                    n.anchor_id,
                    n.parent_anchor_id,
                    n.explicit_anchor,
                    n.xml_locator,
                    n.node_kind.as_str(),
                    n.heading_path,
                    n.text_content,
                    n.text_hash,
                    row_keys_json,
                    mandatory_json,
                    locked,
                    n.token_estimate as i64,
                    if n.is_search_chunk { 1i64 } else { 0i64 },
                ])
                .map_err(|e| map_sqlite_err(e, "insert doc_node"))?;

            fts_stmt
                .execute(rusqlite::params![
                    doc_id,
                    rev,
                    n.anchor_id,
                    n.parent_anchor_id.as_deref().unwrap_or(""),
                    n.heading_path,
                    n.text_content,
                ])
                .map_err(|e| map_sqlite_err(e, "insert doc_fts"))?;
        }
    }

    {
        let mut ref_stmt = tx
            .prepare(
                "INSERT INTO doc_refs (doc_id, rev, source_anchor, target_hint) VALUES (?1, ?2, ?3, ?4)",
            )
            .map_err(|e| map_sqlite_err(e, "prepare insert doc_refs"))?;
        for r in &parsed.refs {
            ref_stmt
                .execute(rusqlite::params![
                    doc_id,
                    rev,
                    r.source_anchor,
                    r.target_hint
                ])
                .map_err(|e| map_sqlite_err(e, "insert doc_ref"))?;
        }
    }

    Ok(())
}

fn load_detail_with_conn(
    conn: &rusqlite::Connection,
    doc_id: &str,
    owner_id: &UserId,
    rev_opt: Option<i64>,
) -> Result<DocDetail, EngineError> {
    let doc = conn
        .query_row(
            "SELECT id, owner_user_id, space_id, title, filename, current_rev, source_format, created_at_ms, updated_at_ms
             FROM docs WHERE id = ?1",
            [doc_id],
            row_to_doc,
        )
        .optional()
        .map_err(|e| map_sqlite_err(e, "query doc"))?
        .ok_or_else(|| EngineError::not_found("文档不存在"))?;

    if &doc.owner_user_id != owner_id {
        return Err(EngineError::forbidden("无权访问该文档"));
    }

    let target_rev = rev_opt.unwrap_or(doc.current_rev);

    let mut rev_stmt = conn
        .prepare(
            "SELECT doc_id, rev, note, author_name, export_mode, node_count, table_count, mandatory_count, byte_size, created_at_ms
             FROM doc_revisions WHERE doc_id = ?1 ORDER BY rev DESC",
        )
        .map_err(|e| map_sqlite_err(e, "prepare revisions"))?;
    let revisions = rev_stmt
        .query_map([doc_id], row_to_rev)
        .map_err(|e| map_sqlite_err(e, "query revisions"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| map_sqlite_err(e, "read revision"))?;

    let mut node_stmt = conn
        .prepare(
            "SELECT ord, anchor_id, parent_anchor_id, explicit_anchor, xml_locator, node_kind, heading_path, text_content, text_hash, row_keys_json, mandatory_tags, locked, token_estimate, is_search_chunk
             FROM doc_nodes WHERE doc_id = ?1 AND rev = ?2 ORDER BY ord ASC",
        )
        .map_err(|e| map_sqlite_err(e, "prepare doc_nodes"))?;
    let nodes = node_stmt
        .query_map(rusqlite::params![doc_id, target_rev], row_to_node)
        .map_err(|e| map_sqlite_err(e, "query doc_nodes"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| map_sqlite_err(e, "read doc_node"))?;

    let mut ref_stmt = conn
        .prepare(
            "SELECT source_anchor, target_hint FROM doc_refs WHERE doc_id = ?1 AND rev = ?2",
        )
        .map_err(|e| map_sqlite_err(e, "prepare doc_refs"))?;
    let refs = ref_stmt
        .query_map(rusqlite::params![doc_id, target_rev], |r| {
            Ok(DocRefRecord {
                source_anchor: r.get(0)?,
                target_hint: r.get(1)?,
            })
        })
        .map_err(|e| map_sqlite_err(e, "query doc_refs"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| map_sqlite_err(e, "read doc_ref"))?;

    Ok(DocDetail {
        doc,
        revisions,
        nodes,
        refs,
    })
}

fn sanitize_fts5_query(raw: &str) -> String {
    let tokens: Vec<String> = raw
        .split_whitespace()
        .map(|t| t.replace('"', ""))
        .filter(|t| !t.is_empty())
        .map(|t| format!("\"{t}\""))
        .collect();
    tokens.join(" ")
}

fn escape_like(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if matches!(ch, '\\' | '%' | '_') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

impl DocStore for SqliteDocStore {
    fn create_doc(
        &self,
        doc: &DocRecord,
        rev: &DocRevisionMeta,
        docx_blob: &[u8],
        parsed: &ParsedDocx,
    ) -> Result<DocDetail, EngineError> {
        self.pool.with_writer(|conn| {
            let tx = conn
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|e| map_sqlite_err(e, "begin create_doc"))?;

            tx.execute(
                "INSERT INTO docs (id, owner_user_id, space_id, title, filename, current_rev, source_format, created_at_ms, updated_at_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    doc.id,
                    doc.owner_user_id.as_str(),
                    doc.space_id,
                    doc.title,
                    doc.filename,
                    doc.current_rev,
                    doc.source_format,
                    doc.created_at_ms,
                    doc.updated_at_ms,
                ],
            )
            .map_err(|e| map_sqlite_err(e, "insert doc"))?;

            tx.execute(
                "INSERT INTO doc_revisions (doc_id, rev, note, author_name, export_mode, docx_blob, node_count, table_count, mandatory_count, byte_size, created_at_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                rusqlite::params![
                    doc.id,
                    rev.rev,
                    rev.note,
                    rev.author_name,
                    rev.export_mode,
                    docx_blob,
                    rev.node_count as i64,
                    rev.table_count as i64,
                    rev.mandatory_count as i64,
                    rev.byte_size as i64,
                    rev.created_at_ms,
                ],
            )
            .map_err(|e| map_sqlite_err(e, "insert initial doc_revision"))?;

            let empty_locks = HashSet::new();
            insert_nodes_and_fts(&tx, &doc.id, rev.rev, parsed, &empty_locks)?;

            tx.commit().map_err(|e| map_sqlite_err(e, "commit create_doc"))?;
            load_detail_with_conn(conn, &doc.id, &doc.owner_user_id, Some(rev.rev))
        })
    }

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
    ) -> Result<DocDetail, EngineError> {
        self.pool.with_writer(|conn| {
            let tx = conn
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|e| map_sqlite_err(e, "begin append_revision"))?;

            let (owner_raw, cur_rev): (String, i64) = tx
                .query_row(
                    "SELECT owner_user_id, current_rev FROM docs WHERE id = ?1",
                    [doc_id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()
                .map_err(|e| map_sqlite_err(e, "query doc for revision"))?
                .ok_or_else(|| EngineError::not_found("文档不存在"))?;

            if owner_raw != owner_id.as_str() {
                return Err(EngineError::forbidden("无权修改该文档"));
            }

            let source_lock_rev = carry_locks_from_rev.unwrap_or(cur_rev);
            let mut locked_anchors = HashSet::new();
            {
                let mut lstmt = tx
                    .prepare(
                        "SELECT anchor_id, explicit_anchor FROM doc_nodes WHERE doc_id = ?1 AND rev = ?2 AND locked = 1",
                    )
                    .map_err(|e| map_sqlite_err(e, "prepare carry locks"))?;
                let rows = lstmt
                    .query_map(rusqlite::params![doc_id, source_lock_rev], |r| {
                        let a: String = r.get(0)?;
                        let ea: Option<String> = r.get(1)?;
                        Ok((a, ea))
                    })
                    .map_err(|e| map_sqlite_err(e, "query carry locks"))?;
                for row in rows {
                    let (a, ea) = row.map_err(|e| map_sqlite_err(e, "read lock row"))?;
                    locked_anchors.insert(a);
                    if let Some(exp) = ea {
                        locked_anchors.insert(exp);
                    }
                }
            }

            let next_rev = cur_rev + 1;
            tx.execute(
                "INSERT INTO doc_revisions (doc_id, rev, note, author_name, export_mode, docx_blob, node_count, table_count, mandatory_count, byte_size, created_at_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                rusqlite::params![
                    doc_id,
                    next_rev,
                    note,
                    author_name,
                    export_mode.as_str(),
                    docx_blob,
                    parsed.node_count as i64,
                    parsed.table_count as i64,
                    parsed.mandatory_count as i64,
                    docx_blob.len() as i64,
                    now_ms,
                ],
            )
            .map_err(|e| map_sqlite_err(e, "insert doc_revision"))?;

            tx.execute(
                "UPDATE docs SET current_rev = ?2, updated_at_ms = ?3 WHERE id = ?1",
                rusqlite::params![doc_id, next_rev, now_ms],
            )
            .map_err(|e| map_sqlite_err(e, "update doc current_rev"))?;

            insert_nodes_and_fts(&tx, doc_id, next_rev, parsed, &locked_anchors)?;

            tx.commit().map_err(|e| map_sqlite_err(e, "commit append_revision"))?;
            load_detail_with_conn(conn, doc_id, owner_id, Some(next_rev))
        })
    }

    fn list_docs(
        &self,
        owner_id: &UserId,
        space_id: Option<&str>,
    ) -> Result<Vec<DocRecord>, EngineError> {
        self.pool.with_reader(|conn| {
            let mut out = Vec::new();
            if let Some(sp) = space_id.map(str::trim).filter(|s| !s.is_empty()) {
                let mut stmt = conn
                    .prepare(
                        "SELECT id, owner_user_id, space_id, title, filename, current_rev, source_format, created_at_ms, updated_at_ms
                         FROM docs WHERE owner_user_id = ?1 AND space_id = ?2 ORDER BY updated_at_ms DESC",
                    )
                    .map_err(|e| map_sqlite_err(e, "prepare list_docs by space"))?;
                let rows = stmt
                    .query_map(rusqlite::params![owner_id.as_str(), sp], row_to_doc)
                    .map_err(|e| map_sqlite_err(e, "query list_docs by space"))?;
                for r in rows {
                    out.push(r.map_err(|e| map_sqlite_err(e, "read doc"))?);
                }
            } else {
                let mut stmt = conn
                    .prepare(
                        "SELECT id, owner_user_id, space_id, title, filename, current_rev, source_format, created_at_ms, updated_at_ms
                         FROM docs WHERE owner_user_id = ?1 ORDER BY updated_at_ms DESC",
                    )
                    .map_err(|e| map_sqlite_err(e, "prepare list_docs"))?;
                let rows = stmt
                    .query_map([owner_id.as_str()], row_to_doc)
                    .map_err(|e| map_sqlite_err(e, "query list_docs"))?;
                for r in rows {
                    out.push(r.map_err(|e| map_sqlite_err(e, "read doc"))?);
                }
            }
            Ok(out)
        })
    }

    fn get_doc_detail(
        &self,
        doc_id: &str,
        owner_id: &UserId,
        rev: Option<i64>,
    ) -> Result<DocDetail, EngineError> {
        self.pool
            .with_reader(|conn| load_detail_with_conn(conn, doc_id, owner_id, rev))
    }

    fn get_revision_blob(
        &self,
        doc_id: &str,
        owner_id: &UserId,
        rev: i64,
    ) -> Result<(DocRecord, DocRevisionMeta, Vec<u8>), EngineError> {
        self.pool.with_reader(|conn| {
            let doc = conn
                .query_row(
                    "SELECT id, owner_user_id, space_id, title, filename, current_rev, source_format, created_at_ms, updated_at_ms
                     FROM docs WHERE id = ?1",
                    [doc_id],
                    row_to_doc,
                )
                .optional()
                .map_err(|e| map_sqlite_err(e, "query doc for blob"))?
                .ok_or_else(|| EngineError::not_found("文档不存在"))?;

            if &doc.owner_user_id != owner_id {
                return Err(EngineError::forbidden("无权下载该文档"));
            }

            let (meta, blob) = conn
                .query_row(
                    "SELECT doc_id, rev, note, author_name, export_mode, node_count, table_count, mandatory_count, byte_size, created_at_ms, docx_blob
                     FROM doc_revisions WHERE doc_id = ?1 AND rev = ?2",
                    rusqlite::params![doc_id, rev],
                    |row| {
                        let meta = row_to_rev(row)?;
                        let bytes: Vec<u8> = row.get(10)?;
                        Ok((meta, bytes))
                    },
                )
                .optional()
                .map_err(|e| map_sqlite_err(e, "query doc_revision blob"))?
                .ok_or_else(|| EngineError::not_found("指定版本不存在"))?;

            Ok((doc, meta, blob))
        })
    }

    fn get_revision_nodes(
        &self,
        doc_id: &str,
        owner_id: &UserId,
        rev: i64,
    ) -> Result<Vec<DocNodeRecord>, EngineError> {
        self.pool.with_reader(|conn| {
            let owner: String = conn
                .query_row("SELECT owner_user_id FROM docs WHERE id = ?1", [doc_id], |r| {
                    r.get(0)
                })
                .optional()
                .map_err(|e| map_sqlite_err(e, "check doc owner"))?
                .ok_or_else(|| EngineError::not_found("文档不存在"))?;

            if owner != owner_id.as_str() {
                return Err(EngineError::forbidden("无权访问该文档节点"));
            }

            let mut stmt = conn
                .prepare(
                    "SELECT ord, anchor_id, parent_anchor_id, explicit_anchor, xml_locator, node_kind, heading_path, text_content, text_hash, row_keys_json, mandatory_tags, locked, token_estimate, is_search_chunk
                     FROM doc_nodes WHERE doc_id = ?1 AND rev = ?2 ORDER BY ord ASC",
                )
                .map_err(|e| map_sqlite_err(e, "prepare get_revision_nodes"))?;

            let nodes = stmt
                .query_map(rusqlite::params![doc_id, rev], row_to_node)
                .map_err(|e| map_sqlite_err(e, "query get_revision_nodes"))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| map_sqlite_err(e, "read revision node"))?;

            Ok(nodes)
        })
    }

    fn set_node_locked(
        &self,
        doc_id: &str,
        owner_id: &UserId,
        anchor_id: &str,
        locked: bool,
        now_ms: i64,
    ) -> Result<DocNodeRecord, EngineError> {
        self.pool.with_writer(|conn| {
            let tx = conn
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|e| map_sqlite_err(e, "begin lock node"))?;

            let (owner_raw, cur_rev): (String, i64) = tx
                .query_row(
                    "SELECT owner_user_id, current_rev FROM docs WHERE id = ?1",
                    [doc_id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()
                .map_err(|e| map_sqlite_err(e, "query doc for lock"))?
                .ok_or_else(|| EngineError::not_found("文档不存在"))?;

            if owner_raw != owner_id.as_str() {
                return Err(EngineError::forbidden("无权操作该文档"));
            }

            let changed = tx
                .execute(
                    "UPDATE doc_nodes SET locked = ?4 WHERE doc_id = ?1 AND rev = ?2 AND anchor_id = ?3",
                    rusqlite::params![doc_id, cur_rev, anchor_id, if locked { 1i64 } else { 0i64 }],
                )
                .map_err(|e| map_sqlite_err(e, "update node locked"))?;

            if changed == 0 {
                return Err(EngineError::not_found("目标条款节点不存在"));
            }

            tx.execute(
                "UPDATE docs SET updated_at_ms = ?2 WHERE id = ?1",
                rusqlite::params![doc_id, now_ms],
            )
            .map_err(|e| map_sqlite_err(e, "touch doc updated_at"))?;

            let updated = tx
                .query_row(
                    "SELECT ord, anchor_id, parent_anchor_id, explicit_anchor, xml_locator, node_kind, heading_path, text_content, text_hash, row_keys_json, mandatory_tags, locked, token_estimate, is_search_chunk
                     FROM doc_nodes WHERE doc_id = ?1 AND rev = ?2 AND anchor_id = ?3",
                    rusqlite::params![doc_id, cur_rev, anchor_id],
                    row_to_node,
                )
                .map_err(|e| map_sqlite_err(e, "reload locked node"))?;

            tx.commit().map_err(|e| map_sqlite_err(e, "commit lock node"))?;
            Ok(updated)
        })
    }

    fn search_doc_fts(
        &self,
        doc_id: &str,
        owner_id: &UserId,
        query: &str,
        limit: usize,
    ) -> Result<Vec<DocSearchHit>, EngineError> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }
        let cap = limit.clamp(1, 40) as i64;

        self.pool.with_reader(|conn| {
            let (owner_raw, cur_rev): (String, i64) = conn
                .query_row(
                    "SELECT owner_user_id, current_rev FROM docs WHERE id = ?1",
                    [doc_id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()
                .map_err(|e| map_sqlite_err(e, "query doc for search"))?
                .ok_or_else(|| EngineError::not_found("文档不存在"))?;

            if owner_raw != owner_id.as_str() {
                return Err(EngineError::forbidden("无权检索该文档"));
            }

            let mut raw_hits: Vec<(String, String, String, String)> = Vec::new();
            let fts_expr = sanitize_fts5_query(trimmed);
            if !fts_expr.is_empty() {
                let mut stmt = conn
                    .prepare(
                        "SELECT anchor_id, parent_anchor_id, heading_path, text_content
                         FROM doc_fts
                         WHERE doc_fts MATCH ?1 AND doc_id = ?2 AND rev = ?3
                         ORDER BY bm25(doc_fts)
                         LIMIT ?4",
                    )
                    .map_err(|e| map_sqlite_err(e, "prepare doc_fts search"))?;
                let rows = stmt
                    .query_map(rusqlite::params![fts_expr, doc_id, cur_rev, cap], |r| {
                        Ok((
                            r.get::<_, String>(0)?,
                            r.get::<_, String>(1)?,
                            r.get::<_, String>(2)?,
                            r.get::<_, String>(3)?,
                        ))
                    })
                    .map_err(|e| map_sqlite_err(e, "query doc_fts"))?;
                for r in rows {
                    raw_hits.push(r.map_err(|e| map_sqlite_err(e, "read fts hit"))?);
                }
            }

            // CJK substring fallback when tokenizer misses unspaced substrings
            if raw_hits.is_empty() {
                let like = format!("%{}%", escape_like(trimmed));
                let mut stmt = conn
                    .prepare(
                        "SELECT anchor_id, COALESCE(parent_anchor_id, ''), heading_path, text_content
                         FROM doc_nodes
                         WHERE doc_id = ?1 AND rev = ?2 AND (text_content LIKE ?3 ESCAPE '\\' OR heading_path LIKE ?3 ESCAPE '\\')
                         ORDER BY ord ASC
                         LIMIT ?4",
                    )
                    .map_err(|e| map_sqlite_err(e, "prepare fallback search"))?;
                let rows = stmt
                    .query_map(rusqlite::params![doc_id, cur_rev, like, cap], |r| {
                        Ok((
                            r.get::<_, String>(0)?,
                            r.get::<_, String>(1)?,
                            r.get::<_, String>(2)?,
                            r.get::<_, String>(3)?,
                        ))
                    })
                    .map_err(|e| map_sqlite_err(e, "query fallback search"))?;
                for r in rows {
                    raw_hits.push(r.map_err(|e| map_sqlite_err(e, "read fallback hit"))?);
                }
            }

            if raw_hits.is_empty() {
                return Ok(Vec::new());
            }

            // Load nodes & refs for current revision to assemble Small-to-Big parent context
            let mut node_stmt = conn
                .prepare(
                    "SELECT ord, anchor_id, parent_anchor_id, explicit_anchor, xml_locator, node_kind, heading_path, text_content, text_hash, row_keys_json, mandatory_tags, locked, token_estimate, is_search_chunk
                     FROM doc_nodes WHERE doc_id = ?1 AND rev = ?2",
                )
                .map_err(|e| map_sqlite_err(e, "prepare nodes lookup"))?;
            let all_nodes: Vec<DocNodeRecord> = node_stmt
                .query_map(rusqlite::params![doc_id, cur_rev], row_to_node)
                .map_err(|e| map_sqlite_err(e, "query nodes lookup"))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| map_sqlite_err(e, "read nodes lookup"))?;

            let by_anchor: HashMap<&str, &DocNodeRecord> = all_nodes
                .iter()
                .map(|n| (n.anchor_id.as_str(), n))
                .collect();

            let mut ref_stmt = conn
                .prepare(
                    "SELECT source_anchor, target_hint FROM doc_refs WHERE doc_id = ?1 AND rev = ?2",
                )
                .map_err(|e| map_sqlite_err(e, "prepare refs lookup"))?;
            let all_refs: Vec<DocRefRecord> = ref_stmt
                .query_map(rusqlite::params![doc_id, cur_rev], |r| {
                    Ok(DocRefRecord {
                        source_anchor: r.get(0)?,
                        target_hint: r.get(1)?,
                    })
                })
                .map_err(|e| map_sqlite_err(e, "query refs lookup"))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| map_sqlite_err(e, "read refs lookup"))?;

            let mut out = Vec::new();
            let mut seen_parents = HashSet::new();

            for (anchor, parent_anchor, heading_path, hit_text) in raw_hits {
                let resolved_parent = if !parent_anchor.is_empty() {
                    by_anchor.get(parent_anchor.as_str()).copied().cloned()
                } else {
                    by_anchor.get(anchor.as_str()).copied().cloned()
                };
                let dedup_key = resolved_parent
                    .as_ref()
                    .map(|p| p.anchor_id.clone())
                    .unwrap_or_else(|| anchor.clone());
                if !seen_parents.insert(dedup_key.clone()) {
                    continue;
                }

                let related_refs: Vec<DocRefRecord> = all_refs
                    .iter()
                    .filter(|r| r.source_anchor == anchor || r.source_anchor == dedup_key)
                    .cloned()
                    .collect();

                out.push(DocSearchHit {
                    hit_anchor_id: anchor,
                    hit_text,
                    heading_path,
                    parent_node: resolved_parent,
                    related_refs,
                });
            }

            Ok(out)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};
    use std::sync::Arc;

    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

    use super::SqliteDocStore;
    use crate::core::ids::UserId;
    use crate::core::traits::DocStore;
    use crate::db::migrate::migrate;
    use crate::db::pool::SqlitePool;
    use crate::doc::model::{DocRecord, DocRevisionMeta, ExportMode};
    use crate::doc::ooxml::parse_docx_ast;

    fn sample_docx() -> Vec<u8> {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut zw = ZipWriter::new(&mut cursor);
            let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
            zw.start_file("word/document.xml", opts).unwrap();
            zw.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>第三章 运维服务方案</w:t></w:r></w:p>
    <w:p><w:r><w:t>★ 一般故障 30 分钟内响应、2 小时内恢复，详见表 2-3。</w:t></w:r></w:p>
  </w:body>
</w:document>"#,
            )
            .unwrap();
            zw.finish().unwrap();
        }
        cursor.into_inner()
    }

    #[test]
    fn sqlite_doc_store_lifecycle_fts_search_and_locks() {
        let pool = Arc::new(SqlitePool::open_in_memory().expect("open mem"));
        let now = 1_763_600_000_000_i64;
        migrate(&pool, now).expect("migrate v4");

        let store = SqliteDocStore::new(Arc::clone(&pool));
        let owner = UserId::new("u_test1");
        let bytes = sample_docx();
        let parsed = parse_docx_ast(&bytes).expect("parse");

        let doc = DocRecord {
            id: String::from("doc_01"),
            owner_user_id: owner.clone(),
            space_id: String::from("presales"),
            title: String::from("技术标_甲方案"),
            filename: String::from("技术标_甲方案.docx"),
            current_rev: 1,
            source_format: String::from("docx"),
            created_at_ms: now,
            updated_at_ms: now,
        };
        let rev = DocRevisionMeta {
            doc_id: doc.id.clone(),
            rev: 1,
            note: String::from("初稿入库"),
            author_name: String::from("chen.mo"),
            export_mode: String::from("raw"),
            node_count: parsed.node_count,
            table_count: parsed.table_count,
            mandatory_count: parsed.mandatory_count,
            byte_size: bytes.len(),
            created_at_ms: now,
        };

        let detail = store
            .create_doc(&doc, &rev, &bytes, &parsed)
            .expect("create doc");
        assert_eq!(detail.nodes.len(), 2);

        let target_anchor = detail.nodes[1].anchor_id.clone();
        let locked = store
            .set_node_locked(&doc.id, &owner, &target_anchor, true, now + 100)
            .expect("lock node");
        assert!(locked.locked);

        let hits = store
            .search_doc_fts(&doc.id, &owner, "30 分钟", 10)
            .expect("search");
        assert!(!hits.is_empty());
        assert_eq!(hits[0].related_refs.len(), 1);
        assert_eq!(hits[0].related_refs[0].target_hint, "表 2-3");

        let next_detail = store
            .append_revision(
                &doc.id,
                &owner,
                "导出修订",
                "chen.mo",
                ExportMode::Clean,
                &bytes,
                &parsed,
                Some(1),
                now + 200,
            )
            .expect("append revision");
        assert_eq!(next_detail.doc.current_rev, 2);
        assert!(next_detail.nodes[1].locked, "lock carried across revisions");
    }
}
