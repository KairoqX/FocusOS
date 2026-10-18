//! Thin Tauri command layer. All logic lives in `kaze-core`.

use kaze_core::db::{self, Db};
use kaze_core::error::AppError;
use kaze_core::models::{
    Chapter, ChapterInput, ChapterProgressInput, ChapterStatus, FocusHistory, StartSessionInput,
    Subject, SubjectInput, Task, TaskInput, TaskQuery, TimerSettings, TimerSnapshot,
};
use kaze_core::{chapters, focus, settings, subjects, tasks};
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
        name: "Kaze Study",
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

#[tauri::command]
pub fn create_subject(db: State<'_, Db>, input: SubjectInput) -> Result<Subject, AppError> {
    db.with(|conn| subjects::create(conn, &input))
}

#[tauri::command]
pub fn update_subject(
    db: State<'_, Db>,
    id: i64,
    input: SubjectInput,
) -> Result<Subject, AppError> {
    db.with(|conn| subjects::update(conn, id, &input))
}

#[tauri::command]
pub fn delete_subject(db: State<'_, Db>, id: i64) -> Result<(), AppError> {
    db.with(|conn| subjects::delete(conn, id))
}

#[tauri::command]
pub fn list_chapters(db: State<'_, Db>, subject_id: i64) -> Result<Vec<Chapter>, AppError> {
    db.with(|conn| chapters::list(conn, subject_id))
}

#[tauri::command]
pub fn create_chapter(
    db: State<'_, Db>,
    subject_id: i64,
    input: ChapterInput,
) -> Result<Chapter, AppError> {
    db.with(|conn| chapters::create(conn, subject_id, &input))
}

#[tauri::command]
pub fn update_chapter(
    db: State<'_, Db>,
    id: i64,
    input: ChapterInput,
) -> Result<Chapter, AppError> {
    db.with(|conn| chapters::update(conn, id, &input))
}

#[tauri::command]
pub fn update_chapter_status(
    db: State<'_, Db>,
    id: i64,
    status: ChapterStatus,
) -> Result<Chapter, AppError> {
    db.with(|conn| chapters::set_status(conn, id, status))
}

#[tauri::command]
pub fn update_chapter_progress(
    db: State<'_, Db>,
    id: i64,
    input: ChapterProgressInput,
) -> Result<Chapter, AppError> {
    db.with(|conn| chapters::set_progress(conn, id, &input))
}

#[tauri::command]
pub fn delete_chapter(db: State<'_, Db>, id: i64) -> Result<(), AppError> {
    db.with(|conn| chapters::delete(conn, id))
}

// ----- Focus timer. The wall clock is read here; all logic lives in `kaze-core`. -----

#[tauri::command]
pub fn get_focus_state(db: State<'_, Db>) -> Result<TimerSnapshot, AppError> {
    db.with(|conn| focus::status(conn, focus::system_now_ms()?))
}

#[tauri::command]
pub fn focus_heartbeat(db: State<'_, Db>) -> Result<TimerSnapshot, AppError> {
    db.with(|conn| focus::heartbeat(conn, focus::system_now_ms()?))
}

#[tauri::command]
pub fn start_focus_session(
    db: State<'_, Db>,
    input: StartSessionInput,
) -> Result<TimerSnapshot, AppError> {
    db.with(|conn| focus::start(conn, &input, focus::system_now_ms()?))
}

#[tauri::command]
pub fn pause_focus_session(db: State<'_, Db>, id: i64) -> Result<TimerSnapshot, AppError> {
    db.with(|conn| focus::pause(conn, id, focus::system_now_ms()?))
}

#[tauri::command]
pub fn resume_focus_session(db: State<'_, Db>, id: i64) -> Result<TimerSnapshot, AppError> {
    db.with(|conn| focus::resume(conn, id, focus::system_now_ms()?))
}

#[tauri::command]
pub fn complete_focus_session(db: State<'_, Db>, id: i64) -> Result<TimerSnapshot, AppError> {
    db.with(|conn| focus::complete(conn, id, focus::system_now_ms()?))
}

#[tauri::command]
pub fn stop_focus_session(db: State<'_, Db>, id: i64) -> Result<TimerSnapshot, AppError> {
    db.with(|conn| focus::stop(conn, id, focus::system_now_ms()?))
}

#[tauri::command]
pub fn reset_focus_session(db: State<'_, Db>, id: i64) -> Result<TimerSnapshot, AppError> {
    db.with(|conn| focus::reset(conn, id, focus::system_now_ms()?))
}

#[tauri::command]
pub fn list_focus_sessions(
    db: State<'_, Db>,
    limit: Option<u32>,
) -> Result<FocusHistory, AppError> {
    db.with(|conn| {
        let today = db::local_today(conn)?;
        focus::history(conn, limit.unwrap_or(30), &today, focus::system_now_ms()?)
    })
}

#[tauri::command]
pub fn get_timer_settings(db: State<'_, Db>) -> Result<TimerSettings, AppError> {
    db.with(settings::get_timer)
}

#[tauri::command]
pub fn update_timer_settings(
    db: State<'_, Db>,
    input: TimerSettings,
) -> Result<TimerSettings, AppError> {
    db.with(|conn| settings::set_timer(conn, &input))
}
