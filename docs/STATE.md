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

M0 — Foundation and **M1 — Application Shell: COMPLETE and verified**.
Next: **M2 — Desktop Index** (known-folder discovery, debounced watcher,
indexing, open via shell; fallback-first, never move files).

## What works (IMPLEMENTED / TESTED / WINDOWS_TESTED)

- Tauri 2 project skeleton, plain Svelte 5 + Vite 7 + TS frontend (no SvelteKit).
- Rust backend modules: `app/` (state, error, logging, shell/tray, shortcuts),
  `storage/` (SQLite, migrations 1–5, settings repo), `commands/` (app_info,
  settings get/set, shortcuts_get). cargo test 6/6, clippy clean, fmt clean.
- Design tokens (light/dark + 5 accent presets), sidebar shell, Today page
  (backend status card + quick-start card), Settings page (theme + accent
  persistence round-trip through SQLite, verified via node:sqlite inspection).
- **Tray (WINDOWS_TESTED)**: icon + menu (Show/Quit), left-click toggles the
  window, close button hides to tray (process stays resident, log confirms).
- **Global shortcut (WINDOWS_TESTED)**: Alt+Shift+D registered at startup
  (conflict → warning + status in Settings, startup never fails); toggles
  show/hide verified live in both directions. Toggle uses
  GetForegroundWindow (tao's is_focused missed WebView2 focus).
- Release build `src-tauri/target/release/desktop-manager.exe` (≈5.1 MB) runs;
  MSI installer built. NSIS bundle FAILED once (`timeout: global` while
  downloading NSIS from github — network flake, retry later; MSI is fine).
- **Windows shell probe** (`probe/shell_probe`): COM `IFolderView` route is
  blocked on this build (ShellWindows collection empty — HRESULTs recorded in
  docs/WINDOWS_SHELL_PROBE.md); the LVM/SysListView32 fallback **verified
  end-to-end**: snapshot 27 icons → move one (write proof: verify 1 differ) →
  restore → verify 27/27 exact → cleanup. No elevation, no file moves.

## What is broken / unfinished

- NSIS installer bundle (network timeout — retry `corepack pnpm tauri build`).
- Explorer-restart persistence (re-apply positions after killing explorer) —
  deferred to a dedicated session; snapshot before killing explorer.
- Programmatic auto-arrange/grid detection (currently read from the context
  menu); needs a canary-move check before batch repositions (M3).
- Command palette itself is M6; the global shortcut currently toggles the
  window. Shortcut re-binding UI lands with the palette (M6/M7).
- No CI yet.

## Next actions

1. Commit + push M1.
2. M2: desktop index — enumerate `D:\Desktop` (user + public, redirect-aware)
   via SHGetKnownFolderPath + walk; index into `desktop_items`; debounced
   filesystem watcher; open item via shell; search. Fallback-first.
3. M3: shell integration built on the verified LVM route (see
   docs/WINDOWS_SHELL_PROBE.md + DECISIONS D8).

## Known blockers

- None blocking. (Machine quirk: never install SDKs/dev tools to C: — user
  preference. Node dir G:\nodejs is read-only for shim installs.)

## Test results log (latest first)

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
