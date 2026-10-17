use rusqlite::Connection;

use crate::error::AppResult;
use crate::models::Subject;

/// Read-only for now; subject management arrives in Milestone 3.
pub fn list(conn: &Connection) -> AppResult<Vec<Subject>> {
    let mut stmt =
        conn.prepare("SELECT id, name FROM subjects ORDER BY name COLLATE NOCASE, id")?;
    let rows = stmt.query_map([], |row| {
        Ok(Subject {
            id: row.get("id")?,
            name: row.get("name")?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<Subject>>>()?)
}
