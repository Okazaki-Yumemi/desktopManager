# STATE

> Hand-off notes so a fresh agent (or human) can continue after context loss.
> Update after every significant work session. Last updated: 2026-09-06 (M12 SJTU integration + v1.0.0 prep).

## Morning handoff (overnight run 2026-09-05 → 2026-09-06) — READ ME FIRST

Overnight autonomous rounds, in order, all pushed to `main`:

| Round | Deliverable | Commit |
| ----- | ----------- | ------ |
| R13 | Calendar month view (42 cells, 周/月 toggle persisted) | 034680a |
| R14 | Focus completion chime (ui.sound) + custom accent picker | 57be16e |
| R15 | Desktop icon size 小/中/大 (ui.iconSize) + dataset bug fix D21 | ad1721d |
| R16 | M8 offline slice: corrupt-DB quarantine recovery (D22) + 500-file scale test | be8b626 |
| R17 | M9 measurement slice: 0/50/200/500 items, all ≤ 10 ms (MEASURED) | e032e08 |
| R18 | Calendar ICS export (D23) + 导出 ICS button | b6bb581 |
| R19 | Release build green: MSI 2.8 MB + NSIS 2.0 MB (exe 5.4 MB) | (this commit) |

Every round passed its gates before commit: cargo test (54/54 at R18),
clippy 0, svelte-check 0, eslint 0, plus a degraded browser smoke via the
vite dev harness. `pnpm tauri build` finished clean (exit 0, no warnings)
and produced both installers. **No round was verified inside the real
Tauri window** (the screen was locked all night; the WebView a11y tree was
unreachable), so treat frontend work as TESTED — not
WINDOWS_TESTED/USER_VERIFIED.

Suggested manual acceptance list (统一验收), roughly 10 minutes:

1. **外观 settings** (M7 + R14/R15): toggle 主题/强调色（含自定义取色器）/
   外观风格/密度/毛玻璃强度/动效/图标大小 — each applies instantly and
   survives an app restart (persistence goes through the settings table).
2. **桌面页**: icon size 小 changes the grid packing; 大 visibly widens
   cells to 3 columns at the usual window size.
3. **日历页**: month view navigation + click-select + dblclick-create;
   导出 ICS button → check `…/app-data/exports/calendar-*.ics` exists and
   opens in Outlook/Google Calendar (floating local times — D23).
4. **专注页**: run a preset session and let it hit the end → chime plays
   (audible? the locked screen made this unverifiable) and the session
   auto-completes; break-end chime + toast after the break.
5. **Corrupt-DB drill** (R16, optional but valuable): quit the app, write a
   few bytes of garbage into `desktopmanager.db`, restart → app starts with
   a fresh DB and `desktopmanager.db.corrupt-<ts>` holds the old bytes; the
   log records the quarantine.
6. **Release build** (R19): installers at
   `src-tauri/target/release/bundle/{msi,nsis}/` — install over the
   existing copy and confirm data survives.

After acceptance, the natural next steps are in ROADMAP: M10 product polish
from real use, then the remaining M8/M9/M11 live items (idle CPU/RAM,
icon extraction, event→UI latency, multi-monitor/DPI/sleep-wake drills).


## Project identity

- Repo: `git@github.com:Okazaki-Yumemi/desktopManager.git` (branch `main`)
- Local: `G:\Programming\Small_interests_projects\desktopManager`
- App id: `com.okazakiyumemi.desktopmanager`
- Product: DesktopManager — lightweight personal desktop workspace manager

## Environment (this machine)

- Windows 11 (build 26200), x64. Real desktop integration testable here.
- Node v24.15.0 at `G:\nodejs` (npm global prefix is BROKEN on this machine —
  `G:\nodejs\node_modules\npm\npmrc` sets `prefix=${APPDATA}\npm` which does
  not expand under Git Bash. Always use `corepack pnpm ...` instead of `pnpm`.)
- pnpm 11.25.0 via corepack (pinned in package.json `packageManager`).
- Rust stable 1.98.1 msvc (`%USERPROFILE%\.cargo\bin`, not in Git Bash PATH —
  add `~/.cargo/bin` per command or use scripts/).
- MSVC BuildTools 18 at `C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools`.
  Windows SDK 10.0.26100 installed **into `G:\WindowsSDK\10`** via a directory
  junction at `C:\Program Files (x86)\Windows Kits\10` (user requires SDK files
  not to live on C:).
