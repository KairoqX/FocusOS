//! Task <-> chapter links and the derived (never stored) chapter/subject progress.
use kaze_core::db;
use kaze_core::error::AppError;
use kaze_core::models::{
    ChapterInput, ChapterProgressInput, ChapterStatus, Priority, ProgressMode, Subject,
    SubjectInput, TaskInput,
};
use kaze_core::{chapters, subjects, tasks};
use rusqlite::Connection;

const TODAY: &str = "2025-03-15";

fn conn() -> Connection {
    db::open_in_memory().unwrap()
}

fn subject(c: &Connection, name: &str) -> Subject {
    subjects::create(c, &SubjectInput { name: name.into() }).unwrap()
}

fn chapter(c: &Connection, subject_id: i64, title: &str) -> i64 {
    chapters::create(
        c,
        subject_id,
        &ChapterInput {
            title: title.into(),
        },
    )
    .unwrap()
    .id
}

fn input(title: &str, subject_id: Option<i64>, chapter_id: Option<i64>) -> TaskInput {
    TaskInput {
        title: title.into(),
        description: String::new(),
        subject_id,
        chapter_id,
        priority: Priority::Medium,
        due_date: None,
        estimated_minutes: None,
    }
}

fn task(c: &Connection, title: &str, subject_id: Option<i64>, chapter_id: Option<i64>) -> i64 {
    tasks::create(c, &input(title, subject_id, chapter_id), TODAY)
        .unwrap()
        .id
}

fn complete(c: &Connection, id: i64) {
    tasks::set_completed(c, id, true, TODAY).unwrap();
}

/// (completed tasks, total tasks, effective percent)
fn progress(c: &Connection, chapter_id: i64) -> (u32, u32, u8) {
    let ch = chapters::get(c, chapter_id).unwrap();
    (ch.task_completed, ch.task_total, ch.progress_percent)
}

fn set_manual(c: &Connection, id: i64, value: i64) {
    let input = ChapterProgressInput {
        mode: ProgressMode::Manual,
        manual_progress: Some(value),
    };
    chapters::set_progress(c, id, &input).unwrap();
}

fn set_automatic(c: &Connection, id: i64) -> kaze_core::models::Chapter {
    let input = ChapterProgressInput {
        mode: ProgressMode::Automatic,
        manual_progress: None,
    };
    chapters::set_progress(c, id, &input).unwrap()
}

fn field<T: std::fmt::Debug>(r: Result<T, AppError>) -> &'static str {
    match r {
        Err(AppError::Validation { field, .. }) => field,
        other => panic!("expected validation error, got {other:?}"),
    }
}

fn assert_no_fk_violations(c: &Connection) {
    let mut stmt = c.prepare("PRAGMA foreign_key_check").unwrap();
    assert!(
        stmt.query([]).unwrap().next().unwrap().is_none(),
        "dangling foreign key found"
    );
}

// ---------- defaults and empty states ----------

#[test]
fn new_chapters_default_to_automatic() {
    let c = conn();
    let s = subject(&c, "S");
    let ch = chapters::get(&c, chapter(&c, s.id, "Intro")).unwrap();
    assert_eq!(ch.progress_mode, ProgressMode::Automatic);
    assert_eq!(
        (
            ch.manual_progress,
            ch.task_total,
            ch.task_completed,
            ch.progress_percent
        ),
        (0, 0, 0, 0)
    );
}

#[test]
fn empty_chapter_and_empty_subject_never_divide_by_zero() {
    let c = conn();
    let empty_subject = subject(&c, "Empty");
    let s = subject(&c, "S");
    chapter(&c, s.id, "No tasks");

    let e = subjects::get(&c, empty_subject.id).unwrap();
    assert_eq!(
        (
            e.chapter_count,
            e.percent_complete,
            e.chapters_without_tasks
        ),
        (0, 0, 0)
    );
    let got = subjects::get(&c, s.id).unwrap();
    assert_eq!(
        (
            got.chapter_count,
            got.percent_complete,
            got.chapters_without_tasks
        ),
        (1, 0, 1)
    );
}

