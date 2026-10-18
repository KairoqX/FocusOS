//! Pure time arithmetic for focus sessions (no I/O), so every rule is unit-testable.
//!
//! A session stores `accumulated_ms` (time from finished running segments) and, while running,
//! `run_started_ms` (when the current segment began). Elapsed time is derived from those and the
//! current wall-clock time; nothing here counts ticks.

/// If a running session has not been "seen" (heartbeat) for longer than this, the app was closed
/// or the computer slept, and the unobserved time is not credited.
pub const HEARTBEAT_GAP_MS: i64 = 180_000;
/// Completion is accepted this much before the planned end, to absorb clock/timer jitter.
pub const COMPLETION_TOLERANCE_MS: i64 = 1_000;
/// Stopping a focus session early only saves it when at least this much time elapsed.
pub const MIN_SAVE_MS: i64 = 60_000;

/// Unclamped elapsed time. A negative running delta (system clock moved backwards) counts as 0.
pub fn raw_elapsed_ms(accumulated_ms: i64, run_started_ms: Option<i64>, now_ms: i64) -> i64 {
    let running = run_started_ms.map_or(0, |start| now_ms.saturating_sub(start).max(0));
    accumulated_ms.max(0).saturating_add(running)
}

/// Elapsed time, capped at the planned duration.
pub fn elapsed_ms(
    accumulated_ms: i64,
    run_started_ms: Option<i64>,
    planned_ms: i64,
    now_ms: i64,
) -> i64 {
    raw_elapsed_ms(accumulated_ms, run_started_ms, now_ms).min(planned_ms)
}

pub fn remaining_ms(
    accumulated_ms: i64,
    run_started_ms: Option<i64>,
    planned_ms: i64,
    now_ms: i64,
) -> i64 {
    planned_ms - elapsed_ms(accumulated_ms, run_started_ms, planned_ms, now_ms)
}

/// True when the planned duration has been reached (within the tolerance).
pub fn is_finished(raw_elapsed_ms: i64, planned_ms: i64) -> bool {
    raw_elapsed_ms.saturating_add(COMPLETION_TOLERANCE_MS) >= planned_ms
}

/// For a *running* session: `Some(new_accumulated_ms)` if it was interrupted (no heartbeat for
/// longer than [`HEARTBEAT_GAP_MS`]), crediting only time up to the last heartbeat.
pub fn interrupted_accumulated(
    accumulated_ms: i64,
    run_started_ms: i64,
    last_seen_ms: i64,
    now_ms: i64,
) -> Option<i64> {
    if now_ms.saturating_sub(last_seen_ms) > HEARTBEAT_GAP_MS {
        let observed = last_seen_ms.saturating_sub(run_started_ms).max(0);
        Some(accumulated_ms.max(0).saturating_add(observed))
    } else {
        None
    }
}

/// When a session that is being completed really reached its planned end (never in the future,
/// so a late completion call does not stretch the session).
pub fn true_end_ms(
    run_started_ms: Option<i64>,
    accumulated_ms: i64,
    planned_ms: i64,
    now_ms: i64,
) -> i64 {
    match run_started_ms {
        Some(start) => start
            .saturating_add((planned_ms - accumulated_ms.max(0)).max(0))
            .min(now_ms),
        None => now_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: i64 = 60_000;

    #[test]
    fn elapsed_while_running_and_paused() {
        // Running since t=1000 with 5 min already banked.
        assert_eq!(
            raw_elapsed_ms(5 * MIN, Some(1_000), 1_000 + 2 * MIN),
            7 * MIN
        );
        // Paused: wall-clock time passing changes nothing.
        assert_eq!(raw_elapsed_ms(5 * MIN, None, 99 * MIN), 5 * MIN);
    }

    #[test]
    fn elapsed_is_capped_and_remaining_never_negative() {
        let planned = 25 * MIN;
        assert_eq!(elapsed_ms(0, Some(0), planned, 40 * MIN), planned);
        assert_eq!(remaining_ms(0, Some(0), planned, 40 * MIN), 0);
        assert_eq!(remaining_ms(0, Some(0), planned, 10 * MIN), 15 * MIN);
        assert_eq!(remaining_ms(0, None, planned, 10 * MIN), planned);
    }

    #[test]
    fn clock_moving_backwards_never_yields_negative_time() {
        assert_eq!(raw_elapsed_ms(2 * MIN, Some(10 * MIN), 5 * MIN), 2 * MIN);
        assert_eq!(raw_elapsed_ms(-5, None, 0), 0);
    }

    #[test]
    fn finish_detection_has_a_small_tolerance() {
        let planned = 25 * MIN;
        assert!(is_finished(planned, planned));
        assert!(is_finished(planned - COMPLETION_TOLERANCE_MS, planned));
        assert!(!is_finished(planned - COMPLETION_TOLERANCE_MS - 1, planned));
    }

    #[test]
    fn interruption_credits_only_observed_time() {
        // Started at 0, last heartbeat at 2 min, now 3 h later.
        let credited = interrupted_accumulated(0, 0, 2 * MIN, 180 * MIN);
        assert_eq!(credited, Some(2 * MIN));
        // Earlier segments are kept.
        assert_eq!(
            interrupted_accumulated(10 * MIN, 50 * MIN, 52 * MIN, 500 * MIN),
            Some(12 * MIN)
        );
    }

    #[test]
    fn interruption_boundary_and_clock_skew() {
        let last_seen = 1_000_000;
        assert_eq!(
            interrupted_accumulated(0, 0, last_seen, last_seen + HEARTBEAT_GAP_MS),
            None
        );
        assert!(
            interrupted_accumulated(0, 0, last_seen, last_seen + HEARTBEAT_GAP_MS + 1).is_some()
        );
        // Clock moved backwards: never an interruption, never negative credit.
        assert_eq!(
            interrupted_accumulated(0, 0, last_seen, last_seen - 10 * MIN),
            None
        );
        assert_eq!(
            interrupted_accumulated(0, 500, 100, 100 + HEARTBEAT_GAP_MS + 1),
            Some(0)
        );
    }

    #[test]
    fn true_end_is_the_planned_end_not_the_late_call_time() {
        let planned = 25 * MIN;
        // Running since 0, completion call arrives 3 min late.
        assert_eq!(true_end_ms(Some(0), 0, planned, 28 * MIN), planned);
        // Segment started after 10 min were banked.
        assert_eq!(
            true_end_ms(Some(100 * MIN), 10 * MIN, planned, 200 * MIN),
            115 * MIN
        );
        // Completing a hair early (inside tolerance) cannot end in the future.
        assert_eq!(
            true_end_ms(Some(0), 0, planned, planned - 500),
            planned - 500
        );
        // Paused sessions end "now".
        assert_eq!(true_end_ms(None, planned, planned, 77), 77);
    }
}
