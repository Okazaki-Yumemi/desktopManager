# Windows Shell Probe — desktop icon positions

Goal: verify, on a real Windows 11 desktop, whether documented shell
interfaces can **read and write the actual Explorer desktop icon positions
without touching the files on disk**. This is the foundation for Strategy A
(shell-integrated collections) in the charter.

Status: **EXECUTED 2026-09-05 — read/write/restore round-trip VERIFIED on this
machine via the LVM route. The documented COM (`IFolderView`) route is blocked
on this machine/build and is recorded below with real HRESULTs; nothing is
faked.** Probe source: `probe/shell_probe` (standalone crate, `diag`/`diag2`
subcommands included for reproducibility).

## Test environment (this machine, at probe time)

- Windows 11 build 26200 (x64), explorer.exe pid 13556.
- Two displays: main 2560x1600 @ virtual (0,0); secondary 2560x1440 @
  virtual (-2560,-559). Virtual-screen origin = (-2560, -559).
- Known-folder desktop (FOLDERID_Desktop) is redirected to `D:\Desktop`
  (probe reports it correctly).
- 27 desktop icons at test time (Chinese + English names).
- Desktop view state (read from the Explorer context menu during the session):
  自动排列图标 (auto-arrange) = **off**, 将图标与网格对齐 (align to grid) =
  **on**, 显示桌面图标 (show desktop icons) = **on**.

## Route 1 — documented COM path: BLOCKED on this machine (recorded)

Path attempted: `CoCreateInstance(CLSID_ShellWindows)` →
`FindWindowSW(SWC_DESKTOP)` → `IDispatch` → `IServiceProvider` →
`QueryService(SID_STopLevelBrowser)` → `IShellBrowser` → `IFolderView`.

Failure table (template: date | windows build | step | api | HRESULT | symptom | conclusion):

```
2026-09-05 | 26200 | create   | CoCreateInstance(ShellWindows, CLSCTX_LOCAL_SERVER) | S_OK      | object created            | activation works
2026-09-05 | 26200 | create   | CoCreateInstance(ShellWindows, CLSCTX_ALL)          | S_OK      | object created            | activation works
2026-09-05 | 26200 | enumerate| IShellWindows::Count()                              | S_OK      | returns 0 (both contexts) | collection EMPTY on this machine
2026-09-05 | 26200 | locate   | FindWindowSW(VT_UI4 SWC_DESKTOP, flags=0)           | S_OK      | NULL dispatch returned    | desktop window not registered
2026-09-05 | 26200 | locate   | FindWindowSW(VT_UI4 SWC_DESKTOP, SWFO_NEEDDISPATCH) | S_OK      | NULL dispatch returned    | desktop window not registered
2026-09-05 | 26200 | locate   | FindWindowSW(VT_I4  SWC_DESKTOP, flags=0/1)         | S_OK      | NULL dispatch returned    | same, not a VARIANT-type issue
```

Conclusion: `CLSID_ShellWindows` activates, but its collection contains zero
windows on this build/config — the desktop browser is not registered there, so
`IFolderView` cannot be obtained through the documented route. Reproduce with
`shell_probe diag` / `diag2`. Re-test on future Windows updates; the probe
tries this route first at every run.

## Route 2 — LVM message fallback: WORKS (verified)

Window chain found (probe `diag2`):

```
Progman (0x10236)
└─ SHELLDLL_DefView (0x10240)      <- direct child of Progman on this machine
   └─ SysListView32 (0x10242)      <- owner pid 13556 = explorer.exe
```

- A top-level search for `SHELLDLL_DefView` and searches under `WorkerW`
  windows return `0x80070006` (E_HANDLE) — that is windows-rs' conversion of a
  NULL `FindWindowExW` return ("not found"), not a real handle error.
- `SendMessageW(listview, LVM_GETITEMCOUNT)` cross-process → 27. No elevation
  required, same desktop, medium integrity.