#[test]
fn a_chapter_without_tasks_is_distinguishable_from_zero_percent_with_tasks() {
    let c = conn();
    let s = subject(&c, "S");
    let none = chapter(&c, s.id, "None");
    let zero = chapter(&c, s.id, "Zero");
    task(&c, "a", Some(s.id), Some(zero));
    task(&c, "b", Some(s.id), Some(zero));

    assert_eq!(progress(&c, none), (0, 0, 0));
    assert_eq!(progress(&c, zero), (0, 2, 0));
    // Only the chapter with no tasks at all is flagged.
    assert_eq!(subjects::get(&c, s.id).unwrap().chapters_without_tasks, 1);
}

// ---------- automatic progress ----------

#[test]
fn automatic_progress_follows_completion_and_reopening() {
    let c = conn();
    let s = subject(&c, "S");
    let ch = chapter(&c, s.id, "Ch");
    let t1 = task(&c, "1", Some(s.id), Some(ch));
    let t2 = task(&c, "2", Some(s.id), Some(ch));
    let t3 = task(&c, "3", Some(s.id), Some(ch));
    assert_eq!(progress(&c, ch), (0, 3, 0));

    complete(&c, t1);
    assert_eq!(progress(&c, ch), (1, 3, 33));
    complete(&c, t2);
    complete(&c, t3);
    assert_eq!(progress(&c, ch), (3, 3, 100));

    tasks::set_completed(&c, t2, false, TODAY).unwrap();
    assert_eq!(progress(&c, ch), (2, 3, 67));
}

#[test]
fn completing_twice_does_not_double_count() {
    let c = conn();
    let s = subject(&c, "S");
    let ch = chapter(&c, s.id, "Ch");
    let t = task(&c, "t", Some(s.id), Some(ch));
    task(&c, "u", Some(s.id), Some(ch));
    complete(&c, t);
    complete(&c, t);
    assert_eq!(progress(&c, ch), (1, 2, 50));
}

#[test]
fn tasks_are_counted_once_even_with_many_chapters_and_subjects() {
    let c = conn();
    let a = subject(&c, "A");
    let b = subject(&c, "B");
    let a1 = chapter(&c, a.id, "a1");
    let a2 = chapter(&c, a.id, "a2");
    let b1 = chapter(&c, b.id, "b1");
    for i in 0..3 {
        task(&c, &format!("a1-{i}"), Some(a.id), Some(a1));
    }
    task(&c, "a2-0", Some(a.id), Some(a2));
    task(&c, "b1-0", Some(b.id), Some(b1));
    assert_eq!(progress(&c, a1).1, 3);
    assert_eq!(progress(&c, a2).1, 1);
    assert_eq!(progress(&c, b1).1, 1);
    assert_eq!(subjects::get(&c, a.id).unwrap().task_count, 4);
}

// ---------- manual mode ----------

#[test]
fn manual_progress_is_not_overwritten_by_task_changes() {
    let c = conn();
    let s = subject(&c, "S");
    let ch = chapter(&c, s.id, "Ch");
    set_manual(&c, ch, 70);

    let t1 = task(&c, "1", Some(s.id), Some(ch));
    let t2 = task(&c, "2", Some(s.id), Some(ch));
    complete(&c, t1);
    complete(&c, t2);
    let got = chapters::get(&c, ch).unwrap();
    assert_eq!(
        (got.progress_mode, got.manual_progress, got.progress_percent),
        (ProgressMode::Manual, 70, 70)
    );
    assert_eq!((got.task_completed, got.task_total), (2, 2)); // task facts stay visible

    tasks::delete(&c, t1).unwrap();
    tasks::set_completed(&c, t2, false, TODAY).unwrap();
    assert_eq!(chapters::get(&c, ch).unwrap().progress_percent, 70);
}

