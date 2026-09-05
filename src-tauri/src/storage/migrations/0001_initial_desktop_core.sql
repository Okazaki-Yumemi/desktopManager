-- Desktop core entities (collections, desktop item index, assignments).
-- Classification is metadata only: we never move the user's real files.

CREATE TABLE settings (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL,           -- JSON-encoded value
    updated_at  INTEGER NOT NULL
);

CREATE TABLE collections (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    color       TEXT NOT NULL DEFAULT '#4f8cff',
    opacity     REAL NOT NULL DEFAULT 0.9,
    icon        TEXT,
    -- Layout uses logical pixels relative to the monitor, so it survives
    -- DPI changes; monitor identity is stored for multi-monitor support.
    monitor_id  TEXT,
    pos_x       REAL NOT NULL DEFAULT 0,
    pos_y       REAL NOT NULL DEFAULT 0,
    width       REAL,
    height      REAL,
    collapsed   INTEGER NOT NULL DEFAULT 0,
    hidden      INTEGER NOT NULL DEFAULT 0,
    sort_order  INTEGER NOT NULL DEFAULT 0,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);

CREATE TABLE desktop_items (
    id            INTEGER PRIMARY KEY,
    path          TEXT NOT NULL UNIQUE,  -- absolute path, natural key
    source        TEXT NOT NULL,         -- 'user_desktop' | 'public_desktop'
    display_name  TEXT NOT NULL,
    kind          TEXT NOT NULL CHECK (kind IN ('file', 'folder', 'shortcut')),
    ext           TEXT,
    size_bytes    INTEGER,
    modified_at   INTEGER,
    first_seen_at INTEGER NOT NULL,
    last_seen_at  INTEGER NOT NULL,
    missing       INTEGER NOT NULL DEFAULT 0 -- indexed before but currently absent on disk
);

CREATE TABLE collection_items (
    id            INTEGER PRIMARY KEY,
    collection_id INTEGER NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    item_path     TEXT NOT NULL,
    sort_order    INTEGER NOT NULL DEFAULT 0,
    added_at      INTEGER NOT NULL,
    UNIQUE (collection_id, item_path)
);

CREATE INDEX idx_desktop_items_last_seen ON desktop_items (last_seen_at);
CREATE INDEX idx_collection_items_collection ON collection_items (collection_id, sort_order);
