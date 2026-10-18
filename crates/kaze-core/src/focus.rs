//! Focus/break session state machine on top of SQLite.
//!
//! Every function takes `now_ms` (UTC epoch milliseconds) explicitly, so behaviour over time,
//! including restarts and sleep, is fully testable. Nothing depends on in-memory ticking.
//!
//! Lifecycle: `running <-> paused` -> `completed` | `stopped`, or discarded by `reset`.
//! At most one session is active at a time (also enforced by a unique index).

use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::error::{AppError, AppResult};
use crate::models::{
    FocusHistory, FocusSession, SessionKind, SessionState, StartSessionInput, TimerSnapshot,
};
use crate::{settings, timer};

/// Current wall-clock time as UTC epoch milliseconds.
pub fn system_now_ms() -> AppResult<i64> {
    let since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| AppError::internal("the system clock is set before 1970"))?;
    i64::try_from(since_epoch.as_millis())
        .map_err(|_| AppError::internal("the system clock is out of range"))
}

const SELECT: &str = "SELECT f.id, f.kind, f.state, f.planned_seconds, f.accumulated_ms, \
    f.run_started_at_ms, f.started_at_ms, f.ended_at_ms, f.auto_paused, f.task_id, \
    t.title AS task_title, f.subject_id, s.name AS subject_name \
    FROM focus_sessions f \
    LEFT JOIN tasks t ON t.id = f.task_id \
    LEFT JOIN subjects s ON s.id = f.subject_id";

fn map_session(row: &Row<'_>, now_ms: i64) -> rusqlite::Result<FocusSession> {
    let state: SessionState = row.get("state")?;
    let planned_ms = row.get::<_, i64>("planned_seconds")?.saturating_mul(1000);
    let accumulated: i64 = row.get("accumulated_ms")?;
    let run_started: Option<i64> = row.get("run_started_at_ms")?;
    let (elapsed_ms, remaining_ms) = if state.is_active() {
        (
            timer::elapsed_ms(accumulated, run_started, planned_ms, now_ms),
            timer::remaining_ms(accumulated, run_started, planned_ms, now_ms),
        )
    } else {
        (accumulated.clamp(0, planned_ms), 0)
    };
    Ok(FocusSession {
        id: row.get("id")?,
        kind: row.get("kind")?,
        state,
        planned_ms,
        elapsed_ms,
        remaining_ms,
        started_at_ms: row.get("started_at_ms")?,
        ended_at_ms: row.get("ended_at_ms")?,
        auto_paused: row.get("auto_paused")?,
        task_id: row.get("task_id")?,
        task_title: row.get("task_title")?,
        subject_id: row.get("subject_id")?,
        subject_name: row.get("subject_name")?,
    })
}

fn load_session(conn: &Connection, id: i64, now_ms: i64) -> AppResult<FocusSession> {
    let sql = format!("{SELECT} WHERE f.id = ?1");
    conn.query_row(&sql, params![id], |r| map_session(r, now_ms))
        .optional()?
        .ok_or_else(|| AppError::not_found("session", id))
}

fn load_active(conn: &Connection, now_ms: i64) -> AppResult<Option<FocusSession>> {
    let sql = format!("{SELECT} WHERE f.state IN ('running', 'paused') LIMIT 1");
    Ok(conn
        .query_row(&sql, [], |r| map_session(r, now_ms))
        .optional()?)
}

/// The bare columns the state machine needs (no joins).
struct Raw {
    state: SessionState,
    planned_ms: i64,
    accumulated_ms: i64,
    run_started_ms: Option<i64>,
    started_at_ms: i64,
}

fn find_raw(conn: &Connection, id: i64) -> AppResult<Option<Raw>> {
    Ok(conn
        .query_row(
            "SELECT state, planned_seconds, accumulated_ms, run_started_at_ms, started_at_ms \
             FROM focus_sessions WHERE id = ?1",
            params![id],
            |r| {
                Ok(Raw {
                    state: r.get(0)?,
                    planned_ms: r.get::<_, i64>(1)?.saturating_mul(1000),
                    accumulated_ms: r.get(2)?,
                    run_started_ms: r.get(3)?,
                    started_at_ms: r.get(4)?,
                })
            },
        )
        .optional()?)
}

fn load_raw(conn: &Connection, id: i64) -> AppResult<Raw> {
    find_raw(conn, id)?.ok_or_else(|| AppError::not_found("session", id))
}

fn snapshot(
    conn: &Connection,
    now_ms: i64,
    finished: Option<FocusSession>,
) -> AppResult<TimerSnapshot> {
    Ok(TimerSnapshot {
        active: load_active(conn, now_ms)?,
        finished,
        now_ms,
    })
}

