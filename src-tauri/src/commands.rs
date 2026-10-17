//! Thin Tauri command layer. All logic lives in `focusos-core`.

use focusos_core::db::{self, Db};
use focusos_core::error::AppError;
use focusos_core::models::{Subject, Task, TaskInput, TaskQuery};
use focusos_core::{subjects, tasks};
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    name: &'static str,
    version: &'static str,
    os: &'static str,
    arch: &'static str,
    schema_version: i64,
    db_path: String,
}

#[tauri::command]
pub fn get_app_info(db: State<'_, Db>) -> Result<AppInfo, AppError> {
    let schema_version = db.with(db::schema_version)?;
    Ok(AppInfo {
        name: "FocusOS",
        version: env!("CARGO_PKG_VERSION"),
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        schema_version,
        db_path: db.path().display().to_string(),
    })
}

#[tauri::command]
pub fn list_tasks(db: State<'_, Db>, query: TaskQuery) -> Result<Vec<Task>, AppError> {
    db.with(|conn| {
        let today = db::local_today(conn)?;
        tasks::list(conn, &query, &today)
    })
}

#[tauri::command]
pub fn create_task(db: State<'_, Db>, input: TaskInput) -> Result<Task, AppError> {
    db.with(|conn| {
        let today = db::local_today(conn)?;
        tasks::create(conn, &input, &today)
    })
}

#[tauri::command]
pub fn update_task(db: State<'_, Db>, id: i64, input: TaskInput) -> Result<Task, AppError> {
    db.with(|conn| {
        let today = db::local_today(conn)?;
        tasks::update(conn, id, &input, &today)
    })
}

#[tauri::command]
pub fn delete_task(db: State<'_, Db>, id: i64) -> Result<(), AppError> {
    db.with(|conn| tasks::delete(conn, id))
}

#[tauri::command]
pub fn set_task_completed(db: State<'_, Db>, id: i64, completed: bool) -> Result<Task, AppError> {
    db.with(|conn| {
        let today = db::local_today(conn)?;
        tasks::set_completed(conn, id, completed, &today)
    })
}

#[tauri::command]
pub fn list_subjects(db: State<'_, Db>) -> Result<Vec<Subject>, AppError> {
    db.with(subjects::list)
}
