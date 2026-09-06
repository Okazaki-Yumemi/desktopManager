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

## D24 — SJTU sync: webview login + same-origin fetch + receive-only IPC (2026-09-06, M12)

- **Login never touches this app.** The sync window is a plain Tauri
  webview pointed at `https://my.sjtu.edu.cn/ui/calendar/`; the user types
  jAccount credentials into the university's own page, so the password and
  the session cookies live only in the WebView2 profile (DPAPI-protected at
  rest). The app has no code path that reads cookies — no reqwest client
  carries the session, and nothing is exported.
- **Data path is page-side fetch → one receive-only command.** An
  initialization script injected into that window fetches the calendar API
  *same-origin* (candidates: `/ui/api/calendar`, `/ui/api/event/list`, then
  `https://calendar.sjtu.edu.cn/api/event/list` — the exact portal endpoint
  is not publicly documented, so the script tries the same-origin prefixes
  first and falls back to the calendar service host; the first JSON with
  `data.events` wins) and pushes the raw JSON body through
  `sjtu_receive`. Tauri's ACL blocks all IPC from remote origins unless a
  capability explicitly grants it, so `capabilities/sjtu-remote.json` grants
  exactly one permission (`allow-sjtu-receive`, one command, payload
  size-capped at 2 MB, strictly parsed) to window `sjtu` on
  `https://my.sjtu.edu.cn` — no reads, no plugins, nothing else. Worst case
  if the university page were compromised: an attacker could write fake
  calendar rows into this app's local DB. Nothing more.
- **Consequence felt app-wide: the app now has an ACL manifest.** Defining
  any `permissions/*.toml` flips Tauri into strict mode — *every*
  application command must be granted explicitly, even to the main window.
  `permissions/allow-app-commands/default.toml` lists all 53 commands;
  adding a new `#[tauri::command]` without adding it there breaks the
  frontend call with "not allowed by ACL" (loud in dev builds). This also
  hardens the app: local windows are no longer implicitly trusted for
  commands either.
- **Payload semantics.** Either the API wrapper `{status, data}` or a bare
  `{events, schoolCalendar}` object is accepted; naive "YYYY-MM-DD HH:MM"
  times are taken as local machine time (same honesty rule as D23); events
  with unusable times are skipped and counted, never fatal. The whole
  `sjtu_events` table is replaced per sync in one transaction — re-syncs
  cannot leave duplicates, and "delete on the university side" propagates
  by replacement. `recurrence` is stored but not expanded: the app imports
  exactly the occurrences the API returns.
- **Reminder is client-side and session-scoped**: a 20 s ticker fires one
  toast + chime ten minutes before each class while the app is open; after
  a restart an imminent class may chime once more (safe direction).
- **Amendment (2026-09-06, R22, field-tested).** The first live run forced
  three changes. (1) The portal embeds the calendar app from
  `calendar.sjtu.edu.cn` in a cross-origin iframe, so same-origin
  candidates on my.sjtu.edu.cn 404 and a cross-origin fetch is
  CORS-blocked. The init script now passively hooks the page's own
  fetch/XHR and forwards the first response matching the calendar JSON
  shape — the endpoint, query params and auth stay entirely the page's
  own. The capability additionally grants `https://calendar.sjtu.edu.cn`
  (same receive-only surface, still nothing else). (2) `sjtu_open_sync`
  must be an async command: creating a window from a synchronous command
  deadlocks WebView2 initialization on Windows — the window frame appears
  but never paints and cannot be closed (exactly the v1.0.0 field report).
  (3) Closing the sync window without a sync emits `sjtu-window-closed`
  so the frontend re-arms the sync button instead of staying disabled.
  End-to-end verified in the real app: auto-login via persisted session →
  sniffed payload → DB replace → sidebar/week view → auto-close.

## D25 — Visual language 1.1.0: tokens first, structure where it serves correctness (2026-09-06, R23)

The v1.1.0 round is a full-frontend polish pass (user request). Ground
rules, in charter priority order (correctness/usability above visual
polish), and what they produced:

- **Tokens stay the single source of truth.** New primitives are theme
  blocks, not component constants: a shadow ramp (`--shadow-sm/md/lg`,
  `--shadow` aliases md), `--grad-accent` (accent-derived gradient, works
  with the custom color picker because it is `color-mix` on `--accent`),
  `--accent-contrast` (readable text ON accent: white in light, dark navy
  in dark — the old `#fff`-on-light-blue primary buttons failed contrast),
  and `--accent-ring` for focus glows.
- **The `.glass` class was defined globally — it previously did not
  exist**, so the 交大 sidebar and the 当日议程 panel silently rendered
  with no background at all. One utility in `base.css` now provides the
  frosted recipe; scoped styles keep border/radius.
- **Focus visibility is global** (`:focus-visible` ring; text fields swap
  the outline for a soft glow so it never fights their border styles).
- **Animations are opt-in utility classes** (`page-enter`, `toast-enter`)
  and are neutralized by `data-motion="off"` and `prefers-reduced-motion`
  — the M7 motion contract still holds.
- **The week view was restructured into one shared grid** (time gutter +
  7 day columns × header/all-day/time rows). This is a usability/correctness
  fix, not decoration: previously each column was an independent flex
  column, so a day with all-day events pushed its time grid out of
  alignment with the other columns; and there were no hour labels at all.
  Hour labels, weekend tint, and a gradient now-line with a dot came with
  the restructure for free.
- **Stale copy is part of polish**: the settings 快速上手 section still
  said 任务/日历 "will arrive with M6" and the footer note promised M7
  features that shipped long ago; both rewritten to describe current
  behavior.
