-- Saved Windows desktop icon layouts (physical pixel positions read through
-- the shell). The snapshot is what makes "restore layout" possible.

CREATE TABLE layout_snapshots (
    id         INTEGER PRIMARY KEY,
    created_at INTEGER NOT NULL,
    reason     TEXT,                    -- 'manual' | 'before_apply_layout' | ...
    payload    TEXT NOT NULL            -- JSON: [{path, x, y, monitor}], physical px
);

CREATE INDEX idx_layout_snapshots_created ON layout_snapshots (created_at DESC);
