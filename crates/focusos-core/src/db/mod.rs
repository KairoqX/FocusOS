mod migrations;

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::Connection;

use crate::error::{AppError, AppResult};
pub use migrations::{latest_version, run_migrations};

/// Shared database handle (one connection behind a mutex; plenty for a local desktop app).
pub struct Db {
    conn: Mutex<Connection>,
    path: PathBuf,
}

impl Db {
    pub fn new(conn: Connection, path: PathBuf) -> Self {
        Db {
            conn: Mutex::new(conn),
            path,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Runs `f` with exclusive access to the connection.
    pub fn with<T>(&self, f: impl FnOnce(&Connection) -> AppResult<T>) -> AppResult<T> {
        let guard = self
            .conn
            .lock()
            .map_err(|_| AppError::internal("database lock was poisoned"))?;
        f(&guard)
    }
}

/// Opens (creating if needed) the database file, configures it and applies migrations.
pub fn open(path: &Path) -> AppResult<Connection> {
    let mut conn = Connection::open(path)?;
    prepare(&mut conn)?;
    Ok(conn)
}

pub fn open_in_memory() -> AppResult<Connection> {
    let mut conn = Connection::open_in_memory()?;
    prepare(&mut conn)?;
    Ok(conn)
}

fn prepare(conn: &mut Connection) -> AppResult<()> {
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 5000;")?;
    let fk: i64 = conn.query_row("PRAGMA foreign_keys", [], |r| r.get(0))?;
    if fk != 1 {
        return Err(AppError::internal(
            "SQLite foreign key enforcement could not be enabled",
        ));
    }
    run_migrations(conn)
}

pub fn schema_version(conn: &Connection) -> AppResult<i64> {
    Ok(conn.query_row("PRAGMA user_version", [], |r| r.get(0))?)
}

/// Today's date in the user's local time zone, `YYYY-MM-DD`.
pub fn local_today(conn: &Connection) -> AppResult<String> {
    Ok(conn.query_row("SELECT date('now', 'localtime')", [], |r| r.get(0))?)
}
