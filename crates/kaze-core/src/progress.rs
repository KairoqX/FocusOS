//! Pure progress calculations (no database access) so they are trivially testable.

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

#[cfg(test)]
mod tests {
    use super::percent_complete;

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
}
