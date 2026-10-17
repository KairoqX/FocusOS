use crate::error::{AppError, AppResult};
use crate::models::{Priority, TaskInput};

pub const TITLE_MAX: usize = 200;
pub const DESCRIPTION_MAX: usize = 5000;
pub const MAX_ESTIMATED_MINUTES: u32 = 1440;
pub const SUBJECT_NAME_MAX: usize = 100;
pub const CHAPTER_TITLE_MAX: usize = 200;

/// A task input that passed validation, with text normalized (trimmed).
#[derive(Debug, Clone, PartialEq)]
pub struct ValidTask {
    pub title: String,
    pub description: String,
    pub subject_id: Option<i64>,
    pub priority: Priority,
    pub due_date: Option<String>,
    pub estimated_minutes: Option<u32>,
}

pub fn validate_task(input: &TaskInput) -> AppResult<ValidTask> {
    let title = input.title.trim();
    if title.is_empty() {
        return Err(AppError::validation("title", "Title is required."));
    }
    if title.chars().count() > TITLE_MAX {
        return Err(AppError::validation(
            "title",
            format!("Title must be at most {TITLE_MAX} characters."),
        ));
    }

    let description = input.description.trim();
    if description.chars().count() > DESCRIPTION_MAX {
        return Err(AppError::validation(
            "description",
            format!("Description must be at most {DESCRIPTION_MAX} characters."),
        ));
    }

    if let Some(id) = input.subject_id {
        if id <= 0 {
            return Err(AppError::validation("subjectId", "Invalid subject."));
        }
    }

    let due_date = match input.due_date.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(d) if is_valid_date(d) => Some(d.to_owned()),
        Some(_) => {
            return Err(AppError::validation(
                "dueDate",
                "Due date must be a valid date (YYYY-MM-DD).",
            ))
        }
    };

    if let Some(m) = input.estimated_minutes {
        if m == 0 || m > MAX_ESTIMATED_MINUTES {
            return Err(AppError::validation(
                "estimatedMinutes",
                format!("Estimate must be between 1 and {MAX_ESTIMATED_MINUTES} minutes."),
            ));
        }
    }

    Ok(ValidTask {
        title: title.to_owned(),
        description: description.to_owned(),
        subject_id: input.subject_id,
        priority: input.priority,
        due_date,
        estimated_minutes: input.estimated_minutes,
    })
}

pub fn validate_subject_name(name: &str) -> AppResult<String> {
    required_text("name", "Subject name", name, SUBJECT_NAME_MAX)
}

pub fn validate_chapter_title(title: &str) -> AppResult<String> {
    required_text("title", "Chapter title", title, CHAPTER_TITLE_MAX)
}

fn required_text(field: &'static str, label: &str, value: &str, max: usize) -> AppResult<String> {
    let v = value.trim();
    if v.is_empty() {
        return Err(AppError::validation(field, format!("{label} is required.")));
    }
    if v.chars().count() > max {
        return Err(AppError::validation(
            field,
            format!("{label} must be at most {max} characters."),
        ));
    }
    Ok(v.to_owned())
}

/// Strict `YYYY-MM-DD` calendar-date check (year 1900..=9999, leap years honored).
pub fn is_valid_date(s: &str) -> bool {
    parse_date(s).is_some()
}