- **Building Rust under Git Bash requires prepending the MSVC bin dir to PATH**
  (MSYS GNU `link` shadows MSVC `link.exe`) and, since SDK lives behind a
  junction, the SDK must be discoverable — see `scripts/winbuild-env.sh`.
- Two displays: main 2560x1600 @(0,0); secondary 2560x1440 @(-2560,-559).
  Known-folder desktop = `D:\Desktop` (redirected).

## Current milestone

M0–M2 complete. **M2 — Desktop Index: everything landed and WINDOWS_TESTED**
(discovery, scanner, index, debounced watcher, open, search, shell icons,
virtual collections + drag/drop assignment, Chinese UI).
M5 core **delivered 2026-09-05** (focus: presets 25/5 + 50/10 + custom + count-up, DB-backed timer with restart recovery, interruptions + notes, scene binding + soft apply, day list + 7-day summary; live smoke verified, D17). A UX round from direct user feedback **delivered 2026-09-05**: frosted-glass surfaces, collection rename, sub-collections with in-app folder browsing (migration 0008), 今天 welcome page (D18/D19). M4 core delivered (scenes) and M3 core delivered (LVM layout snapshot/restore + canary guard). The user confirmed both outstanding manual tests passed on 2026-09-05 — Explorer drag-into-collection and Settings 布局保存→应用 — so M3 layout restore is now USER_VERIFIED end to end. M6 core **delivered 2026-09-05** (tasks: quick capture Ctrl+N, todo/doing/done cycling, priority/due/notes inline editing, status filters + search; calendar: Monday-start week grid with click-to-create hour slots, all-day + timed events, task linking, day agenda; FocusPage binds a task into the session). Backend is unit-tested; frontend passed svelte-check/eslint plus a degraded browser smoke (structure, empty states, creator prefill) over vite dev without the Tauri backend — in-app visual pass left for the user, so treat both pages as TESTED (not WINDOWS_TESTED/USER_VERIFIED yet). M7 core **delivered 2026-09-05** (appearance presets standard/soft/sharp/OLED-dark, comfortable/compact density, glass strength off/soft/normal/strong, motion standard/reduced/off — all data-attribute + token driven, persisted via the settings table, D20). Calendar month view delivered 2026-09-05 night round (42-cell Monday-first grid, per-day event dots, dimmed out-month cells, click selects the day agenda, dblclick opens the all-day creator, 周/月 toggle persisted as ui.calendarView). Completion chime (WebAudio two-tone, on by default, toggle in a new Settings 通知 section, ui.sound) and a custom accent color picker (inline var overrides + ui.accentCustom, cleared when a preset is chosen) landed 2026-09-06. Desktop icon size (small/medium/large via ui.iconSize → data-icon-size → --desktop-cell/--desktop-icon consumed by the desktop grid) landed 2026-09-06; its live smoke exposed and fixed D21 (Chromium rejects dashed dataset writes — enumPref now uses setAttribute). M8's offline slice **delivered 2026-09-06**: `Database::open_with_recovery` detects a corrupt database (SQLITE_NOTADB from the first pragma, or a `PRAGMA quick_check` answer other than `ok`), renames the db plus stale -wal/-shm siblings to `*.corrupt-<epoch-ms>` (never deletes) and starts fresh (D22, 3 unit tests); a temp-dir scale test drives 500 files + 20 folders through scan → sync → vanish (scan 0 ms, first sync 5 ms; soft-remove converges). M9's autonomous slice **delivered 2026-09-06** (docs/PERFORMANCE.md, MEASURED): scan + DB queries at 0/50/200/500 items are all ≤ 10 ms at the top size even debug-built (500 items: scan 0.54 ms, first sync 9.01 ms, list 0.73 ms, search 0.19 ms; the D22 quick_check keeps startup open at ~2–3 ms) — no optimization justified. ICS export **delivered 2026-09-06** (R18): `calendar_ics.rs` serializes every event to an RFC 5545 VCALENDAR (unit-tested escaping, 75-octet folding that never splits UTF-8 chars, floating local datetimes, exclusive all-day DTEND — D23); `event_export_ics` writes `exports/calendar-<stamp>.ics` under the app data dir and the 日历 page has a 导出 ICS button. M12 **SJTU calendar integration delivered 2026-09-06** (user request): a dedicated 交大日程 sidebar on the calendar page fed by my.sjtu.edu.cn — the sync window opens the portal, the user logs into jAccount there (credentials never touch the app), an injected same-origin script pushes the calendar JSON through one receive-only command (`sjtu_receive`, the only IPC granted to the remote page; the app-wide ACL manifest added for this requires every command to be listed in permissions/allow-app-commands), `sjtu_events` (migration 0009) is replaced atomically per sync, the sidebar shows 正在上课/下一节课 with live countdown, a 10-minute-before reminder (toast + chime) fires while the app runs, and week/month/agenda merge SJTU entries in warning-amber with a 交大 tag (D24). The live jAccount login → sync path is UNVERIFIED until the user logs in once; everything else is unit-tested (63 tests) + degraded-smoke + clean startup. Next: v1.0.0 version bump → release build → zip + GitHub Release (gh CLI needs `gh auth login` first). Deferred: ICS import, layout presets, performance mode, 校历周次 display, periodic auto re-sync. Deferred: Explorer-restart persistence re-apply; live measurements (startup-to-shell, idle CPU/RAM, icon extraction, event→UI latency) and live adversarial items (multi-monitor, DPI scaling, sleep/wake, logout/login) need the user's session.

