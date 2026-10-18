//! Pure progress calculations (no database access) so they are trivially testable.
//!
//! Rules (the single source of truth, mirrored nowhere else):
//! * Chapter, automatic mode: completed linked tasks / linked tasks, whole percent, half up;
//!   0 when the chapter has no tasks.
//! * Chapter, manual mode: the stored percentage, ignoring tasks entirely.
//! * Subject: unweighted mean of its chapters' (already rounded) percentages, half up;
//!   0 when it has no chapters. Chapters without tasks therefore still count (as 0%).

use crate::models::ProgressMode;

/// Whole-number completion percentage, rounded half up.
/// Returns 0 when there is nothing to complete; `completed` is clamped to `total`.
pub fn percent_complete(completed: u32, total: u32) -> u8 {
    if total == 0 {
        return 0;
    }
    let completed = u64::from(completed.min(total));
    let total = u64::from(total);
    ((completed * 100 + total / 2) / total) as u8
}

/// Effective progress of one chapter.
pub fn chapter_progress(mode: ProgressMode, manual: u8, completed: u32, total: u32) -> u8 {
    match mode {
        ProgressMode::Manual => manual.min(100),
        ProgressMode::Automatic => percent_complete(completed, total),
    }
}

/// Unweighted mean of chapter percentages, rounded half up; 0 for an empty slice.
pub fn subject_progress(chapter_percents: &[u8]) -> u8 {
    if chapter_percents.is_empty() {
        return 0;
    }
    let n = chapter_percents.len() as u64;
    let sum: u64 = chapter_percents
        .iter()
        .map(|&p| u64::from(p.min(100)))
        .sum();
    ((sum + n / 2) / n) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_zero() {
        assert_eq!(percent_complete(0, 0), 0);
        assert_eq!(percent_complete(5, 0), 0);
    }

    #[test]
    fn rounds_half_up() {
        assert_eq!(percent_complete(1, 3), 33);
        assert_eq!(percent_complete(2, 3), 67);
        assert_eq!(percent_complete(1, 2), 50);
        assert_eq!(percent_complete(1, 8), 13); // 12.5 -> 13
        assert_eq!(percent_complete(1, 200), 1); // 0.5 -> 1
        assert_eq!(percent_complete(1, 201), 0); // just under 0.5
    }

    #[test]
    fn bounds() {
        assert_eq!(percent_complete(0, 10), 0);
        assert_eq!(percent_complete(10, 10), 100);
        assert_eq!(percent_complete(99, 10), 100); // clamped
        assert_eq!(percent_complete(u32::MAX, u32::MAX), 100);
    }

    #[test]
    fn chapter_automatic_uses_tasks_and_ignores_manual_value() {
        assert_eq!(chapter_progress(ProgressMode::Automatic, 90, 0, 0), 0); // no tasks
        assert_eq!(chapter_progress(ProgressMode::Automatic, 90, 0, 4), 0); // tasks, none done
        assert_eq!(chapter_progress(ProgressMode::Automatic, 90, 1, 4), 25);
        assert_eq!(chapter_progress(ProgressMode::Automatic, 0, 4, 4), 100);
    }

    #[test]
    fn chapter_manual_ignores_tasks() {
        assert_eq!(chapter_progress(ProgressMode::Manual, 70, 0, 0), 70);
        assert_eq!(chapter_progress(ProgressMode::Manual, 70, 4, 4), 70);
        assert_eq!(chapter_progress(ProgressMode::Manual, 0, 4, 4), 0);
        assert_eq!(chapter_progress(ProgressMode::Manual, 200, 0, 0), 100); // defensive clamp
    }

    #[test]
    fn subject_is_unweighted_mean() {
        assert_eq!(subject_progress(&[]), 0);
        assert_eq!(subject_progress(&[0]), 0);
        assert_eq!(subject_progress(&[100]), 100);
        assert_eq!(subject_progress(&[100, 0]), 50);
        assert_eq!(subject_progress(&[100, 0, 0]), 33);
        assert_eq!(subject_progress(&[100, 100, 0]), 67);
        assert_eq!(subject_progress(&[100, 0, 0, 50]), 38); // 37.5 -> 38
        assert_eq!(subject_progress(&[1, 0]), 1); // 0.5 -> 1
        assert_eq!(subject_progress(&[1, 0, 0]), 0);
    }
}
