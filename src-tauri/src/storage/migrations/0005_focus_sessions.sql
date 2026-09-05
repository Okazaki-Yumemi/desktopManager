-- Focus (pomodoro / custom timer) sessions. A session may be bound to a task
-- or a scene; previous-scene state for auto-restore is stored via settings.

CREATE TABLE focus_sessions (
    id                INTEGER PRIMARY KEY,
    task_id           INTEGER REFERENCES tasks(id) ON DELETE SET NULL,
    scene_id          INTEGER REFERENCES scenes(id) ON DELETE SET NULL,
    kind              TEXT NOT NULL DEFAULT 'pomodoro' CHECK (kind IN ('pomodoro', 'custom', 'count_up')),
    planned_duration_s INTEGER NOT NULL,
    actual_duration_s  INTEGER NOT NULL DEFAULT 0,
    status            TEXT NOT NULL DEFAULT 'running'
                      CHECK (status IN ('running', 'completed', 'interrupted', 'abandoned')),
    started_at        INTEGER NOT NULL,
    ended_at          INTEGER,
    interruptions     INTEGER NOT NULL DEFAULT 0,
    note              TEXT
);

CREATE INDEX idx_focus_sessions_started ON focus_sessions (started_at);
