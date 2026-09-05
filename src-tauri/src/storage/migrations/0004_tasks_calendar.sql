-- Lightweight task list and local calendar.

CREATE TABLE tasks (
    id                INTEGER PRIMARY KEY,
    title             TEXT NOT NULL,
    notes             TEXT,
    status            TEXT NOT NULL DEFAULT 'todo' CHECK (status IN ('todo', 'doing', 'done')),
    priority          INTEGER NOT NULL DEFAULT 0,
    due_at            INTEGER,
    estimated_minutes INTEGER,
    tags              TEXT,              -- JSON array of strings
    created_at        INTEGER NOT NULL,
    completed_at      INTEGER,
    updated_at        INTEGER NOT NULL
);

CREATE TABLE calendar_events (
    id         INTEGER PRIMARY KEY,
    title      TEXT NOT NULL,
    starts_at  INTEGER NOT NULL,
    ends_at    INTEGER NOT NULL,
    all_day    INTEGER NOT NULL DEFAULT 0,
    notes      TEXT,
    color      TEXT,
    task_id    INTEGER REFERENCES tasks(id) ON DELETE SET NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX idx_tasks_status ON tasks (status);
CREATE INDEX idx_calendar_events_starts ON calendar_events (starts_at);