#[test]
fn switching_back_to_automatic_derives_from_tasks_immediately() {
    let c = conn();
    let s = subject(&c, "S");
    let ch = chapter(&c, s.id, "Ch");
    let t = task(&c, "1", Some(s.id), Some(ch));
    task(&c, "2", Some(s.id), Some(ch));
    complete(&c, t);

    set_manual(&c, ch, 80);
    assert_eq!(chapters::get(&c, ch).unwrap().progress_percent, 80);

    let auto = set_automatic(&c, ch); // returned value is already derived from tasks
    assert_eq!(
        (auto.progress_mode, auto.progress_percent),
        (ProgressMode::Automatic, 50)
    );
    assert_eq!(
        auto.manual_progress, 80,
        "stored manual value is kept for later"
    );

    // Switching to manual without a value reuses the stored one.
    let again = chapters::set_progress(
        &c,
        ch,
        &ChapterProgressInput {
            mode: ProgressMode::Manual,
            manual_progress: None,
        },
    )
    .unwrap();
    assert_eq!(again.progress_percent, 80);
}

#[test]
fn manual_progress_is_validated_and_bounds_are_inclusive() {
    let c = conn();
    let s = subject(&c, "S");
    let ch = chapter(&c, s.id, "Ch");
    set_manual(&c, ch, 0);
    set_manual(&c, ch, 100);
    set_manual(&c, ch, 35);
    for bad in [101, -1, i64::MAX] {
        let r = chapters::set_progress(
            &c,
            ch,
            &ChapterProgressInput {
                mode: ProgressMode::Manual,
                manual_progress: Some(bad),
            },
        );
        assert_eq!(field(r), "manualProgress");
    }
    assert_eq!(
        chapters::get(&c, ch).unwrap().manual_progress,
        35,
        "rejected values change nothing"
    );
    assert!(matches!(
        chapters::set_progress(
            &c,
            999,
            &ChapterProgressInput {
                mode: ProgressMode::Automatic,
                manual_progress: None
            }
        ),
        Err(AppError::NotFound {
            entity: "chapter",
            id: 999
        })
    ));
}

// ---------- subject aggregation ----------

#[test]
fn subject_progress_is_the_unweighted_average_of_chapter_progress() {
    let c = conn();
    let s = subject(&c, "S");
    let done = chapter(&c, s.id, "Done"); // automatic, 1/1 = 100
    let zero = chapter(&c, s.id, "Zero"); // automatic, 0/2 = 0
    chapter(&c, s.id, "Empty"); //            automatic, no tasks = 0
    let manual = chapter(&c, s.id, "Manual"); // manual 50, no tasks
    let t = task(&c, "t", Some(s.id), Some(done));
    complete(&c, t);
    task(&c, "z1", Some(s.id), Some(zero));
    task(&c, "z2", Some(s.id), Some(zero));
    set_manual(&c, manual, 50);
    // Tasks without a chapter must not influence chapter or subject progress.
    let loose = task(&c, "loose", Some(s.id), None);
    complete(&c, loose);

    let got = subjects::get(&c, s.id).unwrap();
    assert_eq!(got.chapter_count, 4);
    assert_eq!(got.percent_complete, 38); // (100 + 0 + 0 + 50) / 4 = 37.5 -> 38
    assert_eq!(got.chapters_without_tasks, 1); // "Empty" only; the manual one is not flagged
    assert_eq!(
        (got.task_count, got.open_tasks, got.unassigned_tasks),
        (4, 2, 1)
    );

    // list() must agree with get().
    assert_eq!(subjects::list(&c).unwrap()[0], got);
}

#[test]
fn chapter_status_does_not_affect_progress() {
    let c = conn();
    let s = subject(&c, "S");
    let ch = chapter(&c, s.id, "Ch");
    chapters::set_status(&c, ch, ChapterStatus::Completed).unwrap();
    assert_eq!(progress(&c, ch).2, 0);
    chapters::set_status(&c, ch, ChapterStatus::NeedsRevision).unwrap();
    assert_eq!(subjects::get(&c, s.id).unwrap().needs_revision, 1);
}

// ---------- linking, validation, reassignment ----------

#[test]
fn tasks_without_a_chapter_keep_working() {
    let c = conn();
    let s = subject(&c, "S");
    let a = task(&c, "subject only", Some(s.id), None);
    let b = task(&c, "no subject", None, None);
    let got = tasks::get(&c, a, TODAY).unwrap();
    assert_eq!((got.chapter_id, got.chapter_title), (None, None));
    complete(&c, a);
    tasks::delete(&c, b).unwrap();
    assert_eq!(subjects::get(&c, s.id).unwrap().unassigned_tasks, 1);
}

