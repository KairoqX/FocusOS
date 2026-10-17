# Kaze Study

Offline-first study workspace. Tauri 2 + Rust backend, React + TypeScript + Vite frontend, SQLite storage.

**Status: Milestone 3 (subjects, chapters, progress).** See "Verification status" below.

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
  append-only (0001 tasks/subjects, 0002 chapters). A database from a newer app version is refused.
- **Deleting a subject** deletes its chapters; its tasks are kept (they become subject-less).
- **Deleting a chapter or task** deletes only that record.

## Semantics
- Chapter statuses: Not started, In progress, Completed, Needs revision.
- **Completion %** = completed chapters / all chapters, rounded half up; 0 when a subject has no chapters.
  "Needs revision" counts as not completed. Computed in Rust from stored records on every read.
- Subject names are unique (case-insensitive); chapter titles are unique per subject (case-insensitive).
- Tasks: **Today** = open and due today or earlier; **Upcoming** = open and due later; undated tasks
  appear in **All tasks**. Completing a task twice keeps the original completion time.

## Backend commands
Tasks: `list_tasks`, `create_task`, `update_task`, `delete_task`, `set_task_completed`.
Subjects: `list_subjects` (includes progress), `create_subject`, `update_subject`, `delete_subject`.
Chapters: `list_chapters`, `create_chapter`, `update_chapter`, `update_chapter_status`, `delete_chapter`.
App: `get_app_info`.

## Shortcuts
Ctrl+1..6 switch views · Ctrl+N new task (Tasks view).

## Verification status
Written without a Rust toolchain. The SQL (both migrations, constraints, aggregate queries, cascade/
set-null behaviour, v1->v2 upgrade) was executed against SQLite, and the TypeScript was checked
against stub types. The Rust has **not** been compiled and `cargo test` has **not** been run here.
