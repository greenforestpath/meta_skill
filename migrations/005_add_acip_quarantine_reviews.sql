-- 005_add_acip_quarantine_reviews.sql
-- Review actions for ACIP quarantine records

CREATE TABLE injection_quarantine_reviews (
    id TEXT PRIMARY KEY,
    quarantine_id TEXT NOT NULL,
    action TEXT NOT NULL, -- confirm_injection | false_positive
    reason TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_quarantine_reviews_quarantine ON injection_quarantine_reviews(quarantine_id);
