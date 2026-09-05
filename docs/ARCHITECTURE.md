# Architecture

## Layering

```
Svelte UI (src/)
    ↓ invoke (typed wrappers in src/services)
Tauri commands (src-tauri/src/commands/)   ← thin, no SQL, no business logic
    ↓
Rust domain/state (src-tauri/src/app/)
    ↓
Storage repositories (src-tauri/src/storage/)  ← the ONLY place SQL lives
    ↓
SQLite (single file in %APPDATA%\com.okazakiyumemi.desktopmanager)
```

Windows-specific behavior (shell integration, known folders, icons) is
isolated in the desktop adapter module (`src-tauri/src/desktop/`), never
scattered through commands or UI code. Business logic stays platform-neutral
so the virtual organizer fallback always works.

## Backend modules

| Module        | Responsibility                                                        |
| ------------- | --------------------------------------------------------------------- |
| `app/state`   | AppState (DB handle, dirs, desktop sources), mutex access with poisoning recovery |
| `app/error`   | `AppError` (thiserror) — serializes to a message string over IPC      |
| `app/logging` | tracing + daily rotated file sink, 14-day retention, stdout in debug  |
| `app/shell`   | Tray icon + show/hide/toggle of the main window                       |
| `app/shortcuts` | Global shortcut registration (conflict-tolerant) + status           |
| `desktop/discovery` | Known-folder desktop discovery (user + public, redirect-aware) |
| `desktop/scanner`   | Top-level scan of a desktop folder into `ScannedItem`s         |
| `desktop/watcher`   | Event-driven fs watcher, debounced rescans (no polling)        |
| `desktop/service`   | Scan orchestration + `desktop:changed` event emission          |
| `desktop/open`      | ShellExecuteW open (the only way items are launched)           |
| `storage`     | Database open (WAL, foreign keys, busy timeout), in-memory variant    |
| `storage/migrations` | Ordered SQL migrations, per-version transaction, pre-upgrade backup |
| `storage/*_repo` | Repository structs; typed queries per aggregate                   |
| `commands/`   | `#[tauri::command]` handlers; lock DB, call repo, map errors          |

## Frontend modules

| Path                  | Responsibility                                        |
| --------------------- | ----------------------------------------------------- |
| `src/pages/`          | One component per top-level page                      |
| `src/components/`     | Reusable presentational components                    |
| `src/stores/*.svelte.ts` | Svelte 5 runes state (router, theme, later: data)  |
| `src/services/backend.ts` | The only place calling `invoke`                    |
| `src/styles/tokens.css` | Design tokens (single visual source of truth)       |
| `src/lib/`            | Pure helpers (unit tested)                            |

## Database schema (v1..v5)

Migration 0001 core desktop: `settings`, `collections`, `desktop_items`,
`collection_items`. 0002: `layout_snapshots`. 0003: `scenes`, `scene_layouts`.
0004: `tasks`, `calendar_events`. 0005: `focus_sessions`.

Key decisions:

- Desktop items are keyed by absolute path (`desktop_items.path UNIQUE`);
  `missing` flag tracks files that vanished from disk without deleting
  assignments.
- Layout coordinates are stored as logical pixels + monitor id, so layouts
  survive DPI changes; physical pixels only appear in `layout_snapshots`
  payloads (what the shell API actually needs).
- Settings are key → JSON value rows; typed convenience getters in the repo.
- `schema_migrations` records applied versions; upgrading an existing DB takes
  a timestamped backup copy first (in `backups/` beside the DB file).

## Error handling rules

- Production code returns `AppResult<T>`; no `unwrap`/`expect` outside tests
  and process startup (`run()`).
- A poisoned DB mutex maps to `AppError::Other` instead of panicking.
- Frontend surfaces errors as toasts/inline states; a missing desktop file is
  data, not a crash.

## Concurrency & performance notes

- One SQLite connection behind a mutex is sufficient; WAL + busy timeout keep
  the UI process and background tasks from blocking fatally. Revisit with a
  pool/read-connection if profiling justifies it.
- Filesystem watching is event-driven with a 500 ms quiet-period debounce; no
  periodic rescans anywhere. A rescan is a top-level `read_dir` of two folders
  (single-digit milliseconds for typical desktops) inside one transaction —
  cheaper and far less fragile than incremental event reconciliation. The UI
  is told only when the index actually changed (`desktop:changed`).
