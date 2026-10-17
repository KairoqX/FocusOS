use focusos_core::db;
use focusos_core::error::AppError;
use focusos_core::models::{Priority, Task, TaskInput, TaskQuery, TaskSort, TaskView};
use focusos_core::tasks;
use rusqlite::Connection;

const TODAY: &str = "2025-03-15";

fn conn() -> Connection {
    db::open_in_memory().unwrap()
}

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

fn due(title: &str, date: &str) -> TaskInput {
    TaskInput {
        due_date: Some(date.to_owned()),
        ..input(title)
    }
}

fn add_subject(conn: &Connection, name: &str) -> i64 {
    conn.execute("INSERT INTO subjects (name) VALUES (?1)", [name])
        .unwrap();
    conn.last_insert_rowid()
}

fn list(conn: &Connection, q: &TaskQuery) -> Vec<Task> {
    tasks::list(conn, q, TODAY).unwrap()
}

fn titles(v: &[Task]) -> Vec<&str> {
    v.iter().map(|t| t.title.as_str()).collect()
}

#[test]
fn create_and_get_round_trip() {
    let c = conn();
    let sid = add_subject(&c, "Chemistry");
    let created = tasks::create(
        &c,
        &TaskInput {
            title: "  Titration lab  ".into(),
            description: "Write up results".into(),
            subject_id: Some(sid),
            priority: Priority::High,
            due_date: Some("2025-03-20".into()),
            estimated_minutes: Some(90),
        },
        TODAY,
    )
    .unwrap();

    assert_eq!(created.title, "Titration lab");
    assert_eq!(created.subject_id, Some(sid));
    assert_eq!(created.subject_name.as_deref(), Some("Chemistry"));
    assert_eq!(created.priority, Priority::High);
    assert_eq!(created.due_date.as_deref(), Some("2025-03-20"));
    assert_eq!(created.estimated_minutes, Some(90));
    assert!(!created.completed && created.completed_at.is_none() && !created.overdue);
    assert_eq!(tasks::get(&c, created.id, TODAY).unwrap(), created);
}

#[test]
fn invalid_input_is_rejected_before_writing() {
    let c = conn();
    assert!(matches!(
        tasks::create(&c, &input("   "), TODAY),
        Err(AppError::Validation { field: "title", .. })
    ));
    assert!(matches!(
        tasks::create(&c, &due("x", "2025-02-30"), TODAY),
        Err(AppError::Validation {
            field: "dueDate",
            ..
        })
    ));
    let n: i64 = c
        .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 0);
}

#[test]
fn unknown_subject_is_a_validation_error() {
    let c = conn();
    let mut i = input("x");
    i.subject_id = Some(999);
    assert!(matches!(
        tasks::create(&c, &i, TODAY),
        Err(AppError::Validation {
            field: "subjectId",
            ..
        })
    ));
}

#[test]
fn database_enforces_foreign_keys_directly() {
    let c = conn();
    let r = c.execute(
        "INSERT INTO tasks (title, subject_id) VALUES ('x', 999)",
        [],
    );
    assert!(r.is_err());
}

#[test]
fn update_changes_fields_and_missing_id_is_not_found() {
    let c = conn();
    let t = tasks::create(&c, &input("Old"), TODAY).unwrap();
    let mut i = due("New", "2025-04-01");
    i.priority = Priority::Low;
    let u = tasks::update(&c, t.id, &i, TODAY).unwrap();
    assert_eq!((u.title.as_str(), u.priority), ("New", Priority::Low));
    assert_eq!(u.due_date.as_deref(), Some("2025-04-01"));
    assert!(matches!(
        tasks::update(&c, 12345, &i, TODAY),
        Err(AppError::NotFound {
            entity: "task",
            id: 12345
        })
    ));
}

#[test]
fn delete_removes_and_missing_is_not_found() {
    let c = conn();
    let t = tasks::create(&c, &input("Gone"), TODAY).unwrap();
    tasks::delete(&c, t.id).unwrap();
    assert!(matches!(
        tasks::get(&c, t.id, TODAY),
        Err(AppError::NotFound { .. })
    ));
    assert!(matches!(
        tasks::delete(&c, t.id),
        Err(AppError::NotFound { .. })
    ));
}

#[test]
fn complete_reopen_and_idempotence() {
    let c = conn();
    let t = tasks::create(&c, &input("Do it"), TODAY).unwrap();

    let done = tasks::set_completed(&c, t.id, true, TODAY).unwrap();
    assert!(done.completed && done.completed_at.is_some());

    // Re-completing must not move the original completion time.
    c.execute(
        "UPDATE tasks SET completed_at = '2020-01-01T00:00:00Z' WHERE id = ?1",
        [t.id],
    )
    .unwrap();
    let again = tasks::set_completed(&c, t.id, true, TODAY).unwrap();
    assert_eq!(again.completed_at.as_deref(), Some("2020-01-01T00:00:00Z"));

    let reopened = tasks::set_completed(&c, t.id, false, TODAY).unwrap();
    assert!(!reopened.completed && reopened.completed_at.is_none());

    assert!(matches!(
        tasks::set_completed(&c, 999, true, TODAY),
        Err(AppError::NotFound { .. })
    ));
}

