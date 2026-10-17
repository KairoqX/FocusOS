use focusos_core::db;
use focusos_core::error::AppError;
use rusqlite::Connection;
use std::path::PathBuf;

fn table_exists(conn: &Connection, name: &str) -> bool {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
        [name],
        |r| r.get(0),
    )
    .unwrap()
}

#[test]
fn fresh_database_is_migrated_to_latest() {
    let conn = db::open_in_memory().unwrap();
    assert_eq!(db::schema_version(&conn).unwrap(), db::latest_version());
    assert!(table_exists(&conn, "tasks"));
    assert!(table_exists(&conn, "subjects"));
}

#[test]
fn running_migrations_twice_is_a_no_op() {
    let mut conn = db::open_in_memory().unwrap();
    db::run_migrations(&mut conn).unwrap();
    assert_eq!(db::schema_version(&conn).unwrap(), db::latest_version());
}

#[test]
fn newer_database_is_rejected() {
    let mut conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA user_version = 999").unwrap();
    match db::run_migrations(&mut conn) {
        Err(AppError::Internal { message }) => assert!(message.contains("999")),
        other => panic!("expected internal error, got {other:?}"),
    }
}

#[test]
fn foreign_keys_are_enabled() {
    let conn = db::open_in_memory().unwrap();
    let fk: i64 = conn
        .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
        .unwrap();
    assert_eq!(fk, 1);
}

struct TempDb(PathBuf);
impl Drop for TempDb {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[test]
fn data_survives_reopening_the_file() {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("focusos-test-{}-{nanos}.db", std::process::id()));
    let _guard = TempDb(path.clone());

    {
        let conn = db::open(&path).unwrap();
        conn.execute("INSERT INTO subjects (name) VALUES ('Physics')", [])
            .unwrap();
    }
    let conn = db::open(&path).unwrap();
    let n: i64 = conn
        .query_row("SELECT COUNT(*) FROM subjects", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 1);
    assert_eq!(db::schema_version(&conn).unwrap(), db::latest_version());
}
