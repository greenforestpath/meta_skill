-- 004_add_acip_quarantine.sql
-- Quarantine records for ACIP prompt-injection defense

CREATE TABLE injection_quarantine (
    quarantine_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    message_index INTEGER NOT NULL,
    content_hash TEXT NOT NULL,
    safe_excerpt TEXT NOT NULL,
    classification_json TEXT NOT NULL,
    audit_tag TEXT,
    created_at TEXT NOT NULL,
    replay_command TEXT NOT NULL
);

CREATE INDEX idx_injection_quarantine_session ON injection_quarantine(session_id);