#[test]
fn deleting_a_subject_keeps_its_tasks() {
    let c = conn();
    let sid = add_subject(&c, "History");
    let mut i = input("Essay");
    i.subject_id = Some(sid);
    let t = tasks::create(&c, &i, TODAY).unwrap();

    c.execute("DELETE FROM subjects WHERE id = ?1", [sid])
        .unwrap();
    let after = tasks::get(&c, t.id, TODAY).unwrap();
    assert_eq!(after.subject_id, None);
    assert_eq!(after.subject_name, None);
}

#[test]
fn views_split_tasks_by_date_and_completion() {
    let c = conn();
    tasks::create(&c, &due("overdue", "2025-03-10"), TODAY).unwrap();
    tasks::create(&c, &due("today", "2025-03-15"), TODAY).unwrap();
    tasks::create(&c, &due("future", "2025-03-20"), TODAY).unwrap();
    tasks::create(&c, &input("undated"), TODAY).unwrap();
    let finished = tasks::create(&c, &due("finished", "2025-03-15"), TODAY).unwrap();
    tasks::set_completed(&c, finished.id, true, TODAY).unwrap();

    let today = list(&c, &TaskQuery::for_view(TaskView::Today));
    assert_eq!(titles(&today), ["overdue", "today"]);
    assert!(today[0].overdue && !today[1].overdue);

    assert_eq!(
        titles(&list(&c, &TaskQuery::for_view(TaskView::Upcoming))),
        ["future"]
    );
    assert_eq!(
        titles(&list(&c, &TaskQuery::for_view(TaskView::Completed))),
        ["finished"]
    );
    assert_eq!(list(&c, &TaskQuery::for_view(TaskView::All)).len(), 5);
}

#[test]
fn filters_for_subject_priority_and_search() {
    let c = conn();
    let math = add_subject(&c, "Math");
    let art = add_subject(&c, "Art");
    let mut a = input("100% algebra");
    a.subject_id = Some(math);
    a.priority = Priority::High;
    let mut b = input("100 sketches");
    b.subject_id = Some(art);
    let mut d = input("Geometry set");
    d.subject_id = Some(math);
    d.description = "ruler and compass".into();
    for i in [&a, &b, &d] {
        tasks::create(&c, i, TODAY).unwrap();
    }
    let all = TaskQuery::for_view(TaskView::All);

    let by_subject = TaskQuery {
        subject_id: Some(math),
        ..all.clone()
    };
    assert_eq!(list(&c, &by_subject).len(), 2);

    let by_priority = TaskQuery {
        priority: Some(Priority::High),
        ..all.clone()
    };
    assert_eq!(titles(&list(&c, &by_priority)), ["100% algebra"]);

    // '%' is matched literally, not as a wildcard.
    let literal = TaskQuery {
        search: Some("100%".into()),
        ..all.clone()
    };
    assert_eq!(titles(&list(&c, &literal)), ["100% algebra"]);

    // Case-insensitive, and searches descriptions too.
    let desc = TaskQuery {
        search: Some("COMPASS".into()),
        ..all.clone()
    };
    assert_eq!(titles(&list(&c, &desc)), ["Geometry set"]);

    let combined = TaskQuery {
        subject_id: Some(math),
        priority: Some(Priority::Medium),
        ..all
    };
    assert_eq!(titles(&list(&c, &combined)), ["Geometry set"]);
}

#[test]
fn sorting() {
    let c = conn();
    let mut low = due("b-low", "2025-03-01");
    low.priority = Priority::Low;
    let mut high_late = due("c-high-late", "2025-03-09");
    high_late.priority = Priority::High;
    let mut high_early = due("a-high-early", "2025-03-05");
    high_early.priority = Priority::High;
    let undated = input("d-undated");
    for i in [&low, &high_late, &high_early, &undated] {
        tasks::create(&c, i, TODAY).unwrap();
    }
    let q = |sort| TaskQuery {
        sort,
        ..TaskQuery::for_view(TaskView::All)
    };

    assert_eq!(
        titles(&list(&c, &q(TaskSort::DueDate))),
        ["b-low", "a-high-early", "c-high-late", "d-undated"]
    );
    assert_eq!(
        titles(&list(&c, &q(TaskSort::Priority))),
        ["a-high-early", "c-high-late", "d-undated", "b-low"]
    );
    assert_eq!(
        titles(&list(&c, &q(TaskSort::Title))),
        ["a-high-early", "b-low", "c-high-late", "d-undated"]
    );
    assert_eq!(list(&c, &q(TaskSort::Created))[0].title, "d-undated");
}

#[test]
fn hostile_text_is_stored_literally() {
    let c = conn();
    let evil = "x'); DROP TABLE tasks; --";
    let t = tasks::create(&c, &input(evil), TODAY).unwrap();
    assert_eq!(t.title, evil);
    let q = TaskQuery {
        search: Some("DROP TABLE".into()),
        ..TaskQuery::for_view(TaskView::All)
    };
    assert_eq!(list(&c, &q).len(), 1);
}
