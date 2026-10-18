//! Focus timer behaviour over (simulated) time. Every call receives an explicit `now_ms`, so
//! pausing, restarts and sleep are tested by moving the clock, not by waiting.
use kaze_core::db;
use kaze_core::error::AppError;
use kaze_core::focus;
use kaze_core::models::{
    FocusSession, SessionKind, SessionState, StartSessionInput, TimerSettings, TimerSnapshot,
};
use kaze_core::settings;
use kaze_core::timer::HEARTBEAT_GAP_MS;
use rusqlite::Connection;
use std::path::PathBuf;

const T0: i64 = 1_700_049_600_000; // 2023-11-15 12:00:00 UTC
const MIN: i64 = 60_000;
const FOCUS: i64 = 25 * MIN; // default planned duration

fn conn() -> Connection {
    db::open_in_memory().unwrap()
}

fn input(kind: SessionKind, task_id: Option<i64>) -> StartSessionInput {
    StartSessionInput { kind, task_id }
}

fn start_focus(c: &Connection, now: i64) -> TimerSnapshot {
    focus::start(c, &input(SessionKind::Focus, None), now).unwrap()
}

fn active(s: &TimerSnapshot) -> &FocusSession {
    s.active.as_ref().expect("expected an active session")
}

/// Heartbeats once a minute from `from` to `to`, like the UI does while the app is open.
fn tick_to(c: &Connection, from: i64, to: i64) {
    let mut t = from;
    while t < to {
        t = (t + MIN).min(to);
        focus::heartbeat(c, t).unwrap();
    }
}

fn rows(c: &Connection) -> i64 {
    c.query_row("SELECT COUNT(*) FROM focus_sessions", [], |r| r.get(0))
        .unwrap()
}

fn conflict_message<T: std::fmt::Debug>(r: Result<T, AppError>) -> String {
    match r {
        Err(AppError::Conflict { message }) => message,
        other => panic!("expected conflict, got {other:?}"),
    }
}

fn today_of(c: &Connection, ms: i64) -> String {
    c.query_row(
        "SELECT date(?1 / 1000, 'unixepoch', 'localtime')",
        [ms],
        |r| r.get(0),
    )
    .unwrap()
}

fn history(c: &Connection) -> kaze_core::models::FocusHistory {
    focus::history(c, 50, &today_of(c, T0), T0 + 365 * 24 * 60 * MIN).unwrap()
}

// ------------------------------------------------------------------ settings

#[test]
fn timer_settings_default_update_and_validate() {
    let c = conn();
    assert_eq!(settings::get_timer(&c).unwrap(), settings::defaults());

    let new = TimerSettings {
        focus_minutes: 50,
        break_minutes: 10,
    };
    assert_eq!(settings::set_timer(&c, &new).unwrap(), new);
    assert_eq!(settings::get_timer(&c).unwrap(), new);

    for (focus_minutes, break_minutes, field) in [
        (0, 5, "focusMinutes"),
        (241, 5, "focusMinutes"),
        (25, 0, "breakMinutes"),
        (25, 61, "breakMinutes"),
    ] {
        match settings::set_timer(
            &c,
            &TimerSettings {
                focus_minutes,
                break_minutes,
            },
        ) {
            Err(AppError::Validation { field: f, .. }) => assert_eq!(f, field),
            other => panic!("expected validation error, got {other:?}"),
        }
    }
    // A rejected update must not half-apply (valid focus + invalid break).
    assert_eq!(settings::get_timer(&c).unwrap(), new);
}

#[test]
fn unreadable_setting_values_fall_back_to_defaults() {
    let c = conn();
    settings::set_timer(
        &c,
        &TimerSettings {
            focus_minutes: 40,
            break_minutes: 8,
        },
    )
    .unwrap();
    c.execute(
        "UPDATE app_settings SET value = 'abc' WHERE key = 'timer.focus_minutes'",
        [],
    )
    .unwrap();
    c.execute(
        "UPDATE app_settings SET value = '9999' WHERE key = 'timer.break_minutes'",
        [],
    )
    .unwrap();
    assert_eq!(settings::get_timer(&c).unwrap(), settings::defaults());
}

// ------------------------------------------------------------------ time model

