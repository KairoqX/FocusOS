use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::error::{AppError, AppResult};
use crate::models::{Chapter, ChapterInput, ChapterProgressInput, ChapterStatus, ProgressMode};
use crate::progress::chapter_progress;
use crate::subjects;
use crate::validation::validate_chapter_title;

// Task counts are derived from the tasks table on every read. A task has a single chapter_id,
// so it is counted at most once per chapter. Progress itself is computed in Rust.
const SELECT: &str = "SELECT c.id, c.subject_id, c.title, c.status, c.progress_mode, \
    c.manual_progress, \
    (SELECT COUNT(*) FROM tasks t WHERE t.chapter_id = c.id) AS task_total, \
    (SELECT COUNT(*) FROM tasks t WHERE t.chapter_id = c.id AND t.completed_at IS NOT NULL) \
        AS task_completed, \
    c.created_at, c.updated_at FROM chapters c";

fn map_row(row: &Row<'_>) -> rusqlite::Result<Chapter> {
    let progress_mode: ProgressMode = row.get("progress_mode")?;
    let manual_progress: u8 = row.get("manual_progress")?;
    let task_total: u32 = row.get("task_total")?;
    let task_completed: u32 = row.get("task_completed")?;
    Ok(Chapter {
        id: row.get("id")?,
        subject_id: row.get("subject_id")?,
        title: row.get("title")?,
        status: row.get("status")?,
        progress_mode,
        manual_progress,
        task_total,
        task_completed,
        progress_percent: chapter_progress(
            progress_mode,
            manual_progress,
            task_completed,
            task_total,
        ),
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

/// Chapters of a subject in creation order.
pub fn list(conn: &Connection, subject_id: i64) -> AppResult<Vec<Chapter>> {
    subjects::ensure_exists(conn, subject_id)?;
    query(conn, Some(subject_id))
}

/// Chapters of one subject (`Some`) or of every subject (`None`), in creation order.
pub(crate) fn query(conn: &Connection, subject_id: Option<i64>) -> AppResult<Vec<Chapter>> {
    match subject_id {
        Some(id) => {
            let sql = format!("{SELECT} WHERE c.subject_id = ?1 ORDER BY c.id");
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params![id], map_row)?;
            Ok(rows.collect::<rusqlite::Result<Vec<Chapter>>>()?)
        }
        None => {
            let sql = format!("{SELECT} ORDER BY c.id");
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map([], map_row)?;
            Ok(rows.collect::<rusqlite::Result<Vec<Chapter>>>()?)
        }
    }
}

pub fn get(conn: &Connection, id: i64) -> AppResult<Chapter> {
    let sql = format!("{SELECT} WHERE c.id = ?1");
    conn.query_row(&sql, params![id], map_row)
        .optional()?
        .ok_or_else(|| AppError::not_found("chapter", id))
}

/// New chapters start in Automatic mode (the column default).
pub fn create(conn: &Connection, subject_id: i64, input: &ChapterInput) -> AppResult<Chapter> {
    let title = validate_chapter_title(&input.title)?;
    subjects::ensure_exists(conn, subject_id)?;
    ensure_title_free(conn, subject_id, &title, None)?;
    conn.execute(
        "INSERT INTO chapters (subject_id, title) VALUES (?1, ?2)",
        params![subject_id, title],
    )?;
    get(conn, conn.last_insert_rowid())
}

pub fn update(conn: &Connection, id: i64, input: &ChapterInput) -> AppResult<Chapter> {
    let title = validate_chapter_title(&input.title)?;
    let existing = get(conn, id)?;
    ensure_title_free(conn, existing.subject_id, &title, Some(id))?;
    conn.execute(
        "UPDATE chapters SET title = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') \
         WHERE id = ?2",
        params![title, id],
    )?;
    get(conn, id)
}

pub fn set_status(conn: &Connection, id: i64, status: ChapterStatus) -> AppResult<Chapter> {
    let changed = conn.execute(
        "UPDATE chapters SET status = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') \
         WHERE id = ?2",
        params![status.as_str(), id],
    )?;
    if changed == 0 {
        return Err(AppError::not_found("chapter", id));
    }
    get(conn, id)
}

/// Sets the tracking mode and, optionally, the manual percentage (0-100).
/// Switching to Automatic keeps the stored manual value but stops using it; task completion
/// never writes to `manual_progress`, so a manual value cannot be overwritten by tasks.
pub fn set_progress(
    conn: &Connection,
    id: i64,
    input: &ChapterProgressInput,
) -> AppResult<Chapter> {
    let existing = get(conn, id)?;
    let manual = match input.manual_progress {
        Some(v) if !(0..=100).contains(&v) => {
            return Err(AppError::validation(
                "manualProgress",
                "Progress must be between 0 and 100.",
            ))
        }
        Some(v) => v,
        None => i64::from(existing.manual_progress),
    };
    conn.execute(
        "UPDATE chapters SET progress_mode = ?1, manual_progress = ?2, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?3",
        params![input.mode.as_str(), manual, id],
    )?;
    get(conn, id)
}

/// Deletes the chapter. Linked tasks are kept and become chapter-less (ON DELETE SET NULL).
pub fn delete(conn: &Connection, id: i64) -> AppResult<()> {
    let changed = conn.execute("DELETE FROM chapters WHERE id = ?1", params![id])?;
    if changed == 0 {
        return Err(AppError::not_found("chapter", id));
    }
    Ok(())
}

/// Titles are unique per subject, case-insensitively (the column uses COLLATE NOCASE).
fn ensure_title_free(
    conn: &Connection,
    subject_id: i64,
    title: &str,
    except: Option<i64>,
) -> AppResult<()> {
    let taken: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM chapters \
         WHERE subject_id = ?1 AND title = ?2 AND id <> COALESCE(?3, -1))",
        params![subject_id, title, except],
        |r| r.get(0),
    )?;
    if taken {
        return Err(AppError::validation(
            "title",
            "This subject already has a chapter with that title.",
        ));
    }
    Ok(())
}