Implementation (probe): per message, allocate a page in explorer.exe with
`VirtualAllocEx` (MEM_RESERVE|MEM_COMMIT, PAGE_READWRITE), write inputs with
`WriteProcessMemory`, `SendMessageW`, read results with `ReadProcessMemory`,
free with `VirtualFreeEx`. Read = `LVM_GETITEMCOUNT`, `LVM_GETITEMPOSITION`
(remote POINT), `LVM_GETITEMTEXTW` (remote LVITEMW + text buffer; Chinese
names round-trip correctly). Write = `LVM_SETITEMPOSITION32` (remote POINT,
client coords).

### The 0x100F incident (recorded per "never fake success")

The first write attempt used a hand-written constant `LVM_FIRST+15` believed to
be `LVM_SETITEMPOSITION32`. It is actually **`LVM_SETITEMPOSITION`** (packed
x/y in lParam); the real `LVM_SETITEMPOSITION32` is **0x1071**. The pointer to
the remote POINT was therefore interpreted as packed coordinates → each icon
landed at pseudo-random coordinates → Explorer's off-view icon rescue cascade
collected all 27 icons into free grid slots at the left edge of the secondary
display. The user's layout was fully restored afterwards from the probe
snapshot (27/27 exact) once the constant was fixed — and every future probing
session must start with a snapshot for exactly this reason.

**Lesson (DECISIONS D7): never hand-write Win32 message/flag constants; import
them from the `windows` crate so the compiler and the crate are the single
source of truth.**

## Verified round-trip (fixed binary, 2026-09-05)

```
prepare    -> created D:\Desktop\dm-probe-1.txt / dm-probe-2.txt
snapshot   -> 27 items (route LVM)
move dm-probe-1.txt -> requested (600,600), landed (604,620)
              (align-to-grid snapped ~4/20 px; write accepted)
verify     -> 26 match, 1 differ (the moved icon) = write proof
restore    -> 27 positioned, 0 missing
verify     -> 27 match, 0 differ, 0 missing  (exit 0)
cleanup    -> both probe files removed
```

Success criteria status:

- Read positions for ≥95% of items: **PASS (27/27, including CJK names).**
- Write + restore round-trip exact: **PASS on a live session** (grid-align
  snapping respected by restoring onto grid-aligned coordinates).
- Auto-arrange / align-to-grid detectable: partially — currently read from the
  context menu during development; programmatic detection is still open (the
  COM route would give `IFolderView2::GetViewModeAndAutoArrangeFlags`; on
  machines where COM is blocked we need an alternative, e.g. a canary move +
  read-back before any batch reposition).
- Survive Explorer restart via re-apply: NOT yet tested (deferred; requires
  killing explorer.exe — do it in a dedicated session, snapshot first).

## Coordinate-space notes (important for M3)

- LVM positions are **SysListView32 client coordinates** (physical pixels in
  explorer's space). Treat them as the canonical stored value for a given
  monitor layout; snapshots must also record the virtual-screen origin so
  positions can be re-anchored when monitors change.
- `MapWindowPoints(listview → screen)` from the (DPI-unaware) console probe
  returned points **unchanged** — cross-process + DPI-virtualization quirk.
  The probe therefore stores client coordinates and does not rely on
  MapWindowPoints. DesktopManager must be a PerMonitorV2-aware process and
  must anchor client↔screen conversion via the listview window rect, tested
  again then.
- On this machine the original icon layout lived on the main display: client
  x 2593..5008 ⇒ screen x 33..2448 (client = screen − virtual origin).

## Risks / notes

- `WorkerW` embedding for group backgrounds is a separate, riskier probe —
  NOT bundled here; deferred per charter §11.
- The probe never moves real user files; it only repositions icons, and it
  snapshots before every mutating command.
- If "自动排列图标" (auto-arrange) is enabled, position writes are overridden
  by the shell (observed behavior class; auto-arrange was off during the
  verified run). The app must detect and refuse/warn rather than fight the
  shell (canary-move check before batch ops).
