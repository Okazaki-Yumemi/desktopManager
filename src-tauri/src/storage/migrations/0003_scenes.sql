-- Scenes: named arrangements of collections (e.g. "Study", "Research").
-- Only visibility/layout metadata lives here; real files are never touched.

CREATE TABLE scenes (
    id         INTEGER PRIMARY KEY,
    name       TEXT NOT NULL,
    color      TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE scene_layouts (
    id            INTEGER PRIMARY KEY,
    scene_id      INTEGER NOT NULL REFERENCES scenes(id) ON DELETE CASCADE,
    collection_id INTEGER NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    visible       INTEGER NOT NULL DEFAULT 1,
    pos_x         REAL,
    pos_y         REAL,
    width         REAL,
    height        REAL,
    collapsed     INTEGER,
    UNIQUE (scene_id, collection_id)
);
