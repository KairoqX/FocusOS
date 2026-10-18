use std::collections::HashMap;

use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::chapters;
use crate::error::{AppError, AppResult};
use crate::models::{Chapter, ChapterStatus, ProgressMode, Subject, SubjectInput};
use crate::progress::subject_progress;
use crate::validation::validate_subject_name;

// Task counts come from the tasks table; chapter-derived fields are filled in from the very
// same chapter rows the UI lists, so the numbers on screen always reconcile.
const SELECT: &str = "SELECT s.id, s.name, \
    (SELECT COUNT(*) FROM tasks t WHERE t.subject_id = s.id) AS task_count, \
    (SELECT COUNT(*) FROM tasks t WHERE t.subject_id = s.id AND t.completed_at IS NULL) \
        AS open_tasks, \
    (SELECT COUNT(*) FROM tasks t WHERE t.subject_id = s.id AND t.chapter_id IS NULL) \
        AS unassigned_tasks \
    FROM subjects s";

fn map_row(row: &Row<'_>) -> rusqlite::Result<Subject> {
    Ok(Subject {
        id: row.get("id")?,
        name: row.get("name")?,
        chapter_count: 0,
        not_started: 0,
        in_progress: 0,
        completed: 0,
        needs_revision: 0,
        chapters_without_tasks: 0,
        percent_complete: 0,
        task_count: row.get("task_count")?,
        open_tasks: row.get("open_tasks")?,
        unassigned_tasks: row.get("unassigned_tasks")?,
    })
}

/// Fills the chapter-derived fields. An empty slice leaves everything at zero (no division).
fn apply_chapters(subject: &mut Subject, chapters: &[Chapter]) {
    subject.chapter_count = chapters.len() as u32;
    for c in chapters {
        match c.status {
            ChapterStatus::NotStarted => subject.not_started += 1,
            ChapterStatus::InProgress => subject.in_progress += 1,
            ChapterStatus::Completed => subject.completed += 1,
            ChapterStatus::NeedsRevision => subject.needs_revision += 1,
        }
        if c.progress_mode == ProgressMode::Automatic && c.task_total == 0 {
            subject.chapters_without_tasks += 1;
        }
    }
    let percents: Vec<u8> = chapters.iter().map(|c| c.progress_percent).collect();
    subject.percent_complete = subject_progress(&percents);
}

pub fn list(conn: &Connection) -> AppResult<Vec<Subject>> {
    let sql = format!("{SELECT} ORDER BY s.name COLLATE NOCASE, s.id");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], map_row)?;
    let mut subjects = rows.collect::<rusqlite::Result<Vec<Subject>>>()?;

    let mut by_subject: HashMap<i64, Vec<Chapter>> = HashMap::new();
    for chapter in chapters::query(conn, None)? {
        by_subject
            .entry(chapter.subject_id)
            .or_default()
            .push(chapter);
    }
    for subject in &mut subjects {
        if let Some(list) = by_subject.get(&subject.id) {
            apply_chapters(subject, list);
        }
    }
    Ok(subjects)
}

pub fn get(conn: &Connection, id: i64) -> AppResult<Subject> {
    let sql = format!("{SELECT} WHERE s.id = ?1");
    let mut subject = conn
        .query_row(&sql, params![id], map_row)
        .optional()?
        .ok_or_else(|| AppError::not_found("subject", id))?;
    let list = chapters::query(conn, Some(id))?;
    apply_chapters(&mut subject, &list);
    Ok(subject)
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
/// Linked tasks are kept; their subject and chapter become empty.
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
