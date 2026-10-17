use rusqlite::types::Value;
use rusqlite::{params, params_from_iter, Connection, OptionalExtension, Row};

use crate::error::{AppError, AppResult};
use crate::models::{Task, TaskInput, TaskQuery, TaskSort, TaskView};
use crate::validation::validate_task;

// The first `?` is `today` (for the computed `overdue` column); callers pass it first.
const SELECT: &str = "SELECT t.id, t.title, t.description, t.subject_id, s.name AS subject_name, \
    t.priority, t.due_date, t.estimated_minutes, t.completed_at, t.created_at, t.updated_at, \
    (t.completed_at IS NULL AND t.due_date IS NOT NULL AND t.due_date < ?) AS overdue \
    FROM tasks t LEFT JOIN subjects s ON s.id = t.subject_id";

fn map_row(row: &Row<'_>) -> rusqlite::Result<Task> {
    let completed_at: Option<String> = row.get("completed_at")?;
    Ok(Task {
        id: row.get("id")?,
        title: row.get("title")?,
        description: row.get("description")?,
        subject_id: row.get("subject_id")?,
        subject_name: row.get("subject_name")?,
        priority: row.get("priority")?,
        due_date: row.get("due_date")?,
        estimated_minutes: row.get("estimated_minutes")?,
        completed: completed_at.is_some(),
        completed_at,
        overdue: row.get("overdue")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

/// `today` is a `YYYY-MM-DD` local date (see `db::local_today`); passed in so logic is testable.
pub fn list(conn: &Connection, query: &TaskQuery, today: &str) -> AppResult<Vec<Task>> {
    let mut clauses: Vec<&'static str> = Vec::new();
    let mut values: Vec<Value> = vec![Value::Text(today.to_owned())];

    match query.view {
        TaskView::Today => {
            clauses.push("t.completed_at IS NULL AND t.due_date IS NOT NULL AND t.due_date <= ?");
            values.push(Value::Text(today.to_owned()));
        }
        TaskView::Upcoming => {
            clauses.push("t.completed_at IS NULL AND t.due_date > ?");
            values.push(Value::Text(today.to_owned()));
        }
        TaskView::Completed => clauses.push("t.completed_at IS NOT NULL"),
        TaskView::All => {}
    }
    if let Some(id) = query.subject_id {
        clauses.push("t.subject_id = ?");
        values.push(Value::Integer(id));
    }
    if let Some(p) = query.priority {
        clauses.push("t.priority = ?");
        values.push(Value::Text(p.as_str().to_owned()));
    }
    if let Some(term) = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        clauses.push("(t.title LIKE ? ESCAPE '\\' OR t.description LIKE ? ESCAPE '\\')");
        let pattern = like_pattern(term);
        values.push(Value::Text(pattern.clone()));
        values.push(Value::Text(pattern));
    }

    let mut sql = String::from(SELECT);
    if !clauses.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&clauses.join(" AND "));
    }
    // ORDER BY comes from a closed enum, never from user text.
    sql.push_str(match query.sort {
        TaskSort::DueDate => {
            " ORDER BY t.completed_at IS NOT NULL, t.due_date IS NULL, t.due_date ASC, \
              CASE t.priority WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END, t.id ASC"
        }
        TaskSort::Priority => {
            " ORDER BY t.completed_at IS NOT NULL, \
              CASE t.priority WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END, \
              t.due_date IS NULL, t.due_date ASC, t.id ASC"
        }
        TaskSort::Created => " ORDER BY t.created_at DESC, t.id DESC",
        TaskSort::Title => " ORDER BY t.title COLLATE NOCASE ASC, t.id ASC",
    });

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(values), map_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<Task>>>()?)
}

pub fn get(conn: &Connection, id: i64, today: &str) -> AppResult<Task> {
    let sql = format!("{SELECT} WHERE t.id = ?");
    conn.query_row(&sql, params![today, id], map_row)
        .optional()?
        .ok_or_else(|| AppError::not_found("task", id))
}

pub fn create(conn: &Connection, input: &TaskInput, today: &str) -> AppResult<Task> {
    let v = validate_task(input)?;
    ensure_subject_exists(conn, v.subject_id)?;
    conn.execute(
        "INSERT INTO tasks (title, description, subject_id, priority, due_date, estimated_minutes) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            v.title,
            v.description,
            v.subject_id,
            v.priority.as_str(),
            v.due_date,
            v.estimated_minutes
        ],
    )?;
    get(conn, conn.last_insert_rowid(), today)
}

pub fn update(conn: &Connection, id: i64, input: &TaskInput, today: &str) -> AppResult<Task> {
    let v = validate_task(input)?;
    ensure_subject_exists(conn, v.subject_id)?;
    let changed = conn.execute(
        "UPDATE tasks SET title = ?1, description = ?2, subject_id = ?3, priority = ?4, \
         due_date = ?5, estimated_minutes = ?6, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?7",
        params![
            v.title,
            v.description,
            v.subject_id,
            v.priority.as_str(),
            v.due_date,
            v.estimated_minutes,
            id
        ],
    )?;
    if changed == 0 {
        return Err(AppError::not_found("task", id));
    }
    get(conn, id, today)
}

pub fn delete(conn: &Connection, id: i64) -> AppResult<()> {
    let changed = conn.execute("DELETE FROM tasks WHERE id = ?1", params![id])?;
    if changed == 0 {
        return Err(AppError::not_found("task", id));
    }
    Ok(())
}

/// Marks a task completed or open. Completing an already-completed task keeps the
/// original completion time (idempotent), which keeps insights accurate.
pub fn set_completed(conn: &Connection, id: i64, completed: bool, today: &str) -> AppResult<Task> {
    let changed = conn.execute(
        "UPDATE tasks SET \
         completed_at = CASE WHEN ?1 THEN COALESCE(completed_at, strftime('%Y-%m-%dT%H:%M:%SZ', 'now')) ELSE NULL END, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?2",
        params![completed, id],
    )?;
    if changed == 0 {
        return Err(AppError::not_found("task", id));
    }
    get(conn, id, today)
}

fn ensure_subject_exists(conn: &Connection, subject_id: Option<i64>) -> AppResult<()> {
    if let Some(id) = subject_id {
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM subjects WHERE id = ?1)",
            params![id],
            |r| r.get(0),
        )?;
        if !exists {
            return Err(AppError::validation(
                "subjectId",
                "The selected subject no longer exists.",
            ));
        }
    }
    Ok(())
}

/// Builds a LIKE pattern matching `term` literally (escapes `%`, `_` and `\`).
fn like_pattern(term: &str) -> String {
    let mut out = String::with_capacity(term.len() + 2);
    out.push('%');
    for c in term.chars() {
        if matches!(c, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('%');
    out
}

#[cfg(test)]
mod tests {
    use super::like_pattern;

    #[test]
    fn like_pattern_escapes_wildcards() {
        assert_eq!(like_pattern("100%"), "%100\\%%");
        assert_eq!(like_pattern("a_b"), "%a\\_b%");
        assert_eq!(like_pattern("c\\d"), "%c\\\\d%");
    }
}
