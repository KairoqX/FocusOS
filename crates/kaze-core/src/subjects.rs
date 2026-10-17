use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::error::{AppError, AppResult};
use crate::models::{Subject, SubjectInput};
use crate::progress::percent_complete;
use crate::validation::validate_subject_name;

// Aggregates come straight from the chapters/tasks tables; nothing is cached or stored.
const SELECT: &str = "SELECT s.id, s.name, \
    COUNT(c.id) AS chapter_count, \
    COALESCE(SUM(c.status = 'not_started'), 0) AS not_started, \
    COALESCE(SUM(c.status = 'in_progress'), 0) AS in_progress, \
    COALESCE(SUM(c.status = 'completed'), 0) AS completed, \
    COALESCE(SUM(c.status = 'needs_revision'), 0) AS needs_revision, \
    (SELECT COUNT(*) FROM tasks t WHERE t.subject_id = s.id) AS task_count, \
    (SELECT COUNT(*) FROM tasks t WHERE t.subject_id = s.id AND t.completed_at IS NULL) AS open_tasks \
    FROM subjects s LEFT JOIN chapters c ON c.subject_id = s.id";

fn map_row(row: &Row<'_>) -> rusqlite::Result<Subject> {
    let chapter_count: u32 = row.get("chapter_count")?;
    let completed: u32 = row.get("completed")?;
    Ok(Subject {
        id: row.get("id")?,
        name: row.get("name")?,
        chapter_count,
        not_started: row.get("not_started")?,
        in_progress: row.get("in_progress")?,
        completed,
        needs_revision: row.get("needs_revision")?,
        percent_complete: percent_complete(completed, chapter_count),
        task_count: row.get("task_count")?,
        open_tasks: row.get("open_tasks")?,
    })
}

pub fn list(conn: &Connection) -> AppResult<Vec<Subject>> {
    let sql = format!("{SELECT} GROUP BY s.id ORDER BY s.name COLLATE NOCASE, s.id");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], map_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<Subject>>>()?)
}

pub fn get(conn: &Connection, id: i64) -> AppResult<Subject> {
    let sql = format!("{SELECT} WHERE s.id = ?1 GROUP BY s.id");
    conn.query_row(&sql, params![id], map_row)
        .optional()?
        .ok_or_else(|| AppError::not_found("subject", id))
}

pub fn create(conn: &Connection, input: &SubjectInput) -> AppResult<Subject> {
    let name = validate_subject_name(&input.name)?;
    ensure_name_free(conn, &name, None)?;
    conn.execute("INSERT INTO subjects (name) VALUES (?1)", params![name])?;
    get(conn, conn.last_insert_rowid())
}

pub fn update(conn: &Connection, id: i64, input: &SubjectInput) -> AppResult<Subject> {
    let name = validate_subject_name(&input.name)?;
    ensure_exists(conn, id)?;
    ensure_name_free(conn, &name, Some(id))?;
    conn.execute(
        "UPDATE subjects SET name = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') \
         WHERE id = ?2",
        params![name, id],
    )?;
    get(conn, id)
}

/// Deletes the subject and (via ON DELETE CASCADE) its chapters.
/// Linked tasks are kept; their subject becomes empty (ON DELETE SET NULL).
pub fn delete(conn: &Connection, id: i64) -> AppResult<()> {
    let changed = conn.execute("DELETE FROM subjects WHERE id = ?1", params![id])?;
    if changed == 0 {
        return Err(AppError::not_found("subject", id));
    }
    Ok(())
}

pub fn ensure_exists(conn: &Connection, id: i64) -> AppResult<()> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM subjects WHERE id = ?1)",
        params![id],
        |r| r.get(0),
    )?;
    if exists {
        Ok(())
    } else {
        Err(AppError::not_found("subject", id))
    }
}

/// Names are unique case-insensitively (the column uses COLLATE NOCASE).
fn ensure_name_free(conn: &Connection, name: &str, except: Option<i64>) -> AppResult<()> {
    let taken: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM subjects WHERE name = ?1 AND id <> COALESCE(?2, -1))",
        params![name, except],
        |r| r.get(0),
    )?;
    if taken {
        return Err(AppError::validation(
            "name",
            "A subject with this name already exists.",
        ));
    }
    Ok(())
}
