# STATE

> Hand-off notes so a fresh agent (or human) can continue after context loss.
> Update after every significant work session. Last updated: 2026-09-05.

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
M3 core **delivered 2026-09-05**: layout snapshot/restore via the verified LVM route + canary auto-arrange guard; live read/canary passed, UI awaiting one user pass. Next: **M4 — Scenes**. Deferred: Explorer-restart persistence re-apply.

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

- NSIS installer bundle (network timeout — retry `corepack pnpm tauri build`).
- Explorer-restart persistence test — deferred to a dedicated session
  (snapshot before killing explorer).
- Programmatic auto-arrange/grid detection — canary-move check before batch
  repositions (M3).
- Command palette itself is M6; the global shortcut currently toggles the
  window.
- No CI yet.
- With `dragDropEnabled: false` the app no longer accepts files dragged from
  Explorer onto the window (we never used that channel; revisit only if a
  future milestone wants it).

## Next actions

1. M3: shell integration on the verified LVM route (see
   docs/WINDOWS_SHELL_PROBE.md + DECISIONS D8), incl. canary auto-arrange
   detection. Start with layout snapshot/restore wired to collections.
2. Retry NSIS bundle build.

## Known blockers

- None blocking. (Machine quirk: never install SDKs/dev tools to C: — user
  preference. Node dir G:\nodejs is read-only for shim installs.)

## Test results log (latest first)

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