#[test]
fn start_uses_configured_duration_and_time_is_derived_from_the_clock() {
    let c = conn();
    settings::set_timer(
        &c,
        &TimerSettings {
            focus_minutes: 50,
            break_minutes: 5,
        },
    )
    .unwrap();
    let s = start_focus(&c, T0);
    let a = active(&s);
    assert_eq!(
        (a.kind, a.state, a.planned_ms),
        (SessionKind::Focus, SessionState::Running, 50 * MIN)
    );
    assert_eq!((a.elapsed_ms, a.remaining_ms), (0, 50 * MIN));

    // No ticking happened in between; the values come purely from the stored timestamps.
    let s = focus::status(&c, T0 + 2 * MIN).unwrap();
    assert_eq!(
        (active(&s).elapsed_ms, active(&s).remaining_ms),
        (2 * MIN, 48 * MIN)
    );
}

#[test]
fn pausing_stops_the_clock_and_resuming_continues_from_banked_time() {
    let c = conn();
    let id = active(&start_focus(&c, T0)).id;
    tick_to(&c, T0, T0 + 5 * MIN);
    let s = focus::pause(&c, id, T0 + 5 * MIN).unwrap();
    assert_eq!(
        (active(&s).state, active(&s).elapsed_ms),
        (SessionState::Paused, 5 * MIN)
    );

    // Hours pass while paused: nothing changes and the pause is not an "interruption".
    let s = focus::status(&c, T0 + 3 * 60 * MIN).unwrap();
    assert_eq!(
        (
            active(&s).state,
            active(&s).elapsed_ms,
            active(&s).auto_paused
        ),
        (SessionState::Paused, 5 * MIN, false)
    );

    let resume_at = T0 + 3 * 60 * MIN;
    let s = focus::resume(&c, id, resume_at).unwrap();
    assert_eq!(active(&s).state, SessionState::Running);
    tick_to(&c, resume_at, resume_at + 2 * MIN);
    let s = focus::status(&c, resume_at + 2 * MIN).unwrap();
    assert_eq!(active(&s).elapsed_ms, 7 * MIN);
}

#[test]
fn pause_and_resume_are_idempotent() {
    let c = conn();
    let id = active(&start_focus(&c, T0)).id;
    focus::pause(&c, id, T0 + MIN).unwrap();
    let again = focus::pause(&c, id, T0 + 2 * MIN).unwrap();
    assert_eq!(
        active(&again).elapsed_ms,
        MIN,
        "second pause must not change banked time"
    );
    focus::resume(&c, id, T0 + 3 * MIN).unwrap();
    let again = focus::resume(&c, id, T0 + 4 * MIN).unwrap();
    assert_eq!(active(&again).state, SessionState::Running);
    assert_eq!(
        active(&again).elapsed_ms,
        MIN + MIN,
        "second resume must not restart the segment"
    );
}

#[test]
fn a_backwards_clock_never_produces_negative_time_or_a_false_interruption() {
    let c = conn();
    start_focus(&c, T0);
    let s = focus::status(&c, T0 - 5 * MIN).unwrap();
    assert_eq!(
        (
            active(&s).state,
            active(&s).elapsed_ms,
            active(&s).remaining_ms
        ),
        (SessionState::Running, 0, FOCUS)
    );
}

// ------------------------------------------------------------------ single active session

#[test]
fn only_one_session_can_be_active() {
    let c = conn();
    let id = active(&start_focus(&c, T0)).id;
    let msg = conflict_message(focus::start(
        &c,
        &input(SessionKind::Focus, None),
        T0 + 1_000,
    ));
    assert!(msg.contains("already in progress"));
    conflict_message(focus::start(
        &c,
        &input(SessionKind::Break, None),
        T0 + 1_000,
    ));
    assert_eq!(rows(&c), 1);

    // The database enforces it too, independent of the Rust checks.
    let raw = c.execute(
        "INSERT INTO focus_sessions (kind, state, planned_seconds, started_at_ms, run_started_at_ms, last_seen_ms) \
         VALUES ('focus', 'running', 1500, 1, 1, 1)",
        [],
    );
    assert!(raw.is_err());

    focus::reset(&c, id, T0 + 2_000).unwrap();
    assert!(focus::start(&c, &input(SessionKind::Focus, None), T0 + 3_000).is_ok());
}

// ------------------------------------------------------------------ completion

