mod commands;

use kaze_core::db;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Per-user app-data directory (portable across Windows/Linux/macOS).
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            // Internal names (identifier, file name) intentionally predate the "Kaze Study" rename:
            // changing them would move the data folder and orphan existing user data.
            let path = dir.join("focusos.db");
            let conn = db::open(&path)?;
            app.manage(db::Db::new(conn, path));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            commands::list_tasks,
            commands::create_task,
            commands::update_task,
            commands::delete_task,
            commands::set_task_completed,
            commands::list_subjects,
            commands::create_subject,
            commands::update_subject,
            commands::delete_subject,
            commands::list_chapters,
            commands::create_chapter,
            commands::update_chapter,
            commands::update_chapter_status,
            commands::update_chapter_progress,
            commands::delete_chapter,
            commands::get_focus_state,
            commands::focus_heartbeat,
            commands::start_focus_session,
            commands::pause_focus_session,
            commands::resume_focus_session,
            commands::complete_focus_session,
            commands::stop_focus_session,
            commands::reset_focus_session,
            commands::list_focus_sessions,
            commands::get_timer_settings,
            commands::update_timer_settings,
        ])
        .run(tauri::generate_context!())
        // Failing to start the runtime at launch is unrecoverable.
        .expect("failed to start Kaze Study");
}
