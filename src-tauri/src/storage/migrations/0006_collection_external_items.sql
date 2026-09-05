-- Snapshot metadata for collection items that are NOT on the desktop index
-- (e.g. shortcuts dragged in from the Start Menu or anywhere on disk).
-- Desktop-indexed rows leave these NULL and keep using live metadata from
-- desktop_items via LEFT JOIN.
ALTER TABLE collection_items ADD COLUMN label TEXT;
ALTER TABLE collection_items ADD COLUMN kind TEXT;
ALTER TABLE collection_items ADD COLUMN ext TEXT;
ALTER TABLE collection_items ADD COLUMN size_bytes INTEGER;
ALTER TABLE collection_items ADD COLUMN modified_at INTEGER;
