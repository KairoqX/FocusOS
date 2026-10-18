use kaze_core::db;
use kaze_core::error::AppError;
use kaze_core::models::{ChapterInput, ChapterStatus, Priority, Subject, SubjectInput, TaskInput};
use kaze_core::{chapters, subjects, tasks};
use rusqlite::Connection;

const TODAY: &str = "2025-03-15";

fn conn() -> Connection {
    db::open_in_memory().unwrap()
}

fn subject(c: &Connection, name: &str) -> Subject {
    subjects::create(c, &SubjectInput { name: name.into() }).unwrap()
}

fn chapter(c: &Connection, subject_id: i64, title: &str, status: ChapterStatus) -> i64 {
    let ch = chapters::create(
        c,
        subject_id,
        &ChapterInput {
            title: title.into(),
        },
    )
    .unwrap();
    if status != ChapterStatus::NotStarted {
        chapters::set_status(c, ch.id, status).unwrap();
    }
    ch.id
}

fn task_for(c: &Connection, title: &str, subject_id: i64) -> i64 {
    let input = TaskInput {
        title: title.into(),
        description: String::new(),
        subject_id: Some(subject_id),
        chapter_id: None,
        priority: Priority::Medium,
        due_date: None,
        estimated_minutes: None,
    };
    tasks::create(c, &input, TODAY).unwrap().id
}

fn validation_field<T: std::fmt::Debug>(r: Result<T, AppError>) -> &'static str {
    match r {
        Err(AppError::Validation { field, .. }) => field,
        other => panic!("expected validation error, got {other:?}"),
    }
}

#[test]
fn subject_names_are_trimmed_required_and_unique_ignoring_case() {
    let c = conn();
    assert_eq!(subject(&c, "  Physics ").name, "Physics");
    assert_eq!(
        validation_field(subjects::create(&c, &SubjectInput { name: "  ".into() })),
        "name"
    );
    assert_eq!(
        validation_field(subjects::create(
            &c,
            &SubjectInput {
                name: "PHYSICS".into()
            }
        )),
        "name"
    );
}

#[test]
fn renaming_a_subject() {
    let c = conn();
    let physics = subject(&c, "Physics");
    let other = subject(&c, "Chemistry");

    // Changing only the case of its own name is allowed.
    let renamed = subjects::update(
        &c,
        physics.id,
        &SubjectInput {
            name: "PHYSICS".into(),
        },
    )
    .unwrap();
    assert_eq!(renamed.name, "PHYSICS");

    // Taking another subject's name is not.
    let clash = subjects::update(
        &c,
        physics.id,
        &SubjectInput {
            name: "chemistry".into(),
        },
    );
    assert_eq!(validation_field(clash), "name");
    assert_eq!(subjects::get(&c, other.id).unwrap().name, "Chemistry");

    assert!(matches!(
        subjects::update(&c, 999, &SubjectInput { name: "x".into() }),
        Err(AppError::NotFound {
            entity: "subject",
            id: 999
        })
    ));
}

#[test]
fn empty_subject_has_zero_progress() {
    let c = conn();
    let s = subject(&c, "Empty");
    assert_eq!(
        (
            s.chapter_count,
            s.completed,
            s.percent_complete,
            s.task_count,
            s.open_tasks
        ),
        (0, 0, 0, 0, 0)
    );
}

#[test]
fn status_counts_come_from_chapter_records() {
    let c = conn();
    let s = subject(&c, "Biology");
    chapter(&c, s.id, "Cells", ChapterStatus::Completed);
    chapter(&c, s.id, "Genetics", ChapterStatus::Completed);
    chapter(&c, s.id, "Ecology", ChapterStatus::InProgress);
    chapter(&c, s.id, "Evolution", ChapterStatus::NeedsRevision);
    chapter(&c, s.id, "Anatomy", ChapterStatus::NotStarted);

    let got = subjects::get(&c, s.id).unwrap();
    assert_eq!(got.chapter_count, 5);
    assert_eq!(
        (
            got.not_started,
            got.in_progress,
            got.completed,
            got.needs_revision
        ),
        (1, 1, 2, 1)
    );
    // Status is the user's label; progress comes from tasks, and there are none yet.
    assert_eq!((got.percent_complete, got.chapters_without_tasks), (0, 5));

    let cells = chapters::list(&c, s.id).unwrap()[0].id;
    chapters::set_status(&c, cells, ChapterStatus::NeedsRevision).unwrap();
    let got = subjects::get(&c, s.id).unwrap();
    assert_eq!((got.completed, got.needs_revision), (1, 2));
}

#[test]
fn list_summaries_are_independent_per_subject_and_sorted_by_name() {
    let c = conn();
    let zed = subject(&c, "zed");
    let alpha = subject(&c, "Alpha");
    chapter(&c, zed.id, "One", ChapterStatus::Completed);
    chapter(&c, alpha.id, "One", ChapterStatus::NotStarted);
    chapter(&c, alpha.id, "Two", ChapterStatus::NotStarted);

    let all = subjects::list(&c).unwrap();
    assert_eq!(
        all.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
        ["Alpha", "zed"]
    );
    assert_eq!((all[0].chapter_count, all[0].completed), (2, 0));
    assert_eq!((all[1].chapter_count, all[1].completed), (1, 1));
}

