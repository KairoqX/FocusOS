-- 0003: link tasks to chapters; per-chapter progress mode.
-- Additive and backward compatible: existing rows keep their data.
--   * tasks.chapter_id is NULL for every existing task ("subject only" tasks keep working).
--   * every existing chapter becomes 'automatic' (progress derived from linked tasks).
-- Deletion rules:
--   * deleting a chapter keeps its tasks (chapter_id -> NULL, subject_id untouched);
--   * deleting a subject keeps its tasks (subject_id and chapter_id -> NULL), chapters cascade.

ALTER TABLE tasks ADD COLUMN chapter_id INTEGER REFERENCES chapters(id) ON DELETE SET NULL;
CREATE INDEX idx_tasks_chapter ON tasks(chapter_id);

ALTER TABLE chapters ADD COLUMN progress_mode TEXT NOT NULL DEFAULT 'automatic'
    CHECK (progress_mode IN ('automatic', 'manual'));
ALTER TABLE chapters ADD COLUMN manual_progress INTEGER NOT NULL DEFAULT 0
    CHECK (manual_progress BETWEEN 0 AND 100);

-- A task's chapter must belong to the task's subject (a composite FK cannot be used: its
-- ON DELETE SET NULL would also null subject_id when a chapter is deleted).
CREATE TRIGGER trg_tasks_chapter_matches_subject_insert
BEFORE INSERT ON tasks
WHEN NEW.chapter_id IS NOT NULL
BEGIN
    SELECT RAISE(ABORT, 'task chapter must belong to the task subject')
    WHERE NOT EXISTS (
        SELECT 1 FROM chapters c WHERE c.id = NEW.chapter_id AND c.subject_id IS NEW.subject_id
    );
END;

CREATE TRIGGER trg_tasks_chapter_matches_subject_update
BEFORE UPDATE OF subject_id, chapter_id ON tasks
WHEN NEW.chapter_id IS NOT NULL
BEGIN
    SELECT RAISE(ABORT, 'task chapter must belong to the task subject')
    WHERE NOT EXISTS (
        SELECT 1 FROM chapters c WHERE c.id = NEW.chapter_id AND c.subject_id IS NEW.subject_id
    );
END;

-- Runs before the subject's FK actions, so SET NULL on tasks.subject_id never sees a stale chapter.
CREATE TRIGGER trg_subjects_release_task_chapters
BEFORE DELETE ON subjects
BEGIN
    UPDATE tasks SET chapter_id = NULL WHERE subject_id = OLD.id AND chapter_id IS NOT NULL;
END;

CREATE TRIGGER trg_chapters_subject_immutable
BEFORE UPDATE OF subject_id ON chapters
WHEN NEW.subject_id IS NOT OLD.subject_id
BEGIN
    SELECT RAISE(ABORT, 'a chapter cannot move to another subject');
END;