#[test]
fn task_returns_chapter_id_and_current_title() {
    let c = conn();
    let s = subject(&c, "S");
    let ch = chapter(&c, s.id, "Old title");
    let t = task(&c, "t", Some(s.id), Some(ch));
    chapters::update(
        &c,
        ch,
        &ChapterInput {
            title: "New title".into(),
        },
    )
    .unwrap();
    let got = tasks::get(&c, t, TODAY).unwrap();
    assert_eq!(
        (got.chapter_id, got.chapter_title.as_deref()),
        (Some(ch), Some("New title"))
    );
}

#[test]
fn a_chapter_must_exist_and_belong_to_the_selected_subject() {
    let c = conn();
    let a = subject(&c, "A");
    let b = subject(&c, "B");
    let in_b = chapter(&c, b.id, "In B");

    assert_eq!(
        field(tasks::create(&c, &input("x", None, Some(in_b)), TODAY)),
        "chapterId"
    );
    assert_eq!(
        field(tasks::create(
            &c,
            &input("x", Some(a.id), Some(in_b)),
            TODAY
        )),
        "chapterId"
    );
    assert_eq!(
        field(tasks::create(&c, &input("x", Some(a.id), Some(999)), TODAY)),
        "chapterId"
    );
    assert_eq!(
        field(tasks::create(&c, &input("x", Some(a.id), Some(0)), TODAY)),
        "chapterId"
    );
    let n: i64 = c
        .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 0, "nothing was written");
    assert!(tasks::create(&c, &input("ok", Some(b.id), Some(in_b)), TODAY).is_ok());
}

#[test]
fn reassigning_a_task_moves_it_between_chapters_without_double_counting() {
    let c = conn();
    let s = subject(&c, "S");
    let first = chapter(&c, s.id, "First");
    let second = chapter(&c, s.id, "Second");
    let t = task(&c, "mover", Some(s.id), Some(first));
    task(&c, "stays", Some(s.id), Some(first));
    complete(&c, t);
    assert_eq!(
        (progress(&c, first), progress(&c, second)),
        ((1, 2, 50), (0, 0, 0))
    );

    tasks::update(&c, t, &input("mover", Some(s.id), Some(second)), TODAY).unwrap();
    assert_eq!(
        (progress(&c, first), progress(&c, second)),
        ((0, 1, 0), (1, 1, 100))
    );

    tasks::update(&c, t, &input("mover", Some(s.id), None), TODAY).unwrap();
    assert_eq!(
        (progress(&c, first), progress(&c, second)),
        ((0, 1, 0), (0, 0, 0))
    );
    let subj = subjects::get(&c, s.id).unwrap();
    assert_eq!((subj.task_count, subj.unassigned_tasks), (2, 1));
}

#[test]
fn changing_a_tasks_subject_requires_a_matching_or_cleared_chapter() {
    let c = conn();
    let one = subject(&c, "One");
    let two = subject(&c, "Two");
    let ch_one = chapter(&c, one.id, "c1");
    let ch_two = chapter(&c, two.id, "c2");
    let t = task(&c, "t", Some(one.id), Some(ch_one));

    // Stale chapter from the old subject is rejected...
    assert_eq!(
        field(tasks::update(
            &c,
            t,
            &input("t", Some(two.id), Some(ch_one)),
            TODAY
        )),
        "chapterId"
    );
    assert_eq!(progress(&c, ch_one).1, 1, "failed update changed nothing");
    // ...clearing it works, and so does choosing the new subject's chapter.
    tasks::update(&c, t, &input("t", Some(two.id), None), TODAY).unwrap();
    assert_eq!(progress(&c, ch_one).1, 0);
    tasks::update(&c, t, &input("t", Some(two.id), Some(ch_two)), TODAY).unwrap();
    assert_eq!(progress(&c, ch_two).1, 1);
    // Removing the subject while keeping a chapter is invalid too.
    assert_eq!(
        field(tasks::update(&c, t, &input("t", None, Some(ch_two)), TODAY)),
        "chapterId"
    );
}

