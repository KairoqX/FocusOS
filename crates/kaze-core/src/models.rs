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
    pub chapter_id: Option<i64>,
    pub chapter_title: Option<String>,
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
    /// Optional; requires `subject_id` and must belong to that subject.
    #[serde(default)]
    pub chapter_id: Option<i64>,
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

/// A subject with live aggregates derived from its chapters and tasks. Nothing here is stored.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Subject {
    pub id: i64,
    pub name: String,
    pub chapter_count: u32,
    /// Chapter counts by the user's own status label.
    pub not_started: u32,
    pub in_progress: u32,
    pub completed: u32,
    pub needs_revision: u32,
    /// Automatic-mode chapters that have no linked tasks yet.
    pub chapters_without_tasks: u32,
    /// Unweighted mean of the chapters' progress percentages (0 when there are no chapters).
    pub percent_complete: u8,
    /// All tasks linked to this subject (with or without a chapter).
    pub task_count: u32,
    /// Linked tasks that are not completed.
    pub open_tasks: u32,
    /// Linked tasks with no chapter; they do not contribute to any chapter's progress.
    pub unassigned_tasks: u32,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProgressMode {
    /// Progress = completed linked tasks / linked tasks.
    Automatic,
    /// Progress = the percentage the user set; ignores task completion.
    Manual,
}

impl ProgressMode {
    pub fn as_str(self) -> &'static str {
        match self {
            ProgressMode::Automatic => "automatic",
            ProgressMode::Manual => "manual",
        }
    }
}

impl FromSql for ProgressMode {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value.as_str()? {
            "automatic" => Ok(ProgressMode::Automatic),
            "manual" => Ok(ProgressMode::Manual),
            other => Err(FromSqlError::Other(
                format!("unknown progress mode '{other}'").into(),
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
    /// The user's own label; independent of progress.
    pub status: ChapterStatus,
    pub progress_mode: ProgressMode,
    /// Stored manual percentage (0-100); only used when `progress_mode` is manual.
    pub manual_progress: u8,
    pub task_total: u32,
    pub task_completed: u32,
    /// Effective progress, derived on every read: manual value or completed/total tasks.
    pub progress_percent: u8,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChapterInput {
    pub title: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterProgressInput {
    pub mode: ProgressMode,
    /// Optional. When omitted the previously stored manual percentage is kept.
    #[serde(default)]
    pub manual_progress: Option<i64>,
}

// ---------------------------------------------------------------------------------------------
// Focus timer
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SessionKind {
    Focus,
    Break,
}

impl SessionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SessionKind::Focus => "focus",
            SessionKind::Break => "break",
        }
    }
}

impl FromSql for SessionKind {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value.as_str()? {
            "focus" => Ok(SessionKind::Focus),
            "break" => Ok(SessionKind::Break),
            other => Err(FromSqlError::Other(
                format!("unknown session kind '{other}'").into(),
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SessionState {
    Running,
    Paused,
    Completed,
    Stopped,
}

impl SessionState {
    /// Running or paused: the (single) session the timer is currently tracking.
    pub fn is_active(self) -> bool {
        matches!(self, SessionState::Running | SessionState::Paused)
    }
}

impl FromSql for SessionState {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value.as_str()? {
            "running" => Ok(SessionState::Running),
            "paused" => Ok(SessionState::Paused),
            "completed" => Ok(SessionState::Completed),
            "stopped" => Ok(SessionState::Stopped),
            other => Err(FromSqlError::Other(
                format!("unknown session state '{other}'").into(),
            )),
        }
    }
}

/// A focus or break session. `elapsed_ms`/`remaining_ms` are derived at `TimerSnapshot::now_ms`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusSession {
    pub id: i64,
    pub kind: SessionKind,
    pub state: SessionState,
    pub planned_ms: i64,
    pub elapsed_ms: i64,
    /// Always 0 for finished sessions.
    pub remaining_ms: i64,
    /// UTC epoch milliseconds.
    pub started_at_ms: i64,
    pub ended_at_ms: Option<i64>,
    /// True when the session was paused automatically because the app was closed or the
    /// computer slept; time away is not counted.
    pub auto_paused: bool,
    pub task_id: Option<i64>,
    pub task_title: Option<String>,
    pub subject_id: Option<i64>,
    pub subject_name: Option<String>,
}

/// What every timer command returns, so the UI never has to guess the current state.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimerSnapshot {
    /// The running or paused session, if any.
    pub active: Option<FocusSession>,
    /// Set when this call finished (or re-reported) a session, so the UI can celebrate/record it.
    pub finished: Option<FocusSession>,
    /// Server time the elapsed/remaining values were computed at.
    pub now_ms: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartSessionInput {
    pub kind: SessionKind,
    /// Optional open task to focus on (focus sessions only).
    #[serde(default)]
    pub task_id: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimerSettings {
    pub focus_minutes: u32,
    pub break_minutes: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusHistory {
    /// Finished focus sessions, newest first.
    pub sessions: Vec<FocusSession>,
    /// Focus time and session count for sessions that started today (local date).
    pub today_focus_ms: i64,
    pub today_sessions: u32,
}
