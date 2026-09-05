# ROADMAP

Milestones follow the product charter. Each milestone ends with:
test → build → real smoke test → docs update → commit → push.

## M0 — Foundation (current)

Tauri 2 + Svelte 5 + TypeScript skeleton; SQLite with versioned migrations
(desktop core, layout snapshots, scenes, tasks/calendar, focus sessions);
repository layer; structured errors; bounded rotated logging; design tokens;
minimal windowed shell with navigation and theme persistence; Windows shell
probe plan (+ prototype if time). Acceptance: dev runs, cargo tests pass,
frontend tests/check/lint pass, release build succeeds.

## M1 — Application Shell

Tray icon + menu (open, quit), close-to-tray behavior, global shortcut for the
command palette (with conflict detection UI), settings persistence, theme
system completion (system/light/dark + accent), Today page content shell.
No fake dashboard data.

## M2 — Desktop Index

Known-folder desktop discovery (user + public desktop, redirect-aware),
filesystem watcher (debounced) instead of polling, files/folders/.lnk indexing,
lazy bounded icon cache, open item via shell, search, virtual collections,
drag/drop assignment, persistence. **Fallback-first: the app must be fully
useful with zero shell integration.**

Status: **complete, WINDOWS_TESTED 2026-09-05** — discovery, watcher, index,
open, search, icon cache, and virtual collections with drag/drop assignment
(required `dragDropEnabled: false`, see DECISIONS D12).

## M3 — Windows Shell Integration

> **Status: core delivered 2026-09-05** — capture/apply via LVM + canary guard (D15), settings UI shipped; live read/canary verified. Deferred: Explorer-restart persistence re-apply.

Run the IFolderView probe for real; if viable: layout snapshot, position read,
position write, restore, "before layout" auto-backup, auto-arrange detection.
Then (behind runtime detection + fallback) desktop group background host
(WorkerW research), multi-monitor + DPI handling, Explorer-restart recovery.
Untested Win32 claims are forbidden — everything here needs WINDOWS_TESTED.

## M4 — Scenes

> **Status: core delivered 2026-09-05** — scene CRUD + per-scene collection visibility + switcher + restore-previous (D16). Deferred: per-scene global shortcuts, focus-scene handoff (with M5), geometry columns.

Create/edit scenes, per-scene collection visibility/layout, scene switcher,
keyboard shortcuts, restore previous scene, focus-scene handoff state.

## M5 — Focus

Pomodoro 25/5, 50/10, custom, count-up; task/scene binding; tray timer state;
compact timer window; interruptions + notes; daily focus summary; scene
integration (soft filtering only — no app blocking in V1).

## M6 — Tasks + Calendar

Tasks (todo/doing/done, priority, due, estimate, tags, fast capture Ctrl+N),
Agenda + Week views, then Month; drag task → calendar to create a time block;
local persistence; ICS import/export after stabilization.

## M7 — Customization

Theme presets (Minimal/Windows/Soft/OLED), accent colors, opacity, density,
icon sizes, animation level, performance mode, layout presets. Focus on
actually-used polish over settings count.

## M8 — Reliability

Adversarial testing: restart, corrupted DB, missing files, Explorer restart,
multi-monitor, DPI 100–200%, hundreds of desktop files, sleep/wake, logout/
login. Fix what breaks; no new features until P0/P1 are clear.

## M9 — Performance

Measure first (docs/PERFORMANCE.md scenarios: 0/50/200/500 desktop items).
Startup, idle CPU/RAM (whole process tree), icon extraction, DB queries, FS
event handling. Optimize only what measurement justifies.

## M10 — Product Polish

Continuous real use; audit visual hierarchy, spacing, alignment, copy,
keyboard flow, empty/error/loading states, drag affordance, right-click,
tray UX, settings discoverability. Log findings in ITERATION_LOG.md.

## M11 — Release Candidate

Fresh install / upgrade / uninstall; DB survives upgrade; no destructive
desktop behavior; restore layout works; all tests + release build green;
README complete; NSIS installer generated.
