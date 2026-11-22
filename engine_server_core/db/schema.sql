CREATE TABLE IF NOT EXISTS schema_ver (
    version INTEGER PRIMARY KEY,
    applied_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    grp TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('admin', 'member')),
    state TEXT NOT NULL CHECK (state IN ('active', 'disabled')),
    daily_requests INTEGER NOT NULL DEFAULT 0,
    monthly_tokens INTEGER NOT NULL DEFAULT 0,
    used_requests INTEGER NOT NULL DEFAULT 0,
    used_tokens INTEGER NOT NULL DEFAULT 0,
    token_hash TEXT NOT NULL UNIQUE,
    token_hint TEXT NOT NULL DEFAULT '',
    password_hash TEXT NOT NULL DEFAULT '',
    created_at_ms INTEGER NOT NULL,
    last_active_ms INTEGER NOT NULL DEFAULT 0
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_users_name_grp ON users (name, grp);
CREATE INDEX IF NOT EXISTS idx_users_name ON users (name);
CREATE INDEX IF NOT EXISTS idx_users_state_role ON users (state, role);
CREATE INDEX IF NOT EXISTS idx_users_created ON users (created_at_ms DESC);

CREATE TABLE IF NOT EXISTS sessions (
    token_hash TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    expires_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions (user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_exp ON sessions (expires_at_ms);

CREATE TABLE IF NOT EXISTS msgs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    conv_id TEXT NOT NULL,
    seq INTEGER NOT NULL,
    payload BLOB NOT NULL,
    created_at_ms INTEGER NOT NULL,
    UNIQUE (user_id, conv_id, seq)
);

CREATE INDEX IF NOT EXISTS idx_msgs_conv_time ON msgs (user_id, conv_id, created_at_ms DESC);

CREATE TABLE IF NOT EXISTS ledger (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    conv_id TEXT NOT NULL,
    prompt_tokens INTEGER NOT NULL,
    cached_tokens INTEGER NOT NULL,
    completion_tokens INTEGER NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_ledger_user_time ON ledger (user_id, created_at_ms);
CREATE INDEX IF NOT EXISTS idx_ledger_time ON ledger (created_at_ms);

CREATE TABLE IF NOT EXISTS docs (
    id TEXT PRIMARY KEY,
    owner_user_id TEXT NOT NULL,
    space_id TEXT NOT NULL,
    title TEXT NOT NULL,
    filename TEXT NOT NULL,
    current_rev INTEGER NOT NULL,
    source_format TEXT NOT NULL DEFAULT 'docx',
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_docs_owner_space ON docs (owner_user_id, space_id, updated_at_ms DESC);

CREATE TABLE IF NOT EXISTS doc_revisions (
    doc_id TEXT NOT NULL,
    rev INTEGER NOT NULL,
    note TEXT NOT NULL,
    author_name TEXT NOT NULL,
    export_mode TEXT NOT NULL,
    docx_blob BLOB NOT NULL,
    node_count INTEGER NOT NULL,
    table_count INTEGER NOT NULL,
    mandatory_count INTEGER NOT NULL,
    byte_size INTEGER NOT NULL,
    created_at_ms INTEGER NOT NULL,
    PRIMARY KEY (doc_id, rev)
);

CREATE TABLE IF NOT EXISTS doc_nodes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    doc_id TEXT NOT NULL,
    rev INTEGER NOT NULL,
    ord INTEGER NOT NULL,
    anchor_id TEXT NOT NULL,
    parent_anchor_id TEXT,
    explicit_anchor TEXT,
    xml_locator TEXT NOT NULL,
    node_kind TEXT NOT NULL,
    heading_path TEXT NOT NULL,
    text_content TEXT NOT NULL,
    text_hash TEXT NOT NULL,
    row_keys_json TEXT NOT NULL DEFAULT '[]',
    mandatory_tags TEXT NOT NULL DEFAULT '[]',
    locked INTEGER NOT NULL DEFAULT 0,
    token_estimate INTEGER NOT NULL DEFAULT 0,
    is_search_chunk INTEGER NOT NULL DEFAULT 0,
    UNIQUE (doc_id, rev, anchor_id)
);

CREATE INDEX IF NOT EXISTS idx_doc_nodes_rev ON doc_nodes (doc_id, rev, ord);

CREATE TABLE IF NOT EXISTS doc_refs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    doc_id TEXT NOT NULL,
    rev INTEGER NOT NULL,
    source_anchor TEXT NOT NULL,
    target_hint TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_doc_refs_rev ON doc_refs (doc_id, rev, source_anchor);

CREATE VIRTUAL TABLE IF NOT EXISTS doc_fts USING fts5(
    doc_id UNINDEXED,
    rev UNINDEXED,
    anchor_id UNINDEXED,
    parent_anchor_id UNINDEXED,
    heading_path,
    text_content
);