## What works (IMPLEMENTED / TESTED / WINDOWS_TESTED)

- Tauri 2 project skeleton, plain Svelte 5 + Vite 7 + TS frontend (no SvelteKit).
- Rust backend modules: `app/` (state, error, logging, shell/tray, shortcuts),
  `desktop/` (discovery, scanner, watcher, service, open, icons), `storage/`
  (SQLite, migrations 1–5, settings + desktop + collections repos),
  `commands/` (app_info, settings, shortcuts_get, desktop_list/search/rescan/
  open/icon, collections_*). cargo test 21/21, clippy clean, fmt clean.
- Design tokens (light/dark + 5 accent presets), sidebar shell, Today page,
  Settings page (theme + accent persistence through SQLite).
- **UI language: Chinese (D9, user decision)** — pages, tray menu, dates
  (`2026年9月5日星期六`), durations.
- **Tray (WINDOWS_TESTED)**: icon + menu (显示主窗口/退出), left-click toggles,
  close hides to tray. Toggle uses GetForegroundWindow.
- **Global shortcut (WINDOWS_TESTED)**: Alt+Shift+D registered at startup
  (conflict → warning + status in Settings, startup never fails).
- **Desktop index (WINDOWS_TESTED 2026-09-05)**:
  - Discovery: `FOLDERID_Desktop` → `D:\Desktop` (redirect-aware) +
    `FOLDERID_PublicDesktop` → `C:\Users\Public\Desktop`.
  - Initial sync indexed 24 items (15 files / 4 folders / 3 user shortcuts /
    2 public shortcuts), classified correctly, in single-digit ms.
  - Watcher (notify, non-recursive, 500 ms quiet debounce): created
    `dm-probe-temp.txt` on the real desktop → auto rescan logged
    `added=1` within ~2 s, DB row `missing=0`; deleted it → `removed=1`,
    row kept as `missing=1` history. No polling, no manual refresh.
  - Open: `desktop_open` allows only indexed paths; ShellExecuteW open of
    `D:\Desktop\兴趣工作` (folder) verified live (also organically exercised
    by the user double-clicking in the UI).
  - Search: UI search box "pdf" → backend `desktop_search` → filtered to
    8 PDFs, count line updates ("共 8 项"). LIKE wildcards escaped.
  - Icons (WINDOWS_TESTED): shell icons extracted on demand
    (SHGetFileInfoW → GetDIBits 32bpp top-down, AND-mask alpha fixup for
    legacy icons), backend LRU (256) + frontend canvas→PNG data-URL cache
    (512 per session); verified visually — PDF/Word/PPT/folder/shortcut
    target icons all render.
  - Frontend 桌面 page: grid with real shell icons, size/date sublines, 公用
    badges, debounced search, refresh button, `desktop:changed` listener.
  - **Virtual collections (WINDOWS_TESTED)**: collections bar (全部 / 集合
    chips / 新建集合 inline input / 移出集合 drop chip when filtered).
    Backend `collections_repo` + `collection_*` commands; assignment is
    metadata-only. Pointer drag assignment verified manually by the user
    (D13 — internal drag is pointer-based so external drops can keep the
    native Tauri channel). **External items** (shortcuts anywhere on disk)
    can be dragged in from Explorer; snapshot metadata on collection_items
    (migration 0006), live metadata wins for indexed paths (D14).
    External drag-in itself needs one manual user pass.
  - **Wallpaper (WINDOWS_TESTED)**: Settings → 自定义背景, file picker →
    base64 IPC → `background.img` in app data → `bg` custom protocol →
    fixed layer with persisted opacity slider (35%→80% live verified).
  - **Data management**: Settings → 数据管理, 清空集合 / 重置全部数据,
    two-step confirm, automatic DB backup before purge (unit-tested).
