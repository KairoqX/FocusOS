//! Locks the JSON shapes the TypeScript frontend depends on.
use kaze_core::error::AppError;
use kaze_core::models::{
    Chapter, ChapterInput, ChapterStatus, Priority, Subject, SubjectInput, Task, TaskInput,
    TaskQuery, TaskSort, TaskView,
};
use serde_json::json;

#[test]
fn query_deserializes_from_frontend_shape() {
    let q: TaskQuery = serde_json::from_value(json!({
        "view": "upcoming", "sort": "dueDate",
        "subjectId": null, "priority": "high", "search": "algebra"
    }))
    .unwrap();
    assert_eq!(q.view, TaskView::Upcoming);
    assert_eq!(q.sort, TaskSort::DueDate);
    assert_eq!(q.priority, Some(Priority::High));
    assert_eq!(q.search.as_deref(), Some("algebra"));
}

#[test]
fn unknown_enum_values_are_rejected() {
    assert!(
        serde_json::from_value::<TaskQuery>(json!({"view": "someday", "sort": "title"})).is_err()
    );
    assert!(
        serde_json::from_value::<TaskInput>(json!({"title": "x", "priority": "urgent"})).is_err()
    );
}

#[test]
fn input_accepts_minimal_and_full_payloads() {
    let min: TaskInput = serde_json::from_value(json!({"title": "x", "priority": "low"})).unwrap();
    assert!(min.description.is_empty() && min.due_date.is_none() && min.subject_id.is_none());
    let full: TaskInput = serde_json::from_value(json!({
        "title": "x", "description": "d", "subjectId": 2, "priority": "medium",
        "dueDate": "2025-01-01", "estimatedMinutes": 30
    }))
    .unwrap();
    assert_eq!(
        (full.subject_id, full.estimated_minutes),
        (Some(2), Some(30))
    );
}

#[test]
fn task_serializes_camel_case() {
    let t = Task {
        id: 1,
        title: "t".into(),
        description: String::new(),
        subject_id: Some(2),
        subject_name: Some("S".into()),
        priority: Priority::High,
        due_date: Some("2025-01-01".into()),
        estimated_minutes: Some(10),
        completed: false,
        completed_at: None,
        overdue: true,
        created_at: "c".into(),
        updated_at: "u".into(),
    };
    let v = serde_json::to_value(&t).unwrap();
    for key in [
        "subjectId",
        "subjectName",
        "dueDate",
        "estimatedMinutes",
        "completedAt",
        "createdAt",
        "updatedAt",
        "overdue",
    ] {
        assert!(v.get(key).is_some(), "missing key {key}");
    }
    assert_eq!(v["priority"], "high");
}

#[test]
fn errors_serialize_with_kind_tag() {
    let v = serde_json::to_value(AppError::validation("title", "Title is required.")).unwrap();
    assert_eq!(
        v,
        json!({"kind": "validation", "field": "title", "message": "Title is required."})
    );
    let v = serde_json::to_value(AppError::not_found("task", 7)).unwrap();
    assert_eq!(v, json!({"kind": "notFound", "entity": "task", "id": 7}));
}

#[test]
fn chapter_status_uses_camel_case_on_the_wire() {
    for (status, wire) in [
        (ChapterStatus::NotStarted, "notStarted"),
        (ChapterStatus::InProgress, "inProgress"),
        (ChapterStatus::Completed, "completed"),
        (ChapterStatus::NeedsRevision, "needsRevision"),
    ] {
        assert_eq!(serde_json::to_value(status).unwrap(), json!(wire));
        assert_eq!(
            serde_json::from_value::<ChapterStatus>(json!(wire)).unwrap(),
            status
        );
    }
    assert!(serde_json::from_value::<ChapterStatus>(json!("needs_revision")).is_err());
}

#[test]
fn subject_and_chapter_shapes() {
    let s = Subject {
        id: 1,
        name: "S".into(),
        chapter_count: 4,
        not_started: 1,
        in_progress: 1,
        completed: 1,
        needs_revision: 1,
        percent_complete: 25,
        task_count: 3,
        open_tasks: 2,
    };
    let v = serde_json::to_value(&s).unwrap();
    for key in [
        "chapterCount",
        "notStarted",
        "inProgress",
        "completed",
        "needsRevision",
        "percentComplete",
        "taskCount",
        "openTasks",
    ] {
        assert!(v.get(key).is_some(), "missing key {key}");
    }
    let c = Chapter {
        id: 1,
        subject_id: 2,
        title: "t".into(),
        status: ChapterStatus::InProgress,
        created_at: "c".into(),
        updated_at: "u".into(),
    };
    let v = serde_json::to_value(&c).unwrap();
    assert_eq!(
        (v["subjectId"].clone(), v["status"].clone()),
        (json!(2), json!("inProgress"))
    );

    let si: SubjectInput = serde_json::from_value(json!({"name": "x"})).unwrap();
    let ci: ChapterInput = serde_json::from_value(json!({"title": "y"})).unwrap();
    assert_eq!((si.name.as_str(), ci.title.as_str()), ("x", "y"));
}
