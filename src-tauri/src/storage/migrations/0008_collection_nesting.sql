-- Sub-collections: a collection may reference a parent collection.
-- Assignments and scene visibility stay per-collection; deleting a parent
-- deletes the whole subtree (enforced in the repo via a recursive query so
-- it holds regardless of the foreign_keys pragma).
ALTER TABLE collections
    ADD COLUMN parent_id INTEGER REFERENCES collections(id);

CREATE INDEX idx_collections_parent ON collections(parent_id);