- Release build ~5.1 MB runs; MSI built. NSIS bundle failed once (network
  timeout downloading NSIS) — retry later.
- **Windows shell probe** (`probe/shell_probe`): COM `IFolderView` route
  blocked on this build (HRESULTs in WINDOWS_SHELL_PROBE.md); LVM fallback
  verified end-to-end (snapshot → move → restore, 27/27, no elevation).

## What is broken / unfinished

- **SJTU sync live path unverified** (needs the user's jAccount): open the
  交大日程 sidebar → 同步 → log in once in the popped-up window → confirm
  the countdown card fills. If the endpoint candidates all miss (the portal
  API is undocumented), capture the network request from the real page and
  add it to INIT_SCRIPT's CANDIDATES in commands/sjtu.rs.
- Explorer-restart persistence test — deferred to a dedicated session
  (snapshot before killing explorer).
- Command palette itself is M6; the global shortcut currently toggles the
  window.
- No CI yet.
- With `dragDropEnabled: false` the app no longer accepts files dragged from
  Explorer onto the window (we never used that channel; revisit only if a
  future milestone wants it).

## Next actions

1. v1.0.0: bump versions (package.json / tauri.conf.json / Cargo.toml) →
   `corepack pnpm tauri build` → zip installers → tag v1.0.0 → GitHub
   Release with notes (`gh auth login` needed before `gh release create`).
2. User live test of SJTU sync (see "What is broken / unfinished").

## Known blockers

- None blocking. (Machine quirk: never install SDKs/dev tools to C: — user
  preference. Node dir G:\nodejs is read-only for shim installs.)

## Test results log (latest first)

- 2026-09-06 (M12 SJTU integration, TESTED): 9 new unit tests — payload
  mapping (the user's captured sample: 4 personal + 1 school event, exact
  epoch-millis assertions, external-id shape, all-day heuristic for school
  day spans), bare-vs-wrapper tolerance, skipped-event counting, invalid
  JSON error, 2 MB cap, seconds/date time formats; repo replace-all
  atomicity/idempotency, ordering, clear. cargo test 63/63, clippy 0,
  svelte-check 0, eslint 0, vite build ok. Degraded browser smoke: calendar
  page renders the 交大日程 sidebar (empty state + sync button, computed
  width 264 px / radius 12 px via scoped probe); sidebar + settings 同步
  buttons fail with graceful toasts in browser mode. Real app launched once
  with the new ACL manifest: migration 0009 applied, "DesktopManager
  started", no panics, killed after 7 s. Live jAccount login → sync
  UNVERIFIED (user session).

- 2026-09-06 (ICS export, TESTED): 6 new unit tests on the serializer
  (RFC specials escaping, fold/unfold round trip incl. CJK chars,
  CRLF framing, UID/PRODID presence, all-day exclusive DTEND, blank-note
  skipping); cargo test 54/54, clippy 0, svelte-check 0, eslint 0.
  Degraded browser smoke: the 日历 导出 ICS button renders and its
  failure path shows a graceful toast without crashing (no Tauri
  backend in the plain-browser harness). A real in-app export (file
  lands in exports/, imports into another calendar app) is left for
  the user's session — TESTED, not WINDOWS_TESTED/USER_VERIFIED.

- 2026-09-06 (M9 measurement slice, MEASURED): perf_measure.rs harness
  (cargo test m9_ -- --ignored --nocapture), debug build, best-of-N.
  0/50/200/500 items: scan 0.03–0.54 ms, first sync 3.01–9.01 ms,
  re-sync 0.99–6.74 ms, list_visible 0.01–0.73 ms, search 0.02–0.19 ms,
  open incl. D22 quick_check 1.84–2.84 ms. Everything ≤ 10 ms at 500
  items → no optimization work justified. Still unmeasured: live-app
  startup, idle CPU/RAM, icon extraction, event→UI latency (user
  session). clippy 0; regular cargo test 48/48 unchanged.

- 2026-09-06 (M8 offline slice, TESTED): cargo test 48/48 (3 new recovery
  tests + 1 scale test), clippy 0. Recovery: a garbage db file is
  quarantined byte-identical and the fresh database is usable; a healthy
  db survives reopen untouched; stale -wal/-shm never keep their original
  bytes at the real path (SQLite cleans them during the failed open, or
  they are quarantined and replaced by the fresh WAL). Scale (debug
  build, temp dir): 520 items — scan 0 ms, first sync 5 ms; vanished
  files soft-remove (missing = 1) and the visible count converges.
  quick_check now runs at every startup; its cost gets measured in M9.
  A deliberate corruption drill on the real app DB is left for the
  user's session.

- 2026-09-06 (icon size, TESTED): svelte-check 0/0, eslint 0, vitest 6/6.
  Degraded browser smoke over vite dev: 设置 图标大小 小/中/大 each flips
  data-icon-size and computed --desktop-cell/--desktop-icon (185px/28px,
  230px/36px, 290px/44px); a scoped probe element confirms the desktop
  grid minmax tracks re-pack (4 → 3 columns on 大, icon 28/36/44px) and
  reload applies the attribute at startup via load(). The live smoke
  found a real bug: dataset["icon-size"] throws SyntaxError on Chromium
  ("not a valid property name", thrown after the state assignment) —
  enumPref fixed to setAttribute (D21); earlier M7 prefs were unaffected
  (dash-free names). Degraded-mode persistence is impossible by design
  (no Tauri backend): DB roundtrip and real-app visual pass left for the
  user's session, so TESTED, not WINDOWS_TESTED/USER_VERIFIED.

- 2026-09-06 (chime + custom accent, TESTED): svelte-check 0/0; eslint 0;
  vitest 6/6. Degraded browser smoke: setting the color input to #e91e63
  flips data-accent=custom with inline --accent/--accent-soft computed
  (color-mix 13%), choosing the rose preset clears the inline overrides and
  restores the preset value; the 提示音 toggle's aria-checked states flip
  both ways. The chime itself (WebAudio, focus=rising/break=falling two-tone)
  is best-effort and could not be audibly verified in a locked session —
  code path runs, resume failures are swallowed by design. In-app pass
  pending user review.

- 2026-09-06 night (month view, TESTED): svelte-check 0/0; eslint 0; vitest
  6/6. Degraded browser smoke: month renders 42 cells with 周一-first
  headers and label 2026年9月; clicking the 15th moves the agenda to
  “9月15日 周二”; 下一月 shows Oct with leading 28/29/30 dimmed;
  上一月 returns; 周 toggle restores the 7-column week grid. Persistence
  path (ui.calendarView) rides the settings table; in-app visual pass
  pending user review.

- 2026-09-05 (M7 core, TESTED): svelte-check 0/0; eslint 0; vitest 6/6
  (new resolveEnum allow-list tests). Degraded browser smoke over vite dev:
  the four new appearance controls flip `data-surface/-density/-glass/-motion`
  on <html> and the computed tokens follow live (bg #000000 under OLED dark,
  --space-2 5px compact, --glass-alpha 0.68 + blur(34px) strong, duration 0ms
  motion-off). In-app visual pass pending user review (locked session).
  Note: vite served stale transforms for timestamped module URLs while the
  session was hidden — touch + full reload clears it.

- 2026-09-05 (M6 core, TESTED): cargo test 44 passed + 2 ignored; clippy 0;
  svelte-check 0/0; eslint 0 (rewrote 4 Date-mutation sites non-mutating).
  Degraded browser smoke over vite dev (no Tauri backend): Tasks and Calendar
  pages render full structure with graceful error toasts, week range
  8月31日–9月6日 correct for 2026-09-05 (Sat), 7x24 slots, creator
  prefills 2026-09-05T09:00 from a slot click and cancels cleanly. In-app
  visual pass pending user review (machine was locked overnight).

- 2026-09-05 (UX round, partial WINDOWS_TESTED): frosted glass + Today welcome
  page verified live on the user's wallpaper (clock/motto/focus line/connected
  pill rendered); migration 0008 confirmed applied on the real DB
  (collections.parent_id present, existing row NULL). Rename / sub-collection
  chips / folder expand sit behind the web a11y tree, so interactive clicks
  are left for the user — repo logic covered by unit tests. 39 unit tests,
  clippy 0, svelte-check 0, eslint 0.

