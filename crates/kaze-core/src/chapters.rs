use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::error::{AppError, AppResult};
use crate::models::{Chapter, ChapterInput, ChapterStatus};
use crate::subjects;
use crate::validation::validate_chapter_title;

const SELECT: &str = "SELECT id, subject_id, title, status, created_at, updated_at FROM chapters";

fn map_row(row: &Row<'_>) -> rusqlite::Result<Chapter> {
    Ok(Chapter {
        id: row.get("id")?,
        subject_id: row.get("subject_id")?,
        title: row.get("title")?,
        status: row.get("status")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

/// Chapters of a subject in creation order.
pub fn list(conn: &Connection, subject_id: i64) -> AppResult<Vec<Chapter>> {
    subjects::ensure_exists(conn, subject_id)?;
    let sql = format!("{SELECT} WHERE subject_id = ?1 ORDER BY id");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![subject_id], map_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<Chapter>>>()?)
}

pub fn get(conn: &Connection, id: i64) -> AppResult<Chapter> {
    let sql = format!("{SELECT} WHERE id = ?1");
    conn.query_row(&sql, params![id], map_row)
        .optional()?
        .ok_or_else(|| AppError::not_found("chapter", id))
}

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
