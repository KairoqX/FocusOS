# Kaze Study

Offline-first study workspace. Tauri 2 + Rust backend, React + TypeScript + Vite frontend, SQLite storage.

**Status: Milestone 3 + task↔chapter linking and automatic/manual chapter progress.** See "Verification status" below.

## Layout

```
crates/kaze-core/   Rust business logic: models, validation, progress maths, migrations, repositories.
                    No Tauri dependency -> portable, fast to test.
src-tauri/          Thin Tauri shell: window, app-data path, typed commands.
src/                React UI. All backend calls go through src/api.ts.
assets/             Icon source image (regenerate icons: `npx tauri icon assets/app-icon-source.png`).
```

The Cargo workspace root is the project folder (`Cargo.toml`); build output goes to `./target`.

## Prerequisites (Windows)
Rust (MSVC): `rustup default stable-msvc` · Node 20+ · VS Build Tools "Desktop development with C++" · WebView2.
SQLite is compiled in (rusqlite `bundled`).

## Run
```powershell
npm install
npm run tauri dev
```

## Checks
```powershell
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p kaze-core        # fast; does not compile Tauri
npm run typecheck
```

## Data
- Database: `focusos.db` in the OS per-user app-data directory (path shown on the Dashboard).
  The file name and bundle identifier (`com.example.focusos`) predate the rename to Kaze Study and are
  kept on purpose: changing either moves the data folder and would hide existing data.
- Schema is versioned with `PRAGMA user_version`; migrations in `crates/kaze-core/migrations` are
  append-only (0001 tasks/subjects, 0002 chapters, 0003 task↔chapter link + chapter progress mode). A database from a newer app version is refused.
- **Deleting a task** deletes only that task. **Deleting a chapter** keeps its tasks (they keep their
  subject and lose the chapter). **Deleting a subject** deletes its chapters; its tasks are kept and lose
  both subject and chapter. This is enforced by foreign keys plus triggers, so it also holds for raw SQL.
- A task's chapter must belong to the task's own subject (checked in Rust with a field error, and again by
  DB triggers). A chapter can never move to another subject.
- Back up `focusos.db` before first launching a new version; migrations are forward-only.

## Semantics
- **Chapter progress** is never stored; it is derived on every read in `kaze-core/src/progress.rs`:
  - *Automatic* (default for every chapter): completed linked tasks / linked tasks, whole percent, rounded half up.
    A task is completed when `tasks.completed_at` is set. A chapter with no tasks shows 0% plus an
    "No tasks yet" empty state (visually distinct from 0% with tasks).
  - *Manual*: the percentage the user set (0-100). Task changes never write to it. Switching back to Automatic
    derives from tasks immediately (the manual value is remembered, not used).
- **Subject progress** = unweighted mean of its chapters' displayed percentages, rounded half up; 0 with no
  chapters. Tasks that have a subject but no chapter do not affect progress (they are counted separately).
- **Chapter status** (Not started / In progress / Completed / Needs revision) is the user's own label and is
  independent of progress. "Needs revision" feeds the "needs attention" count.
- Subject and chapter names: unique (case-insensitive); chapter titles unique per subject.
- Tasks: **Today** = open and due today or earlier; **Upcoming** = open and due later; undated tasks appear in
  **All tasks**. Completing a task twice keeps the original completion time.

## Backend commands
Tasks: `list_tasks`, `create_task`, `update_task`, `delete_task`, `set_task_completed`.
Subjects: `list_subjects` (includes progress), `create_subject`, `update_subject`, `delete_subject`.
Chapters: `list_chapters`, `create_chapter`, `update_chapter`, `update_chapter_status`, `update_chapter_progress`, `delete_chapter`.
App: `get_app_info`.

## Shortcuts
Ctrl+1..6 switch views · Ctrl+N new task (Tasks view).

## Tests
- Rust: `cargo test -p kaze-core` (unit + integration: progress maths, empty chapters, manual overrides,
  reassignment, completion/reopening, deletion, migrations incl. v1->v3 and v2->v3 upgrades, JSON contract).
- UI: `npm install` then `npm test` (Vitest + Testing Library, jsdom). Covers the dependent subject/chapter
  dropdowns (`TaskForm`), both progress modes (`ChapterProgress`) and the Subjects screen. Test files are
  excluded from `npm run typecheck`/`build` so they can never break a release build.

## Verification status
Written without a Rust toolchain or network access. **Not run here:** `cargo build/test/clippy/fmt`,
`npm test` (Vitest could not be installed), the Tauri window. **Run here:** every migration and the shipped
SQL against SQLite (upgrade paths, triggers, cascades, aggregate queries), a TypeScript check against stub
types, and equivalent UI scenarios in headless Chromium against the real components with a mocked IPC bridge.