#[test]
fn task_counts_do_not_inflate_chapter_counts() {
    let c = conn();
    let s = subject(&c, "History");
    chapter(&c, s.id, "Rome", ChapterStatus::NotStarted);
    chapter(&c, s.id, "Greece", ChapterStatus::NotStarted);
    let t1 = task_for(&c, "Essay", s.id);
    task_for(&c, "Reading", s.id);
    task_for(&c, "Flashcards", s.id);
    tasks::set_completed(&c, t1, true, TODAY).unwrap();

    let got = subjects::get(&c, s.id).unwrap();
    assert_eq!(
        (got.chapter_count, got.task_count, got.open_tasks),
        (2, 3, 2)
    );
}

#[test]
fn chapter_titles_are_unique_per_subject_ignoring_case() {
    let c = conn();
    let a = subject(&c, "A");
    let b = subject(&c, "B");
    chapter(&c, a.id, "Intro", ChapterStatus::NotStarted);
    assert_eq!(
        validation_field(chapters::create(
            &c,
            a.id,
            &ChapterInput {
                title: "INTRO".into()
            }
        )),
        "title"
    );
    // The same title in a different subject is fine.
    assert!(chapters::create(
        &c,
        b.id,
        &ChapterInput {
            title: "Intro".into()
        }
    )
    .is_ok());
    assert_eq!(
        validation_field(chapters::create(
            &c,
            a.id,
            &ChapterInput { title: "  ".into() }
        )),
        "title"
    );
}

#[test]
fn chapters_list_in_creation_order_and_can_be_renamed() {
    let c = conn();
    let s = subject(&c, "S");
    let first = chapter(&c, s.id, "Zebra", ChapterStatus::NotStarted);
    chapter(&c, s.id, "Apple", ChapterStatus::NotStarted);
    let titles = |c: &Connection| {
        chapters::list(c, s.id)
            .unwrap()
            .into_iter()
            .map(|x| x.title)
            .collect::<Vec<_>>()
    };
    assert_eq!(titles(&c), ["Zebra", "Apple"]);

    chapters::update(
        &c,
        first,
        &ChapterInput {
            title: "Mango".into(),
        },
    )
    .unwrap();
    assert_eq!(titles(&c), ["Mango", "Apple"]);

    // Cannot rename onto a sibling's title; renaming to itself (other case) is fine.
    assert_eq!(
        validation_field(chapters::update(
            &c,
            first,
            &ChapterInput {
                title: "apple".into()
            }
        )),
        "title"
    );
    assert!(chapters::update(
        &c,
        first,
        &ChapterInput {
            title: "MANGO".into()
        }
    )
    .is_ok());
}

#[test]
fn missing_records_report_not_found() {
    let c = conn();
    assert!(matches!(
        chapters::list(&c, 5),
        Err(AppError::NotFound {
            entity: "subject",
            ..
        })
    ));
    assert!(matches!(
        chapters::create(&c, 5, &ChapterInput { title: "x".into() }),
        Err(AppError::NotFound {
            entity: "subject",
            ..
        })
    ));
    assert!(matches!(
        chapters::set_status(&c, 5, ChapterStatus::Completed),
        Err(AppError::NotFound {
            entity: "chapter",
            ..
        })
    ));
    assert!(matches!(
        chapters::delete(&c, 5),
        Err(AppError::NotFound { .. })
    ));
    assert!(matches!(
        subjects::delete(&c, 5),
        Err(AppError::NotFound { .. })
    ));
}

#[test]
fn deleting_a_chapter_updates_subject_counts() {
    let c = conn();
    let s = subject(&c, "S");
    let done = chapter(&c, s.id, "Done", ChapterStatus::Completed);
    chapter(&c, s.id, "Todo", ChapterStatus::NotStarted);
    let before = subjects::get(&c, s.id).unwrap();
    assert_eq!((before.chapter_count, before.completed), (2, 1));
    chapters::delete(&c, done).unwrap();
    let got = subjects::get(&c, s.id).unwrap();
    assert_eq!((got.chapter_count, got.completed), (1, 0));
}

#[test]
fn deleting_a_subject_cascades_chapters_but_keeps_tasks() {
    let c = conn();
    let s = subject(&c, "Doomed");
    let other = subject(&c, "Safe");
    chapter(&c, s.id, "Ch1", ChapterStatus::Completed);
    chapter(&c, other.id, "Ch1", ChapterStatus::Completed);
    let t = task_for(&c, "Keep me", s.id);

    subjects::delete(&c, s.id).unwrap();

    let remaining: i64 = c
        .query_row("SELECT COUNT(*) FROM chapters", [], |r| r.get(0))
        .unwrap();
    assert_eq!(remaining, 1, "only the other subject's chapter remains");
    let kept = tasks::get(&c, t, TODAY).unwrap();
    assert_eq!((kept.subject_id, kept.subject_name), (None, None));
}

#[test]
fn database_rejects_bad_chapter_rows_directly() {
    let c = conn();
    let s = subject(&c, "S");
    assert!(c
        .execute(
            "INSERT INTO chapters (subject_id, title) VALUES (999, 'x')",
            []
        )
        .is_err());
    assert!(c
        .execute(
            "INSERT INTO chapters (subject_id, title, status) VALUES (?1, 'x', 'bogus')",
            [s.id]
        )
        .is_err());
    assert!(c
        .execute(
            "INSERT INTO chapters (subject_id, title) VALUES (?1, '  ')",
            [s.id]
        )
        .is_err());
}
