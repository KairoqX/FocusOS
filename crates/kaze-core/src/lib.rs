//! Kaze Study core: validation, models and SQLite repositories.
//! Deliberately free of Tauri/UI dependencies so it is portable and testable alone.

pub mod chapters;
pub mod db;
pub mod error;
pub mod focus;
pub mod models;
pub mod progress;
pub mod settings;
pub mod subjects;
pub mod tasks;
pub mod timer;
pub mod validation;
