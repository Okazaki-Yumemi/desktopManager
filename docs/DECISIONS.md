# Decisions

ADR-style log. Newest at the bottom. Status: proposed / accepted / superseded.

## D1 — Plain Svelte 5 + Vite, not SvelteKit (accepted, 2026-09-05)

create-tauri-app's svelte-ts template now ships SvelteKit. SvelteKit brings
SSR machinery, routing conventions and adapter layers a desktop shell does not
need. Plain Svelte 5 + vite-plugin-svelte keeps the bundle tiny, startup fast,
and matches the planned `pages/components/stores` structure. Runes-style
stores (`.svelte.ts`) replace a store library.

## D2 — rusqlite (bundled) in Rust, not tauri-plugin-sql (accepted, 2026-09-05)

The charter forbids scattering SQL through UI code. tauri-plugin-sql exposes
SQL to the JS side, which invites exactly that. rusqlite keeps queries behind
Rust repositories; the `bundled` feature compiles SQLite so there is no system
dependency, at the cost of some compile time.

## D3 — tracing + tracing-appender for logging (accepted, 2026-09-05)

Daily rotated files with a 14-day retention sweep at startup satisfy the
"bounded, privacy-safe, no spam" requirement without a server-grade stack.
Release default level: info for crates, debug for our lib; debug builds also
log to stdout.

## D4 — Windows SDK lives on G: via a directory junction (accepted, 2026-09-05)

Machine constraint from the user: developer SDK payloads must not consume C:.
The standalone SDK installer insists on `C:\Program Files (x86)\Windows
Kits\10` (registry-visible path that MSVC/rustc tooling discovers). Solution:
`C:\Program Files (x86)\Windows Kits\10` is a junction to `G:\WindowsSDK\10`,
so every tool sees the standard path while the bytes live on G:.
Documented here so a future agent does not "fix" it.

## D5 — Git Bash builds need the MSVC bin dir first in PATH (accepted, 2026-09-05)

MSYS2's GNU `link` shadows MSVC `link.exe`, breaking Rust builds from Git
Bash. `scripts/winbuild-env.sh` prepends the detected MSVC bin directory.
Never rely on ambient PATH for Rust builds in this shell.

## D6 — Item identity = absolute path (accepted, 2026-09-05)

Desktop items are identified by their absolute path in `desktop_items` and
`collection_items`. It is stable across renames of metadata, matches the shell
world, and keeps the indexer stateless. File identity via inode is not
portable and unnecessary at this scope.

## D7 — Never hand-write Win32 constants; import from the windows crate (accepted, 2026-09-05)

The shell probe's first write attempt used a hand-computed message constant
(`LVM_FIRST+15` believed to be `LVM_SETITEMPOSITION32`; it is actually
`LVM_SETITEMPOSITION`, whose lParam packs coordinates). The remote-memory
pointer was interpreted as packed coordinates, icons landed at pseudo-random
positions, and Explorer's off-view icon rescue scrambled the desktop (fully
recovered from the probe snapshot; see WINDOWS_SHELL_PROBE.md). Rule: every
Win32 constant, flag and message value comes from the `windows` crate import —
hand-written hex is a build-blocking review finding.

## D8 — Shell icon backend: LVM messages as primary, COM IFolderView kept as first-try (accepted, 2026-09-05)

On this machine (Win11 build 26200) `CLSID_ShellWindows` activates but its
collection is empty (`Count()=0`; `FindWindowSW(SWC_DESKTOP)` returns S_OK
with a NULL dispatch across all VARIANT-type/flag variants), so the documented
COM `IFolderView` route cannot be used here. The probe verified the
SysListView32 LVM route end-to-end instead: read (`LVM_GETITEMCOUNT`,
`LVM_GETITEMPOSITION`, `LVM_GETITEMTEXTW` with remote buffers) and write
(`LVM_SETITEMPOSITION32`) round-trip exactly, without elevation and without
touching files. Runtime behavior: try COM first (cleaner, no cross-process
memory when available), fall back to LVM; both behind one repository
interface so M3/M4 code never branches on the backend.

## D9 — UI language is Chinese (accepted, 2026-09-05)

User decision ("用中文"). All UI copy, tray menu labels, dates and durations
are Chinese (`zh-CN`); product/brand strings (DesktopManager) stay as-is.
Backends logs and code comments remain English for greppability.

## D10 — `display_name`: shortcuts show stem, everything else the real file name (accepted, 2026-09-05)

The desktop never shows `.lnk`, so the indexer strips that suffix (and `.url`).
Regular files keep their full name with extension: we do not replicate
Explorer's per-type "hide known extensions" behavior, and an honest name is
better for search. Revisit only if it bothers real use.

## D11 — Watcher = debounce + full rescan, not incremental event replay (accepted, 2026-09-05)

Reconciling raw `ReadDirectoryChangesW` event streams (rename pairs, lost
events, buffer overflows) is fragile exactly where data safety matters. The
watcher therefore only signals "something happened"; the scan re-reads both
desktop folders (a few ms) and diffs inside one SQLite transaction. Costs a
little more I/O, buys determinism: the index is always a pure function of the
directory contents.