#[test]
fn completion_is_validated_against_elapsed_time_and_is_idempotent() {
    let c = conn();
    let id = active(&start_focus(&c, T0)).id;
    tick_to(&c, T0, T0 + 10 * MIN);

    // Too early: rejected, nothing recorded, still running.
    assert!(conflict_message(focus::complete(&c, id, T0 + 10 * MIN)).contains("not finished"));
    assert_eq!(history(&c).sessions.len(), 0);
    assert_eq!(
        active(&focus::status(&c, T0 + 10 * MIN).unwrap()).state,
        SessionState::Running
    );

    tick_to(&c, T0 + 10 * MIN, T0 + 25 * MIN);
    let done = focus::complete(&c, id, T0 + 25 * MIN).unwrap();
    assert!(done.active.is_none());
    let f = done.finished.expect("finished session");
    assert_eq!(
        (f.state, f.elapsed_ms, f.remaining_ms, f.ended_at_ms),
        (SessionState::Completed, FOCUS, 0, Some(T0 + FOCUS))
    );

    // Duplicate completion (retry, double timer) returns the same record and writes nothing new.
    let again = focus::complete(&c, id, T0 + 26 * MIN).unwrap();
    assert_eq!(
        again.finished.expect("same session").ended_at_ms,
        Some(T0 + FOCUS)
    );
    assert_eq!(rows(&c), 1);
    assert_eq!(history(&c).sessions.len(), 1);
}

#[test]
fn completion_accepts_a_tiny_early_call() {
    let c = conn();
    let id = active(&start_focus(&c, T0)).id;
    tick_to(&c, T0, T0 + 24 * MIN);
    let done = focus::complete(&c, id, T0 + FOCUS - 500).unwrap();
    let f = done.finished.unwrap();
    assert_eq!(
        (f.state, f.ended_at_ms),
        (SessionState::Completed, Some(T0 + FOCUS - 500)),
        "never ends in the future"
    );
}

#[test]
fn a_late_completion_call_still_ends_at_the_planned_time() {
    let c = conn();
    let id = active(&start_focus(&c, T0)).id;
    tick_to(&c, T0, T0 + 24 * MIN);
    // The UI timer was throttled; the call arrives 2 minutes late but the app was alive.
    let s = focus::status(&c, T0 + 26 * MIN).unwrap();
    assert_eq!(
        (active(&s).state, active(&s).remaining_ms),
        (SessionState::Running, 0)
    );
    let f = focus::complete(&c, id, T0 + 26 * MIN)
        .unwrap()
        .finished
        .unwrap();
    assert_eq!((f.elapsed_ms, f.ended_at_ms), (FOCUS, Some(T0 + FOCUS)));
}

// ------------------------------------------------------------------ restarts and sleep

