use rusqlite::Connection;

use super::schema_version;
use crate::error::{AppError, AppResult};

struct Migration {
    version: i64,
    name: &'static str,
    sql: &'static str,
}

/// Append-only, strictly increasing versions. Never edit a released migration.
const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "init_subjects_tasks",
    sql: include_str!("../../migrations/0001_init.sql"),
}];

pub fn latest_version() -> i64 {
    MIGRATIONS.last().map_or(0, |m| m.version)
}

/// Applies pending migrations, each in its own transaction, tracked by `PRAGMA user_version`.
pub fn run_migrations(conn: &mut Connection) -> AppResult<()> {
    let current = schema_version(conn)?;
    let latest = latest_version();
    if current > latest {
        return Err(AppError::internal(format!(
            "This database uses schema version {current}, but this version of FocusOS only \
             supports up to {latest}. Please update FocusOS."
        )));
    }

    for m in MIGRATIONS.iter().filter(|m| m.version > current) {
        let tx = conn.transaction()?;
        tx.execute_batch(m.sql).map_err(|e| {
            AppError::internal(format!("migration {} ({}) failed: {e}", m.version, m.name))
        })?;
        tx.execute_batch(&format!("PRAGMA user_version = {}", m.version))?;
        tx.commit()?;
    }
    Ok(())
}