- 2026-09-05 (M3 core, WINDOWS_TESTED): `cargo test -- --ignored` 2/2 on the real desktop — live LVM read + canary roundtrip. 27 unit tests, clippy/svelte-check/eslint 0. Settings UI restore pass pending user.
- 2026-09-05 (M2 collections, WINDOWS_TESTED): collection created via inline
  input (toast, auto-switch to it, empty-state hint); filter chips with live
  item counts; 移出集合 drop chip appears only when a collection is active;
  chip 删除 button only on the active collection. HTML5 drag-drop assignment
  did not work initially — root cause: Tauri `dragDropEnabled: true` default
  installs an OLE drop handler on WebView2 that swallows in-page dragstart/
  drop; fixed by setting `dragDropEnabled: false` in tauri.conf.json and
  **verified working by the user manually**. cargo test 21/21 (4 new
  collections-repo tests incl. allow-list rejection + idempotency), clippy 0,
  fmt clean; svelte-check 0, eslint 0, vitest 3/3, vite build ok.
- 2026-09-05 (M2 extras, WINDOWS_TESTED): schema v6 migration clean. Pointer
  drag assign (count 0→1, user-verified). Wallpaper set via native dialog
  (qrcode jpg), opacity slider live 35→80, settings persisted (`ui.background`
  in settings table, `background.img` in app data), bg layer renders under
  content on all pages. cargo test 24/24, clippy 0, fmt clean; svelte-check 0,
  eslint 0, vitest 3/3, vite build ok.
