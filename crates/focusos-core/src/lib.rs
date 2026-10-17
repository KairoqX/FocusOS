//! FocusOS core: validation, models and SQLite repositories.
//! Deliberately free of Tauri/UI dependencies so it is portable and testable alone.

pub mod db;
pub mod error;
pub mod models;
pub mod subjects;
pub mod tasks;
pub mod validation;
