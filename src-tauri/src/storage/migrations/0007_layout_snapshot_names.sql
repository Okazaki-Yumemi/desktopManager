-- User-facing labels for saved desktop icon layouts (M3). The table itself
-- predates the feature (0002); `name` gives each snapshot a stable, listable
-- label instead of overloading `reason`.

ALTER TABLE layout_snapshots ADD COLUMN name TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_layout_snapshots_name
    ON layout_snapshots (name) WHERE name IS NOT NULL;