// ---------- deletion ----------

#[test]
fn deleting_a_task_updates_chapter_progress() {
    let c = conn();
    let s = subject(&c, "S");
    let ch = chapter(&c, s.id, "Ch");
    let done = task(&c, "done", Some(s.id), Some(ch));
    let open = task(&c, "open", Some(s.id), Some(ch));
    complete(&c, done);
    assert_eq!(progress(&c, ch), (1, 2, 50));
    tasks::delete(&c, open).unwrap();
    assert_eq!(progress(&c, ch), (1, 1, 100));
    tasks::delete(&c, done).unwrap();
    assert_eq!(progress(&c, ch), (0, 0, 0));
}

#[test]
fn deleting_a_chapter_keeps_its_tasks_and_their_subject() {
    let c = conn();
    let s = subject(&c, "S");
    let ch = chapter(&c, s.id, "Doomed");
    let t = task(&c, "t", Some(s.id), Some(ch));
    chapters::delete(&c, ch).unwrap();

    let got = tasks::get(&c, t, TODAY).unwrap();
    assert_eq!(
        (got.subject_id, got.chapter_id, got.chapter_title),
        (Some(s.id), None, None)
    );
    assert_eq!(subjects::get(&c, s.id).unwrap().unassigned_tasks, 1);
    assert_no_fk_violations(&c);
}

#[test]
fn deleting_a_subject_leaves_no_dangling_references() {
    let c = conn();
    let s = subject(&c, "Doomed");
    let keep = subject(&c, "Keep");
    let ch = chapter(&c, s.id, "Ch");
    let keep_ch = chapter(&c, keep.id, "Keep ch");
    let linked = task(&c, "linked", Some(s.id), Some(ch));
    let other = task(&c, "other", Some(keep.id), Some(keep_ch));

    subjects::delete(&c, s.id).unwrap();

    let got = tasks::get(&c, linked, TODAY).unwrap();
    assert_eq!((got.subject_id, got.chapter_id), (None, None));
    assert!(matches!(
        chapters::get(&c, ch),
        Err(AppError::NotFound { .. })
    ));
    let untouched = tasks::get(&c, other, TODAY).unwrap();
    assert_eq!(
        (untouched.subject_id, untouched.chapter_id),
        (Some(keep.id), Some(keep_ch))
    );
    assert_no_fk_violations(&c);
}

// ---------- database-level guarantees (independent of Rust validation) ----------

#[test]
fn database_rejects_inconsistent_links_even_via_raw_sql() {
    let c = conn();
    let a = subject(&c, "A");
    let b = subject(&c, "B");
    let in_b = chapter(&c, b.id, "In B");
    assert!(c
        .execute(
            "INSERT INTO tasks (title, subject_id, chapter_id) VALUES ('x', ?1, ?2)",
            [a.id, in_b]
        )
        .is_err());
    assert!(c
        .execute(
            "INSERT INTO tasks (title, chapter_id) VALUES ('x', ?1)",
            [in_b]
        )
        .is_err());
    assert!(c
        .execute(
            "UPDATE chapters SET subject_id = ?1 WHERE id = ?2",
            [a.id, in_b]
        )
        .is_err());
    assert!(c
        .execute(
            "UPDATE chapters SET manual_progress = 101 WHERE id = ?1",
            [in_b]
        )
        .is_err());
}

#[test]
fn raw_subject_delete_succeeds_regardless_of_foreign_key_action_order() {
    let c = conn();
    let s = subject(&c, "S");
    let ch = chapter(&c, s.id, "Ch");
    let t = task(&c, "t", Some(s.id), Some(ch));
    c.execute("DELETE FROM subjects WHERE id = ?1", [s.id])
        .unwrap();
    let got = tasks::get(&c, t, TODAY).unwrap();
    assert_eq!((got.subject_id, got.chapter_id), (None, None));
    assert_no_fk_violations(&c);
}