/// Detects a running session that nobody has observed for too long (app closed, computer asleep,
/// crash) and pauses it, crediting only the observed time. Safe to call repeatedly.
fn reconcile(conn: &Connection, now_ms: i64) -> AppResult<()> {
    let running: Option<(i64, i64, i64, i64)> = conn
        .query_row(
            "SELECT id, accumulated_ms, run_started_at_ms, last_seen_ms \
             FROM focus_sessions WHERE state = 'running'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?;
    if let Some((id, accumulated, run_started, last_seen)) = running {
        if let Some(credited) =
            timer::interrupted_accumulated(accumulated, run_started, last_seen, now_ms)
        {
            conn.execute(
                "UPDATE focus_sessions SET state = 'paused', run_started_at_ms = NULL, \
                 accumulated_ms = ?1, auto_paused = 1, last_seen_ms = ?2 \
                 WHERE id = ?3 AND state = 'running'",
                params![credited, now_ms, id],
            )?;
        }
    }
    Ok(())
}

/// Current state (after recovering from any interruption). Call on startup and when the window
/// becomes visible again.
pub fn status(conn: &Connection, now_ms: i64) -> AppResult<TimerSnapshot> {
    reconcile(conn, now_ms)?;
    snapshot(conn, now_ms, None)
}

/// Proof of life while a session runs; the UI calls this every few seconds.
pub fn heartbeat(conn: &Connection, now_ms: i64) -> AppResult<TimerSnapshot> {
    reconcile(conn, now_ms)?;
    conn.execute(
        "UPDATE focus_sessions SET last_seen_ms = ?1 WHERE state = 'running'",
        params![now_ms],
    )?;
    snapshot(conn, now_ms, None)
}

pub fn start(
    conn: &Connection,
    input: &StartSessionInput,
    now_ms: i64,
) -> AppResult<TimerSnapshot> {
    reconcile(conn, now_ms)?;
    if load_active(conn, now_ms)?.is_some() {
        return Err(AppError::conflict(
            "A session is already in progress. Finish or reset it first.",
        ));
    }
    let cfg = settings::get_timer(conn)?;
    let minutes = match input.kind {
        SessionKind::Focus => cfg.focus_minutes,
        SessionKind::Break => cfg.break_minutes,
    };
    let planned_seconds = i64::from(minutes) * 60;
    let (task_id, subject_id) = match (input.kind, input.task_id) {
        (SessionKind::Focus, Some(task_id)) => resolve_task(conn, task_id)?,
        _ => (None, None),
    };
    conn.execute(
        "INSERT INTO focus_sessions (kind, state, planned_seconds, task_id, subject_id, \
         started_at_ms, run_started_at_ms, accumulated_ms, last_seen_ms) \
         VALUES (?1, 'running', ?2, ?3, ?4, ?5, ?5, 0, ?5)",
        params![
            input.kind.as_str(),
            planned_seconds,
            task_id,
            subject_id,
            now_ms
        ],
    )?;
    snapshot(conn, now_ms, None)
}

/// Returns `(task_id, subject_id)` for an open task.
fn resolve_task(conn: &Connection, task_id: i64) -> AppResult<(Option<i64>, Option<i64>)> {
    let row: Option<(Option<i64>, bool)> = conn
        .query_row(
            "SELECT subject_id, completed_at IS NOT NULL FROM tasks WHERE id = ?1",
            params![task_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    match row {
        None => Err(AppError::not_found("task", task_id)),
        Some((_, true)) => Err(AppError::validation(
            "taskId",
            "Choose a task that is not completed.",
        )),
        Some((subject_id, false)) => Ok((Some(task_id), subject_id)),
    }
}

/// Pausing an already-paused session is a harmless no-op (double clicks, retries).
pub fn pause(conn: &Connection, id: i64, now_ms: i64) -> AppResult<TimerSnapshot> {
    reconcile(conn, now_ms)?;
    let raw = load_raw(conn, id)?;
    match raw.state {
        SessionState::Running => {
            let banked = timer::raw_elapsed_ms(raw.accumulated_ms, raw.run_started_ms, now_ms);
            conn.execute(
                "UPDATE focus_sessions SET state = 'paused', run_started_at_ms = NULL, \
                 accumulated_ms = ?1, last_seen_ms = ?2 WHERE id = ?3 AND state = 'running'",
                params![banked, now_ms, id],
            )?;
        }
        SessionState::Paused => {}
        SessionState::Completed | SessionState::Stopped => {
            return Err(AppError::conflict("This session has already ended."));
        }
    }
    snapshot(conn, now_ms, None)
}

/// Resuming an already-running session is a harmless no-op.
pub fn resume(conn: &Connection, id: i64, now_ms: i64) -> AppResult<TimerSnapshot> {
    reconcile(conn, now_ms)?;
    let raw = load_raw(conn, id)?;
    match raw.state {
        SessionState::Paused => {
            conn.execute(
                "UPDATE focus_sessions SET state = 'running', run_started_at_ms = ?1, \
                 last_seen_ms = ?1, auto_paused = 0 WHERE id = ?2 AND state = 'paused'",
                params![now_ms, id],
            )?;
        }
        SessionState::Running => {}
        SessionState::Completed | SessionState::Stopped => {
            return Err(AppError::conflict("This session has already ended."));
        }
    }
    snapshot(conn, now_ms, None)
}

/// Records a session as completed once its planned time has really elapsed.
/// Idempotent: completing an already-completed session returns it again and writes nothing,
/// so retries and double calls can never record a session twice.
pub fn complete(conn: &Connection, id: i64, now_ms: i64) -> AppResult<TimerSnapshot> {
    reconcile(conn, now_ms)?;
    let raw = load_raw(conn, id)?;
    match raw.state {
        SessionState::Completed => {
            return snapshot(conn, now_ms, Some(load_session(conn, id, now_ms)?));
        }
        SessionState::Stopped => {
            return Err(AppError::conflict("This session was already stopped."));
        }
        SessionState::Running | SessionState::Paused => {}
    }
    let elapsed = timer::raw_elapsed_ms(raw.accumulated_ms, raw.run_started_ms, now_ms);
    if !timer::is_finished(elapsed, raw.planned_ms) {
        return Err(AppError::conflict("This session has not finished yet."));
    }
    let ended_at = timer::true_end_ms(
        raw.run_started_ms,
        raw.accumulated_ms,
        raw.planned_ms,
        now_ms,
    )
    .max(raw.started_at_ms);
    conn.execute(
        "UPDATE focus_sessions SET state = 'completed', run_started_at_ms = NULL, \
         accumulated_ms = ?1, ended_at_ms = ?2, last_seen_ms = ?3 \
         WHERE id = ?4 AND state IN ('running', 'paused')",
        params![raw.planned_ms, ended_at, now_ms, id],
    )?;
    snapshot(conn, now_ms, Some(load_session(conn, id, now_ms)?))
}

/// Ends a session early and keeps the time that really elapsed ("Stop & save").
/// Sessions shorter than a minute are not saved (use `reset`). Idempotent for ended sessions.
pub fn stop(conn: &Connection, id: i64, now_ms: i64) -> AppResult<TimerSnapshot> {
    reconcile(conn, now_ms)?;
    let raw = load_raw(conn, id)?;
    if !raw.state.is_active() {
        return snapshot(conn, now_ms, Some(load_session(conn, id, now_ms)?));
    }
    let elapsed = timer::raw_elapsed_ms(raw.accumulated_ms, raw.run_started_ms, now_ms);
    if timer::is_finished(elapsed, raw.planned_ms) {
        return complete(conn, id, now_ms);
    }
    if elapsed < timer::MIN_SAVE_MS {
        return Err(AppError::conflict(
            "Sessions shorter than a minute are not saved. Use Reset to discard it.",
        ));
    }
    conn.execute(
        "UPDATE focus_sessions SET state = 'stopped', run_started_at_ms = NULL, \
         accumulated_ms = ?1, ended_at_ms = ?2, last_seen_ms = ?2 \
         WHERE id = ?3 AND state IN ('running', 'paused')",
        params![elapsed, now_ms.max(raw.started_at_ms), id],
    )?;
    snapshot(conn, now_ms, Some(load_session(conn, id, now_ms)?))
}

/// Discards the active session without recording anything. Resetting a session that no longer
/// exists is a no-op; finished sessions are history and cannot be reset.
pub fn reset(conn: &Connection, id: i64, now_ms: i64) -> AppResult<TimerSnapshot> {
    reconcile(conn, now_ms)?;
    match find_raw(conn, id)? {
        None => {}
        Some(raw) if raw.state.is_active() => {
            conn.execute(
                "DELETE FROM focus_sessions WHERE id = ?1 AND state IN ('running', 'paused')",
                params![id],
            )?;
        }
        Some(_) => {
            return Err(AppError::conflict(
                "Finished sessions are part of your history and cannot be reset.",
            ));
        }
    }
    snapshot(conn, now_ms, None)
}

/// Finished focus sessions (newest first) plus today's totals. `today` is a local `YYYY-MM-DD`.
pub fn history(conn: &Connection, limit: u32, today: &str, now_ms: i64) -> AppResult<FocusHistory> {
    let limit = i64::from(limit.clamp(1, 200));
    let sql = format!(
        "{SELECT} WHERE f.kind = 'focus' AND f.state IN ('completed', 'stopped') \
         ORDER BY f.started_at_ms DESC, f.id DESC LIMIT ?1"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![limit], |r| map_session(r, now_ms))?;
    let sessions = rows.collect::<rusqlite::Result<Vec<FocusSession>>>()?;

    let (today_focus_ms, today_sessions): (i64, u32) = conn.query_row(
        "SELECT COALESCE(SUM(accumulated_ms), 0), COUNT(*) FROM focus_sessions \
         WHERE kind = 'focus' AND state IN ('completed', 'stopped') \
         AND date(started_at_ms / 1000, 'unixepoch', 'localtime') = ?1",
        params![today],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    Ok(FocusHistory {
        sessions,
        today_focus_ms,
        today_sessions,
    })
}
