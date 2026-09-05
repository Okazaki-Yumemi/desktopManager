# Iteration Log

One entry per work session: what was attempted, what changed, what was
learned, what is next. Newest at the bottom.

## 2026-09-05 — Round 1: M0 Foundation

**Environment findings (machine-specific, worth remembering):**

- Remote repo was empty; bootstrapped from scratch on `main`.
- Git Bash on this machine: MSYS `link` shadows MSVC `link.exe` (Rust builds
  fail without PATH fix) → scripts/winbuild-env.sh.
- `G:\nodejs\node_modules\npm\npmrc` has a broken `prefix=${APPDATA}\npm`;
  global npm installs pollute the project dir. Use `corepack pnpm` only.
- pnpm 11 reads build-script approvals from `pnpm-workspace.yaml`
  (`allowBuilds`), not the package.json `pnpm` field.
- Windows SDK was missing (MSVC couldn't link kernel32.lib). Per user
  requirement it must not live on C: → junction
  `C:\Program Files (x86)\Windows Kits\10` → `G:\WindowsSDK\10`
  (see DECISIONS.md D4). One UAC elevation, approved by user.

**Built:**

- Tauri 2 skeleton (plain Svelte 5 + Vite 7 + TS, no SvelteKit — D1).
- Rust: app/{state,error,logging}, storage/{db,migrations 1–5,settings repo},
  commands/{app_info,settings}. Tests for migrations + settings repo.
- Frontend: tokens + base styles, sidebar shell, Today (backend status),
  Settings (theme persisted via DB), placeholders for M2/M5/M6 pages.
  vitest + svelte-check + eslint + vite build all green.

**Next:** verify cargo test/clippy once SDK install completes, release build +
real smoke test, then IFolderView probe (docs/WINDOWS_SHELL_PROBE.md).

## 2026-09-05 — Round 2: release build, shell probe executed, desktop restored

**Built / verified:**

- cargo test 6/6, clippy clean, fmt clean (src-tauri).
- Release build ≈5.1 MB; MSI installer OK; NSIS bundle failed on a network
  timeout downloading NSIS (retry later). Release smoke: app launches, DB
  migrations to v5, theme/settings persistence verified via node:sqlite
  inspection + log file (earlier "empty log" was a race with the async writer;
  the "theme not persisting" was the user manually switching themes — both
  false alarms).
- **Shell probe executed on the real desktop** (results in
  docs/WINDOWS_SHELL_PROBE.md):
  - COM route blocked: `ShellWindows::Count()=0`, `FindWindowSW(SWC_DESKTOP)`
    returns S_OK + NULL dispatch (4 variants). HRESULTs recorded.
  - LVM fallback verified end-to-end: snapshot → move → verify(1 DIFF) →
    restore → verify(27/27) → cleanup. No elevation, files untouched.
  - Window chain: Progman > SHELLDLL_DefView (direct child) > SysListView32,
    owner = explorer.exe; LVM messages work cross-process via VirtualAllocEx /
    Read/WriteProcessMemory.

**Incident (honest record):** the first write attempt used a hand-written
message constant that was actually `LVM_SETITEMPOSITION` (0x100F), not
`LVM_SETITEMPOSITION32` (0x1071). The remote pointer was interpreted as packed
coordinates; Explorer's off-view icon rescue scrambled all 27 desktop icons
into left-edge slots. Root-caused via the windows crate's authoritative
constants, fixed, and the user's layout was fully restored from the probe
snapshot (27/27 exact). Lesson codified as DECISIONS D7 (never hand-write Win32
constants); backend strategy codified as D8 (COM first-try, LVM fallback).

**Machine notes:** desktop listview client coords are the canonical position
space (MapWindowPoints no-ops cross-process from a DPI-unaware console);
virtual-screen origin (-2560,-559) must be stored alongside positions.

**Next:** commit + push (first push to the remote), then M1 — tray, global
shortcut, custom titlebar.
