# FocusOS

Offline-first study workspace. Tauri 2 + Rust backend, React + TypeScript + Vite frontend, SQLite storage.

**Status: Milestone 2 (database + task CRUD).** See "Verification status" below.

## Layout

```
crates/focusos-core/   Rust business logic: models, validation, migrations, repositories.
                       No Tauri dependency -> portable, fast to test.
src-tauri/             Thin Tauri shell: window, app-data path, typed commands.
src/                   React UI. All backend calls go through src/api.ts.
assets/                Icon source image (regenerate icons with `npx tauri icon assets/app-icon-source.png`).
```

The Cargo workspace root is the project folder (`Cargo.toml`). Build output goes to `./target`
(older `src-tauri/target` folders from Milestone 1 can be deleted).

## Prerequisites (Windows)
Rust (MSVC): `rustup default stable-msvc` · Node 20+ · VS Build Tools "Desktop development with C++" · WebView2.
SQLite is compiled in (rusqlite `bundled`), so no separate SQLite install is needed.

## Run
```powershell
npm install
npm run tauri dev
```

## Checks
```powershell
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p focusos-core        # fast; does not compile Tauri
npm run typecheck
```

## Data
- Database: `focusos.db` in the OS per-user app-data directory (shown on the Dashboard).
- Schema is versioned with `PRAGMA user_version`; migrations live in `crates/focusos-core/migrations`
  and are append-only. A database from a newer app version is refused, never modified.
- Deleting a subject keeps its tasks (they become subject-less). Deleting a task deletes only that task.

## Task semantics
- **Today**: open tasks due today or earlier (overdue included, flagged "Overdue").
- **Upcoming**: open tasks due after today. Undated tasks appear only in **All tasks**.
- **Completed**: tasks with a completion time. Completing twice keeps the original time.
- Dates are local `YYYY-MM-DD`; timestamps are UTC ISO-8601.

## Backend commands (Milestone 2)
`get_app_info`, `list_tasks`, `create_task`, `update_task`, `delete_task`,
`set_task_completed` (replaces `complete_task`; also reopens), `list_subjects` (read-only until Milestone 3).

## Shortcuts
Ctrl+1..6 switch views · Ctrl+N new task (Tasks view).

## Verification status
Written without access to a Rust toolchain. The SQL (migration, constraints, queries) was executed
against SQLite and the TypeScript was checked against stub types. The Rust code has **not** been
compiled and `cargo test` has **not** been run. Run the checks above and report any failures.