## D12 — `dragDropEnabled: false` on the main window (accepted 2026-09-05; superseded by D13 the same day)

Tauri's default (`dragDropEnabled: true`) installs its own OLE drop target on
WebView2 to expose external file drops as Tauri events — and in doing so it
swallows in-page HTML5 dragstart/drop, which made collection drag assignment
dead on Windows. Flipped to `false` to fix internal drag (see D13 for the
final resolution).

## D13 — Internal drag is pointer-based; `dragDropEnabled` stays default (accepted, 2026-09-05)

D12's flag is also Tauri's only channel for external drops with real paths,
which the user explicitly wants (drag shortcuts from Explorer into a
collection). Both at once is impossible with HTML5 DnD on WebView2, so:

- `dragDropEnabled` is back to **true** → external file drops arrive as
  Tauri drag-drop events with absolute paths.
- Internal card→chip drag is a hand-rolled **pointer drag** (mousedown,
  6 px threshold, `elementFromPoint` chip hit-test, floating ghost). It
  never touches dragstart, so it works regardless of the native drop
  handler and gives full visual control of the drag ghost.

## D14 — Collections may hold external items; opening stays allow-listed (accepted, 2026-09-05)

Users can drag shortcuts/files/folders that are **not on the desktop** into a
collection. Non-indexed paths are stored with snapshot metadata (label, kind,
ext, size, mtime) on the `collection_items` row (migration 0006); if the path
is (or becomes) desktop-indexed, live metadata wins at read time. Opening:
`collection_open` allows a path if it is visible in the desktop index **or**
held by any collection — user-curated lists act as the allow-list, the same
trust level as double-clicking in Explorer. `desktop_icon` was widened the
same way so external items render real shell icons.

## D15 — Desktop icon layout via LVM + canary guard (2026-09-05, M3)

- The probe-verified route (D7, docs/WINDOWS_SHELL_PROBE.md) is now the product
  path: `desktop::shell_layout` ports the probe's LVM implementation
  (Progman/WorkerW → SHELLDLL_DefView → SysListView32, remote buffers in
  explorer.exe). The COM/IFolderView route stays out of the app — blocked on
  this machine (probe route 1).
- Stored positions are ListView **client coordinates, verbatim** (probe's
  coordinate-space notes: cross-process MapWindowPoints is unreliable). A
  restore writes them back unchanged; re-anchoring across monitor-layout
  changes remains open.
- Matching is by listview caption; duplicate captions are consumed in order;
  icons absent from the desktop are counted as missing, never an error.
- **Canary guard**: before any batch restore, icon 0 is moved +150/+150 px and
  read back. A no-op read-back means the shell overrides writes (自动排列图标)
  and the restore is refused outright. The canary always attempts to return to
  its original spot first. Align-to-grid snapping counts as accepted (a grid
  move is not a no-op).
- Honest status: read + canary verified live on this machine (ignored tests,
  `cargo test -- --ignored` 2/2); the settings UI and a real user restore pass
  are still pending one manual run.

## D16 — Scenes hide collections by default-visible rows (2026-09-05, M4)

- Scenes (0003 tables) get their V1 semantics: a scene stores per-collection
  visibility rows; **collections without a row are visible** (empty scene =
  everything shows). Orphaned rows from deleted collections are filtered via
  the visibility JOIN, independent of the foreign_keys pragma.
- UI model: hidden collections stay in the chips row but dimmed with an
  EyeOff toggle, so un-hiding is always one click and never needs an edit
  mode. Geometry columns (pos/size/collapsed) stay unused for now.
- Switching: clicking the active scene chip restores the previous scene;
  the active scene persists in settings (`ui.activeScene`).
- Deferred from M4 scope: global per-scene keyboard shortcuts and the
  focus-scene handoff state (M5 territory). Honestly labeled UNVERIFIED→
  verified: create/activate/delete + persistence loop exercised live on this
  machine.
## D17 — Focus sessions use the database as the clock (2026-09-05, M5)

- A running focus session is a focus_sessions row with started_at; elapsed
  time is derived from it at read time (UI ticks drive the display only). A
  crash, restart or webview reload cannot lose a running session: the Focus
  page recovers it via focus_running() on mount and keeps counting.
  start() refuses while another row is running, so zombie running rows
  cannot accumulate.
- V1 semantics: sessions end as completed or abandoned with
  actual_duration_s = (ended_at - started_at) in whole seconds; mid-session
  interruptions are tallied in the interruptions column (the schema's
  `interrupted` status stays reserved). Breaks between blocks are UI-only
  and not persisted — closing the app during a break loses only the break.
- Crossing the planned duration while the page is open auto-completes and
  starts the preset break. A session recovered already past its plan is
  never auto-judged: the user chooses 完成/放弃 (over-time is shown).
- Task binding exists in schema/commands (task_id) but the UI exposes only
  scene binding until M6 ships the task list. Scene integration is soft: a
  one-click 应用场景 writes ui.activeScene (the M4 key), never forced; no
  app blocking in V1.
- Deferred from the M5 line: tray timer state, compact timer window, audio
  notification on phase end.

