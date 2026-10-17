use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ValueRef};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    Medium,
    High,
}

impl Priority {
    pub fn as_str(self) -> &'static str {
        match self {
            Priority::Low => "low",
            Priority::Medium => "medium",
            Priority::High => "high",
        }
    }
}

impl FromSql for Priority {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value.as_str()? {
            "low" => Ok(Priority::Low),
            "medium" => Ok(Priority::Medium),
            "high" => Ok(Priority::High),
            other => Err(FromSqlError::Other(
                format!("unknown priority '{other}'").into(),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub subject_id: Option<i64>,
    pub subject_name: Option<String>,
    pub priority: Priority,
    /// `YYYY-MM-DD`
    pub due_date: Option<String>,
    pub estimated_minutes: Option<u32>,
    pub completed: bool,
    /// UTC ISO-8601 timestamp, present only when completed.
    pub completed_at: Option<String>,
    /// True when the task is open and its due date is before today (local date).
    pub overdue: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Payload for creating or updating a task. Validated in `validation::validate_task`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskInput {
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub subject_id: Option<i64>,
    pub priority: Priority,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default)]
    pub estimated_minutes: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskView {
    /// Open tasks due today or earlier (overdue included).
    Today,
    /// Open tasks due after today.
    Upcoming,
    Completed,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskSort {
    DueDate,
    Priority,
    Created,
    Title,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskQuery {
    pub view: TaskView,
    pub sort: TaskSort,
    #[serde(default)]
    pub subject_id: Option<i64>,
    #[serde(default)]
    pub priority: Option<Priority>,
    #[serde(default)]
    pub search: Option<String>,
}

impl TaskQuery {
    pub fn for_view(view: TaskView) -> Self {
        TaskQuery {
            view,
            sort: TaskSort::DueDate,
            subject_id: None,
            priority: None,
            search: None,
        }
    }
}

/// A subject with live aggregates computed from the chapters and tasks tables.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Subject {
    pub id: i64,
    pub name: String,
    pub chapter_count: u32,
    pub not_started: u32,
    pub in_progress: u32,
    pub completed: u32,
    pub needs_revision: u32,
    /// Completed chapters / all chapters, rounded to a whole percent (0 when there are none).
    pub percent_complete: u8,
    /// All tasks linked to this subject.
    pub task_count: u32,
    /// Linked tasks that are not completed.
    pub open_tasks: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubjectInput {
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChapterStatus {
    NotStarted,
    InProgress,
    Completed,
    NeedsRevision,
}

impl ChapterStatus {
    /// The value stored in SQLite (matches the CHECK constraint in migration 0002).
    pub fn as_str(self) -> &'static str {
        match self {
            ChapterStatus::NotStarted => "not_started",
            ChapterStatus::InProgress => "in_progress",
            ChapterStatus::Completed => "completed",
            ChapterStatus::NeedsRevision => "needs_revision",
        }
    }
}

impl FromSql for ChapterStatus {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value.as_str()? {
            "not_started" => Ok(ChapterStatus::NotStarted),
            "in_progress" => Ok(ChapterStatus::InProgress),
            "completed" => Ok(ChapterStatus::Completed),
            "needs_revision" => Ok(ChapterStatus::NeedsRevision),
            other => Err(FromSqlError::Other(
                format!("unknown chapter status '{other}'").into(),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Chapter {
    pub id: i64,
    pub subject_id: i64,
    pub title: String,
    pub status: ChapterStatus,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChapterInput {
    pub title: String,
}
