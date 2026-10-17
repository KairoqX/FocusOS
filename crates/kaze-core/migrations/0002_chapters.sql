-- 0002: chapters.
-- Deletion rule: deleting a subject deletes its chapters (CASCADE); its tasks are kept
-- (tasks.subject_id was defined ON DELETE SET NULL in 0001).

CREATE TABLE chapters (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    subject_id INTEGER NOT NULL REFERENCES subjects(id) ON DELETE CASCADE,
    title      TEXT NOT NULL COLLATE NOCASE
                    CHECK (length(trim(title)) BETWEEN 1 AND 200),
    status     TEXT NOT NULL DEFAULT 'not_started'
                    CHECK (status IN ('not_started', 'in_progress', 'completed', 'needs_revision')),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    UNIQUE (subject_id, title)  -- also serves as the subject_id lookup index
);
