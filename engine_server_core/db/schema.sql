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
    created_at_ms INTEGER NOT NULL,
    last_active_ms INTEGER NOT NULL DEFAULT 0
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_users_name_grp ON users (name, grp);
CREATE INDEX IF NOT EXISTS idx_users_state_role ON users (state, role);
CREATE INDEX IF NOT EXISTS idx_users_created ON users (created_at_ms DESC);

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
