-- SJTU-synced calendar entries (M12). A local projection of the university
-- calendar service: each successful sync replaces every row in one
-- transaction (see storage/sjtu_repo.rs), so the table never holds stale
-- duplicates. User-created calendar events live in calendar_events and are
-- never touched by the sync.

CREATE TABLE sjtu_events (
    id          INTEGER PRIMARY KEY,
    external_id TEXT NOT NULL UNIQUE,
    title       TEXT NOT NULL,
    location    TEXT,
    starts_at   INTEGER NOT NULL,
    ends_at     INTEGER NOT NULL,
    all_day     INTEGER NOT NULL DEFAULT 0,
    status      TEXT,
    recurrence  INTEGER,
    source      TEXT NOT NULL DEFAULT 'personal', -- 'personal' | 'school'
    calendar_id TEXT,
    synced_at   INTEGER NOT NULL
);

CREATE INDEX idx_sjtu_events_starts ON sjtu_events (starts_at);