struct TempDb(PathBuf);
impl Drop for TempDb {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
fn temp_db(tag: &str) -> TempDb {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    TempDb(std::env::temp_dir().join(format!(
        "kaze-focus-{tag}-{}-{nanos}.db",
        std::process::id()
    )))
}

#[test]
fn a_quick_restart_keeps_the_session_running() {
    let file = temp_db("quick");
    let id = {
        let c = db::open(&file.0).unwrap();
        active(&start_focus(&c, T0)).id
    }; // app closed
    let c = db::open(&file.0).unwrap(); // app reopened a minute later
    let s = focus::status(&c, T0 + MIN).unwrap();
    let a = active(&s);
    assert_eq!(
        (a.id, a.state, a.elapsed_ms, a.auto_paused),
        (id, SessionState::Running, MIN, false)
    );
}

#[test]
fn a_long_absence_pauses_the_session_and_credits_only_observed_time() {
    let file = temp_db("sleep");
    let id = {
        let c = db::open(&file.0).unwrap();
        let id = active(&start_focus(&c, T0)).id;
        tick_to(&c, T0, T0 + 2 * MIN); // last heartbeat at +2 min, then the app/computer went away
        id
    };
    let c = db::open(&file.0).unwrap();
    let back = T0 + 4 * 60 * MIN; // four hours later
    let s = focus::status(&c, back).unwrap();
    let a = active(&s);
    assert_eq!(
        (a.id, a.state, a.elapsed_ms, a.auto_paused),
        (id, SessionState::Paused, 2 * MIN, true)
    );

    // Repeated checks and heartbeats change nothing (no silent resume, no extra credit).
    focus::heartbeat(&c, back + 30_000).unwrap();
    let s = focus::status(&c, back + 5 * MIN).unwrap();
    assert_eq!(
        (active(&s).state, active(&s).elapsed_ms),
        (SessionState::Paused, 2 * MIN)
    );

    // The user decides to continue: the flag clears and counting restarts from "now".
    let s = focus::resume(&c, id, back + 5 * MIN).unwrap();
    assert_eq!(
        (active(&s).state, active(&s).auto_paused),
        (SessionState::Running, false)
    );
    tick_to(&c, back + 5 * MIN, back + 8 * MIN);
    assert_eq!(
        active(&focus::status(&c, back + 8 * MIN).unwrap()).elapsed_ms,
        5 * MIN
    );
}

#[test]
fn the_interruption_threshold_is_exact() {
    let c = conn();
    start_focus(&c, T0);
    let s = focus::status(&c, T0 + HEARTBEAT_GAP_MS).unwrap();
    assert_eq!(
        active(&s).state,
        SessionState::Running,
        "exactly at the gap is still fine"
    );
    let s = focus::status(&c, T0 + HEARTBEAT_GAP_MS + 1).unwrap();
    assert_eq!(
        (
            active(&s).state,
            active(&s).elapsed_ms,
            active(&s).auto_paused
        ),
        (SessionState::Paused, 0, true)
    );
}

#[test]
fn a_session_that_would_have_finished_while_away_is_not_credited_in_full() {
    let c = conn();
    let id = active(&start_focus(&c, T0)).id;
    tick_to(&c, T0, 10 * MIN + T0);
    // Computer sleeps for two hours; the planned 25 minutes "passed" on the wall clock.
    let s = focus::status(&c, T0 + 130 * MIN).unwrap();
    assert_eq!(
        (active(&s).state, active(&s).elapsed_ms),
        (SessionState::Paused, 10 * MIN)
    );
    assert!(conflict_message(focus::complete(&c, id, T0 + 130 * MIN)).contains("not finished"));
}

// ------------------------------------------------------------------ stop & reset

#[test]
fn stopping_early_saves_real_elapsed_time_but_not_tiny_sessions() {
    let c = conn();
    let id = active(&start_focus(&c, T0)).id;
    tick_to(&c, T0, T0 + 30_000);
    assert!(conflict_message(focus::stop(&c, id, T0 + 30_000)).contains("shorter than a minute"));
    assert!(focus::status(&c, T0 + 30_000).unwrap().active.is_some());

    tick_to(&c, T0 + 30_000, T0 + 10 * MIN);
    let f = focus::stop(&c, id, T0 + 10 * MIN)
        .unwrap()
        .finished
        .unwrap();
    assert_eq!(
        (f.state, f.elapsed_ms, f.ended_at_ms),
        (SessionState::Stopped, 10 * MIN, Some(T0 + 10 * MIN))
    );
    assert!(focus::status(&c, T0 + 10 * MIN).unwrap().active.is_none());

    // Idempotent, and it counts as focus time.
    focus::stop(&c, id, T0 + 11 * MIN).unwrap();
    assert_eq!(rows(&c), 1);
    assert_eq!(history(&c).sessions[0].elapsed_ms, 10 * MIN);
}

#[test]
fn stopping_after_the_planned_time_completes_the_session() {
    let c = conn();
    let id = active(&start_focus(&c, T0)).id;
    tick_to(&c, T0, T0 + FOCUS);
    let f = focus::stop(&c, id, T0 + FOCUS).unwrap().finished.unwrap();
    assert_eq!(f.state, SessionState::Completed);
}

#[test]
fn reset_discards_the_session_without_recording_it() {
    let c = conn();
    let id = active(&start_focus(&c, T0)).id;
    tick_to(&c, T0, T0 + 5 * MIN);
    let s = focus::reset(&c, id, T0 + 5 * MIN).unwrap();
    assert!(s.active.is_none() && s.finished.is_none());
    assert_eq!(rows(&c), 0);
    assert!(
        focus::reset(&c, id, T0 + 6 * MIN).is_ok(),
        "resetting twice is harmless"
    );
    assert!(history(&c).sessions.is_empty());
}

#[test]
fn finished_sessions_cannot_be_reset_or_changed() {
    let c = conn();
    let id = active(&start_focus(&c, T0)).id;
    tick_to(&c, T0, T0 + FOCUS);
    focus::complete(&c, id, T0 + FOCUS).unwrap();
    conflict_message(focus::reset(&c, id, T0 + FOCUS));
    conflict_message(focus::pause(&c, id, T0 + FOCUS));
    conflict_message(focus::resume(&c, id, T0 + FOCUS));
    assert_eq!(rows(&c), 1);
}

// ------------------------------------------------------------------ breaks and task links

#[test]
fn breaks_use_their_own_duration_and_never_count_as_focus() {
    let c = conn();
    settings::set_timer(
        &c,
        &TimerSettings {
            focus_minutes: 25,
            break_minutes: 7,
        },
    )
    .unwrap();
    c.execute("INSERT INTO tasks (title) VALUES ('t')", [])
        .unwrap();
    // A task id on a break is ignored.
    let s = focus::start(&c, &input(SessionKind::Break, Some(1)), T0).unwrap();
    let a = active(&s);
    assert_eq!(
        (a.kind, a.planned_ms, a.task_id),
        (SessionKind::Break, 7 * MIN, None)
    );
    let id = a.id;
    tick_to(&c, T0, T0 + 7 * MIN);
    assert_eq!(
        focus::complete(&c, id, T0 + 7 * MIN)
            .unwrap()
            .finished
            .unwrap()
            .kind,
        SessionKind::Break
    );
    let h = history(&c);
    assert!(h.sessions.is_empty());
    assert_eq!((h.today_focus_ms, h.today_sessions), (0, 0));
}

#[test]
fn sessions_can_link_to_an_open_task_and_survive_its_deletion() {
    let c = conn();
    c.execute("INSERT INTO subjects (name) VALUES ('Physics')", [])
        .unwrap();
    c.execute(
        "INSERT INTO tasks (title, subject_id) VALUES ('Read optics', 1)",
        [],
    )
    .unwrap();
    c.execute(
        "INSERT INTO tasks (title, completed_at) VALUES ('Old', '2025-01-01T00:00:00Z')",
        [],
    )
    .unwrap();

    match focus::start(&c, &input(SessionKind::Focus, Some(2)), T0) {
        Err(AppError::Validation {
            field: "taskId", ..
        }) => {}
        other => panic!("completed task must be rejected, got {other:?}"),
    }
    assert!(matches!(
        focus::start(&c, &input(SessionKind::Focus, Some(99)), T0),
        Err(AppError::NotFound {
            entity: "task",
            id: 99
        })
    ));
    assert_eq!(rows(&c), 0, "failed starts leave nothing behind");

    let s = focus::start(&c, &input(SessionKind::Focus, Some(1)), T0).unwrap();
    let a = active(&s);
    assert_eq!(
        (a.task_title.as_deref(), a.subject_name.as_deref()),
        (Some("Read optics"), Some("Physics"))
    );

    c.execute("DELETE FROM tasks WHERE id = 1", []).unwrap();
    let s = focus::status(&c, T0 + MIN).unwrap();
    assert_eq!(
        (active(&s).task_id, active(&s).task_title.clone()),
        (None, None)
    );
}

// ------------------------------------------------------------------ history

#[test]
fn history_is_newest_first_limited_and_totals_cover_today_only() {
    let c = conn();
    let ins =
        "INSERT INTO focus_sessions (kind, state, planned_seconds, started_at_ms, last_seen_ms, \
               ended_at_ms, accumulated_ms) VALUES (?1, 'completed', 1500, ?2, ?2, ?2 + 1, ?3)";
    c.execute(ins, rusqlite::params!["focus", T0, 25 * MIN])
        .unwrap();
    c.execute(ins, rusqlite::params!["focus", T0, 10 * MIN])
        .unwrap();
    c.execute(ins, rusqlite::params!["break", T0, 5 * MIN])
        .unwrap();
    c.execute(
        ins,
        rusqlite::params!["focus", T0 - 3 * 24 * 60 * MIN, 20 * MIN],
    )
    .unwrap();

    let h = history(&c);
    assert_eq!(h.sessions.len(), 3, "breaks are not history");
    assert!(
        h.sessions[2].started_at_ms < h.sessions[0].started_at_ms,
        "newest first"
    );
    assert_eq!((h.today_focus_ms, h.today_sessions), (35 * MIN, 2));

    let limited = focus::history(&c, 1, &today_of(&c, T0), T0).unwrap();
    assert_eq!(limited.sessions.len(), 1);
    assert_eq!(limited.today_sessions, 2, "totals ignore the list limit");
}

#[test]
fn old_clients_cannot_corrupt_state_through_raw_sql() {
    let c = conn();
    let bad = [
        "INSERT INTO focus_sessions (kind, state, planned_seconds, started_at_ms, last_seen_ms) VALUES ('focus', 'running', 1500, 1, 1)", // running without run_started
        "INSERT INTO focus_sessions (kind, state, planned_seconds, started_at_ms, last_seen_ms) VALUES ('focus', 'completed', 1500, 1, 1)", // completed without ended_at
        "INSERT INTO focus_sessions (kind, state, planned_seconds, started_at_ms, last_seen_ms) VALUES ('nap', 'paused', 1500, 1, 1)",
        "INSERT INTO focus_sessions (kind, state, planned_seconds, started_at_ms, last_seen_ms) VALUES ('focus', 'paused', 5, 1, 1)",
    ];
    for sql in bad {
        assert!(
            c.execute(sql, []).is_err(),
            "should have been rejected: {sql}"
        );
    }
}