fn parse_date(s: &str) -> Option<(i32, u32, u32)> {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let digits_ok = b
        .iter()
        .enumerate()
        .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit());
    if !digits_ok {
        return None; // also guarantees ASCII, so the slices below are on char boundaries
    }
    let year: i32 = s[0..4].parse().ok()?;
    let month: u32 = s[5..7].parse().ok()?;
    let day: u32 = s[8..10].parse().ok()?;
    if year < 1900 || day == 0 || day > days_in_month(year, month) {
        return None;
    }
    Some((year, month, day))
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(title: &str) -> TaskInput {
        TaskInput {
            title: title.to_owned(),
            description: String::new(),
            subject_id: None,
            priority: Priority::Medium,
            due_date: None,
            estimated_minutes: None,
        }
    }

    fn field_of(r: AppResult<ValidTask>) -> &'static str {
        match r {
            Err(AppError::Validation { field, .. }) => field,
            other => panic!("expected validation error, got {other:?}"),
        }
    }

    #[test]
    fn dates() {
        assert!(is_valid_date("2024-02-29"));
        assert!(!is_valid_date("2023-02-29"));
        assert!(is_valid_date("2000-02-29"));
        assert!(!is_valid_date("1900-02-29"));
        assert!(!is_valid_date("2024-13-01"));
        assert!(!is_valid_date("2024-00-10"));
        assert!(!is_valid_date("2024-04-31"));
        assert!(!is_valid_date("2024-1-01"));
        assert!(!is_valid_date("24-01-01"));
        assert!(!is_valid_date("2024/01/01"));
        assert!(!is_valid_date("1899-12-31"));
        assert!(!is_valid_date("２０２４-01-01")); // full-width digits
        assert!(!is_valid_date(""));
    }

    #[test]
    fn title_is_trimmed_and_required() {
        assert_eq!(
            validate_task(&input("  Read ch. 3 ")).unwrap().title,
            "Read ch. 3"
        );
        assert_eq!(field_of(validate_task(&input("   "))), "title");
        assert_eq!(field_of(validate_task(&input(""))), "title");
    }

    #[test]
    fn title_length_counts_characters() {
        assert!(validate_task(&input(&"é".repeat(TITLE_MAX))).is_ok());
        assert_eq!(
            field_of(validate_task(&input(&"a".repeat(TITLE_MAX + 1)))),
            "title"
        );
    }

    #[test]
    fn description_limit() {
        let mut i = input("t");
        i.description = "x".repeat(DESCRIPTION_MAX);
        assert!(validate_task(&i).is_ok());
        i.description.push('x');
        assert_eq!(field_of(validate_task(&i)), "description");
    }

    #[test]
    fn due_date_rules() {
        let mut i = input("t");
        i.due_date = Some("  ".into());
        assert_eq!(validate_task(&i).unwrap().due_date, None);
        i.due_date = Some(" 2025-03-15 ".into());
        assert_eq!(
            validate_task(&i).unwrap().due_date.as_deref(),
            Some("2025-03-15")
        );
        i.due_date = Some("2025-02-30".into());
        assert_eq!(field_of(validate_task(&i)), "dueDate");
    }

    #[test]
    fn estimate_bounds() {
        let mut i = input("t");
        for ok in [1, 25, MAX_ESTIMATED_MINUTES] {
            i.estimated_minutes = Some(ok);
            assert!(validate_task(&i).is_ok());
        }
        for bad in [0, MAX_ESTIMATED_MINUTES + 1] {
            i.estimated_minutes = Some(bad);
            assert_eq!(field_of(validate_task(&i)), "estimatedMinutes");
        }
    }

    #[test]
    fn subject_and_chapter_names() {
        assert_eq!(validate_subject_name("  Physics ").unwrap(), "Physics");
        assert!(validate_subject_name("   ").is_err());
        assert!(validate_subject_name(&"a".repeat(SUBJECT_NAME_MAX)).is_ok());
        assert!(validate_subject_name(&"a".repeat(SUBJECT_NAME_MAX + 1)).is_err());
        assert_eq!(validate_chapter_title(" Optics\t").unwrap(), "Optics");
        assert!(matches!(
            validate_chapter_title(""),
            Err(AppError::Validation { field: "title", .. })
        ));
        assert!(validate_chapter_title(&"é".repeat(CHAPTER_TITLE_MAX + 1)).is_err());
    }

    #[test]
    fn subject_id_must_be_positive() {
        let mut i = input("t");
        i.subject_id = Some(0);
        assert_eq!(field_of(validate_task(&i)), "subjectId");
        i.subject_id = Some(3);
        assert!(validate_task(&i).is_ok());
    }
}
