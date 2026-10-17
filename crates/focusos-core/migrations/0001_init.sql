-- 0001: subjects and tasks.
-- Deletion rule: deleting a subject keeps its tasks (tasks.subject_id becomes NULL).

CREATE TABLE subjects (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT NOT NULL COLLATE NOCASE UNIQUE
                    CHECK (length(trim(name)) BETWEEN 1 AND 100),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE TABLE tasks (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    title             TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 200),
    description       TEXT NOT NULL DEFAULT '' CHECK (length(description) <= 5000),
    subject_id        INTEGER REFERENCES subjects(id) ON DELETE SET NULL,
    priority          TEXT NOT NULL DEFAULT 'medium'
                           CHECK (priority IN ('low', 'medium', 'high')),
    due_date          TEXT CHECK (due_date IS NULL OR
                                  due_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
    estimated_minutes INTEGER CHECK (estimated_minutes IS NULL OR
                                     estimated_minutes BETWEEN 1 AND 1440),
    completed_at      TEXT,
    created_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE INDEX idx_tasks_subject      ON tasks(subject_id);
CREATE INDEX idx_tasks_open_due     ON tasks(due_date) WHERE completed_at IS NULL;
CREATE INDEX idx_tasks_completed_at ON tasks(completed_at) WHERE completed_at IS NOT NULL;
