
## D12 — `dragDropEnabled: false` on the main window (accepted, 2026-09-05)

Tauri's default (`dragDropEnabled: true`) installs its own OLE drop target on
WebView2 to expose external file drops as Tauri events — and in doing so it
**swallows in-page HTML5 dragstart/drop**, which made collection drag
assignment dead on Windows (verified: synthetic drags and the user's real
drags both no-oped). We never consume external file drops, so the default
only cost us the feature we do use. Flipped to `false`; HTML5 DnD now works.
Trade-off: dragging files from Explorer onto the window does nothing (by
design until some milestone asks for it).
I code. tauri-plugin-sql exposes
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