- 2026-09-05 (M2 core, WINDOWS_TESTED): real `D:\Desktop` indexed (24 items,
  correct kinds, user+public sources). Watcher: create file → auto rescan
  `added=1` in <2 s; delete → `removed=1`, history row `missing=1`. Open via
  shell verified (folder `兴趣工作` opened; allow-list rejects non-indexed
  paths by construction). UI search "pdf" → 8 hits live. Chinese UI verified
  across pages incl. tray menu and date formats. cargo test 17/17, clippy 0
  warn, fmt clean; vitest 3/3, svelte-check 0, eslint 0, vite build ok.
- 2026-09-05 (M1, WINDOWS_TESTED): tray icon + menu live-verified (left-click
  toggle via Win+B → tray → Enter; close button hides to tray, process
  resident). Global shortcut Alt+Shift+D registered + toggles both directions;
  toggle root-caused to tao is_focused unreliability → GetForegroundWindow
  compare. Accent switching (violet→ocean) verified live + persisted
  (`ui.accent` in settings table). Settings shows "Alt + Shift + D" with
  green "registered" badge. Frontend 3/3 tests, svelte-check/eslint clean.
- 2026-09-05 (probe): full protocol green on real desktop — snapshot 27 →
  move (600,600)→landed (604,620, grid-snap) → verify 26+1 DIFF → restore →
  verify 27 match / 0 differ → cleanup removed both probe files.
- 2026-09-05 (probe): COM route blocked — `ShellWindows::Count()=0`
  (CLSCTX_LOCAL_SERVER and CLSCTX_ALL), `FindWindowSW` S_OK + NULL dispatch ×4
  variants. Window chain Progman>SHELLDLL_DefView>SysListView32 verified.
- 2026-09-05: release build OK; MSI built; app launches, migrations to v5,
  settings/theme persistence verified (node:sqlite inspection + log file).
- 2026-09-05: pnpm test 3/3 pass; svelte-check 0 err; eslint 0 err; vite build ok.
- 2026-09-05 (M4 core, WINDOWS_TESTED): scene create/activate/delete +
  ui.activeScene persistence exercised live in the running app; 30 unit
  tests, clippy/svelte-check/eslint 0.
- 2026-09-05 (M3/M4 manual passes, USER_VERIFIED): the user confirmed both
  outstanding manual tests passed — dragging an external shortcut from
  Explorer into a collection, and Settings 桌面布局快照 保存→应用 on the real
  desktop (layout restore now USER_VERIFIED end to end).
- 2026-09-05 (M5 core, WINDOWS_TESTED): focus start → note → complete live
  (00:34 block, note shown, ok toast, auto-break 5:00, 跳过休息 → idle);
  preset switching live; the user also ran a start→abandon block themselves
  (00:02). DB verified via node:sqlite (2 rows correct, focus.preset
  persisted). 36 unit tests, clippy 0, svelte-check 0, eslint 0.
