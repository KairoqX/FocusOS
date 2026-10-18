//! Typed access to the `app_settings` key/value table.

use rusqlite::{params, Connection, OptionalExtension};

use crate::error::{AppError, AppResult};
use crate::models::TimerSettings;

pub const DEFAULT_FOCUS_MINUTES: u32 = 25;
pub const DEFAULT_BREAK_MINUTES: u32 = 5;
pub const MAX_FOCUS_MINUTES: u32 = 240;
pub const MAX_BREAK_MINUTES: u32 = 60;

const FOCUS_KEY: &str = "timer.focus_minutes";
const BREAK_KEY: &str = "timer.break_minutes";

pub fn defaults() -> TimerSettings {
    TimerSettings {
        focus_minutes: DEFAULT_FOCUS_MINUTES,
        break_minutes: DEFAULT_BREAK_MINUTES,
    }
}

/// Missing or unreadable values fall back to the defaults, so a damaged row can never block the timer.
pub fn get_timer(conn: &Connection) -> AppResult<TimerSettings> {
    Ok(TimerSettings {
        focus_minutes: read_minutes(conn, FOCUS_KEY, DEFAULT_FOCUS_MINUTES, MAX_FOCUS_MINUTES)?,
        break_minutes: read_minutes(conn, BREAK_KEY, DEFAULT_BREAK_MINUTES, MAX_BREAK_MINUTES)?,
    })
}

pub fn validate_timer(input: &TimerSettings) -> AppResult<()> {
    if !(1..=MAX_FOCUS_MINUTES).contains(&input.focus_minutes) {
        return Err(AppError::validation(
            "focusMinutes",
            format!("Focus duration must be between 1 and {MAX_FOCUS_MINUTES} minutes."),
        ));
    }
    if !(1..=MAX_BREAK_MINUTES).contains(&input.break_minutes) {
        return Err(AppError::validation(
            "breakMinutes",
            format!("Break duration must be between 1 and {MAX_BREAK_MINUTES} minutes."),
        ));
    }
    Ok(())
}

/// Validates, then writes both values atomically.
pub fn set_timer(conn: &Connection, input: &TimerSettings) -> AppResult<TimerSettings> {
    validate_timer(input)?;
    let tx = conn.unchecked_transaction()?;
    upsert(&tx, FOCUS_KEY, input.focus_minutes)?;
    upsert(&tx, BREAK_KEY, input.break_minutes)?;
    tx.commit()?;
    Ok(*input)
}

fn upsert(conn: &Connection, key: &str, minutes: u32) -> AppResult<()> {
    conn.execute(
        "INSERT INTO app_settings (key, value) VALUES (?1, ?2) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')",
        params![key, minutes.to_string()],
    )?;
    Ok(())
}

fn read_minutes(conn: &Connection, key: &str, default: u32, max: u32) -> AppResult<u32> {
    let raw: Option<String> = conn
        .query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            params![key],
            |r| r.get(0),
        )
        .optional()?;
    Ok(raw
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|v| (1..=max).contains(v))
        .unwrap_or(default))
}