## D18 — Frosted-glass surfaces over the wallpaper (2026-09-05, UX round)

- User feedback: opaque cards hide the custom wallpaper and feel heavy. All
  major surfaces (sidebar, cards, chips, item cards, search, toasts, drag
  ghost) now use a translucent tint var(--glass) plus
  backdrop-filter: var(--glass-filter) (blur 22px + saturate), themed per
  light/dark in tokens.css. Tokens are the single source; components never
  hard-code the recipe. Hover/active states keep the existing opaque-ish
  tokens for contrast.

## D19 — Sub-collections and in-app folder browsing (2026-09-05, UX round)

- Collections may nest (migration 0008, collections.parent_id). Depth is
  capped at 5 in the repo; delete removes the whole subtree via a recursive
  CTE regardless of the foreign_keys pragma; rename is duplicate-checked.
  The chips row renders depth-first with indent, so sub-collections appear
  directly under their parent.
- Folder references expand in place: browse_children lists the immediate
  children of a directory (read-only, 500-entry cap, hidden entries
  skipped, dirs first). The open/icon/browse allow-list (D14) is extended:
  a path is also allowed when it lives inside a desktop-indexed or
  collection-held directory — expanding a curated folder must not dead-end.
  This remains read-only metadata/file-open policy; nothing is moved.
- The 今天 page is a welcome screen (big clock + per-day motto + today's
  focus total); backend status and the quick-start hint moved to Settings.


## D20 — Appearance preferences ride `data-*` attributes and tokens (2026-09-05, M7)

- Four enum preferences (surface style, density, glass strength, motion) are
  stored in the existing settings table (`ui.surface`, `ui.density`,
  `ui.glass`, `ui.motion`) and mirrored onto `<html data-*>` attributes by a
  shared `enumPref` factory. tokens.css owns every visual consequence; no
  component reads the attributes. Values are validated through a pure
  `resolveEnum` allow-list helper (unit tested), so a corrupted settings row
  degrades to the default instead of breaking the UI.
- Glass strength tunes `--glass-alpha` / `--glass-blur` which the theme
  tints consume via `var(…)`; `off` sets `--glass-filter: none` and alpha 1
  (solid panels). The `oled` preset re-tints only the dark palette — under
  the light theme it is honestly a no-op rather than a broken hybrid.
  Motion `off` zeroes the duration tokens and force-disables transitions and
  keyframes globally.

## D21 — Write `data-*` attributes with setAttribute, not the dataset setter (2026-09-06, R15)

- The desktop icon-size preference (ui.iconSize) is the first enumPref whose
  attribute name contains a dash (`data-icon-size`). Live smoke showed the
  value updating in Svelte state while the attribute never appeared: on
  Chromium/WebView2 `documentElement.dataset["icon-size"] = …` throws
  `SyntaxError: Failed to set a named property 'icon-size' on 'DOMStringMap':
  'icon-size' is not a valid property name` — after the in-memory assignment
  has already run, so the UI looks half-broken rather than dead. Dash-free
  names (surface/density/glass/motion) never hit this path.
- enumPref now writes through `setAttribute("data-" + attr, …)` in both
  load() and set(), always with the full literal attribute name. Rule for
  future code: never index `dataset` with a dashed name on the write path.

## D22 — Corrupt database: quarantine and rebuild, never delete (2026-09-06, M8)

- A corrupt SQLite file previously failed `AppState::init`, so the app would
  not start at all and the only manual fix was deleting the data directory
  — the opposite of the data-safety-first charter.
- `Database::open_with_recovery` treats two signals as corruption:
  SQLITE_NOTADB / SQLITE_CORRUPT from opening the file, and any
  `PRAGMA quick_check` answer other than `ok` (this also catches damage the
  header alone does not reveal). It renames the db and any -wal/-shm
  siblings to `<name>.corrupt-<epoch-ms>` in place, then opens a fresh
  database and continues startup. Renamed, never deleted — the user can
  still try to salvage the bytes. Non-corruption failures keep failing
  loudly instead of silently wiping state.
- quick_check runs at every startup. On this app's DB size class the cost is
  negligible (the M8 scale test syncs 520 items in 5 ms debug; quick_check
  is the same order); M9 re-measures startup with it in the path.

## D23 — ICS export: floating local times, exclusive all-day DTEND (2026-09-06, R18)

- The calendar stores epoch millis and shows wall-clock times with no
  timezone model, so the honest iCalendar representation is a *floating*
  local time (`DTSTART:20260906T090000`, no TZID, no Z). Emitting UTC or
  inventing a TZID would claim knowledge the app does not have and would
  shift events when the file crosses timezones. Consequence: files exported
  and re-imported by the same machine round-trip exactly; other machines
  interpret them as their local wall time.
- All-day events use `DTSTART;VALUE=DATE` with an exclusive `DTEND` (start +
  1 day) as RFC 5545 requires; the app stores all-day end as the same
  midnight the UI means, so the exporter adds the day.
- TEXT values are escaped per RFC §3.3.11 and every content line is folded
  at the 75-octet limit on char boundaries (CJK-safe, round-trip tested).
  Import (parsing other producers' quirks) is deliberately deferred.
