-- 0004: focus/break sessions and key-value app settings.
-- Times are UTC epoch milliseconds. Elapsed time is always derived:
--   accumulated_ms + (now - run_started_at_ms while running)
-- so the stored row, not an in-memory tick counter, is the source of truth.
-- Deletion rule: deleting a task or subject keeps the session (task_id / subject_id -> NULL).

CREATE TABLE app_settings (
    key        TEXT PRIMARY KEY CHECK (length(key) BETWEEN 1 AND 100),
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE TABLE focus_sessions (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    kind              TEXT NOT NULL CHECK (kind IN ('focus', 'break')),
    state             TEXT NOT NULL CHECK (state IN ('running', 'paused', 'completed', 'stopped')),
    planned_seconds   INTEGER NOT NULL CHECK (planned_seconds BETWEEN 60 AND 14400),
    task_id           INTEGER REFERENCES tasks(id) ON DELETE SET NULL,
    subject_id        INTEGER REFERENCES subjects(id) ON DELETE SET NULL,
    started_at_ms     INTEGER NOT NULL,
    run_started_at_ms INTEGER,                       -- set only while running
    accumulated_ms    INTEGER NOT NULL DEFAULT 0 CHECK (accumulated_ms >= 0),
    last_seen_ms      INTEGER NOT NULL,              -- last heartbeat while running
    ended_at_ms       INTEGER,                       -- set only when completed/stopped
    auto_paused       INTEGER NOT NULL DEFAULT 0 CHECK (auto_paused IN (0, 1)),
    created_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    CHECK ((state = 'running') = (run_started_at_ms IS NOT NULL)),
    CHECK ((state IN ('completed', 'stopped')) = (ended_at_ms IS NOT NULL))
);

-- At most one active (running or paused) session, enforced by the database itself.
CREATE UNIQUE INDEX idx_focus_single_active ON focus_sessions ((1))
    WHERE state IN ('running', 'paused');
CREATE INDEX idx_focus_history ON focus_sessions (started_at_ms)
    WHERE state IN ('completed', 'stopped');
CREATE INDEX idx_focus_task ON focus_sessions (task_id);
