
## 2026-09-05 — Round 5: M2 virtual collections + drag assignment (M2 complete)

**Shipped:**

- `collections_repo` (list with item counts / create / rename / delete /
  assign / unassign / items-of-collection) + 7 `collection_*` commands.
  Assignment is metadata-only and allow-listed to indexed paths (same policy
  as open/icon); delete removes assignments explicitly so it holds regardless
  of the foreign_keys pragma.
- DesktopPage: collections bar — 全部 chip, per-collection chips (color dot +
  live count), inline 新建集合 input (Enter/blur submit, Esc cancel, palette
  color rotation), per-collection 删除 button, 移出集合 drop chip while a
  collection is active. HTML5 dragstart on item cards, drop targets on chips;
  toast feedback; collection view filters the grid (search box narrows it
  client-side).

**Debugged for real:** drag assignment was dead in the running app. Synthetic
drags were not the cause — the user confirmed real drags no-oped too. Root
cause: Tauri 2's `dragDropEnabled: true` default puts a native OLE drop
handler on WebView2 that swallows in-page HTML5 dragstart/drop on Windows.
`dragDropEnabled: false` in tauri.conf.json fixed it (D12); **the user
manually verified drag-drop works**.

Tests: cargo test 21/21 (4 new repo tests: roundtrip+order, allow-list
rejection, duplicate names + rename, delete cascade + missing hidden),
clippy 0, fmt clean; svelte-check 0, eslint 0, vitest 3/3, vite build ok.

**Next:** M3 shell integration on the verified LVM route (snapshot/restore
wired to collections; canary auto-arrange detection before batch moves).

## 2026-09-05 — Round 3: M2 core (Desktop Index) + Chinese UI

**Shipped:**

- UI language switched to Chinese per user decision (D9): all pages, tray
  menu (显示主窗口/退出), `zh-CN` dates/durations, `datetime` tests updated.
- `desktop/` module:
  - `discovery.rs` — SHGetKnownFolderPath (FOLDERID_Desktop → `D:\Desktop`
    redirect honored + FOLDERID_PublicDesktop), env fallback, same-dir dedupe.
  - `scanner.rs` — top-level scan; kind = folder / shortcut (.lnk/.url) /
    file; hidden+system entries skipped (crate FILE_ATTRIBUTE_* constants);
    display_name rule per D10.
  - `desktop_repo.rs` — `sync_scan` upserts + `missing=1` history in ONE
    transaction; `list_visible`, `search` (LIKE-escaped), `find_visible`.
  - `watcher.rs` — notify recommended watcher (non-recursive), 500 ms
    quiet-period debounce, self-reconnecting thread; rescan → emit
    `desktop:changed` only on real change (D11).
  - `open.rs` — ShellExecuteW; commands validate the path is currently
    indexed before opening (webview cannot open arbitrary paths).
- DesktopPage: real grid (lucide icons by kind, size/date, 公用 badge),
  debounced backend search, refresh button, event-driven reload, Chinese
  empty/error states.
- `@lucide/svelte` added (first icon usage).

**Verified live (WINDOWS_TESTED):** 24 real items indexed (15/4/3 user +
2 public); create/delete of `dm-probe-temp.txt` on `D:\Desktop` auto-synced
(`added=1` then `removed=1`, history row kept); folder open via UI worked;
UI search "pdf" → 8 hits; Chinese date "2026年9月5日星期六". cargo test 14/14
(7 new: discovery, scanner ×3, repo ×3), clippy 0 warn, fmt clean; vitest
3/3, svelte-check 0, eslint 0, vite build ok.

**Learned:**

- windows 0.61: `SHGetKnownFolderPath(&GUID, KNOWN_FOLDER_FLAG(0), None)
  -> Result<PWSTR>` (3 args, returns the buffer; free with CoTaskMemFree);
  constants are PascalCase `FOLDERID_Desktop`/`FOLDERID_PublicDesktop`.
- `Connection::unchecked_transaction` allows `&self` repos (query_row's
  `&mut self` on prepared statements needs the raw conn, not a `tx()`).
- notify 8 API: `recommended_watcher(closure)` + `watch(path, NonRecursive)`;
  only the signal matters — the rescan reads reality from disk.
- Element `class:` directives are invalid on Svelte 5 components — pass
  `class={cond ? "x" : ""}` to lucide icons instead.

**Next:** M2 remainder (lazy bounded icon cache; collections + drag/drop),
then M3 shell integration on the LVM route.
in: Progman > SHELLDLL_DefView (direct child) > SysListView32,
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

## 2026-09-05 — Round 3: first push + M1 Application Shell

**Shipped:**

- First push to `github.com:Okazaki-Yumemi/desktopManager` (`7fa6135`, 86
  files): full M0 foundation + verified probe + docs.
- **M1 Application Shell** (WINDOWS_TESTED on the real desktop):
  - Tray icon + menu (Show DesktopManager / Quit); left-click toggles the
    window; the window close button hides to tray instead of quitting
    (resident tool behavior, logged).
  - Global shortcut `Alt+Shift+D` (command-palette binding, toggles the
    window until the palette exists in M6). Registration is conflict-tolerant:
    a taken binding logs a warning and shows in Settings; startup never fails.
  - Theme system completion: 5 accent presets (ocean default) applied via
    `data-accent` attribute + tokens.css cascade, persisted in settings.
  - Settings: accent swatches, global-shortcut card with live registration
    status (green "registered" badge verified live).
  - Today: quick-start card (shortcut hint + close-to-tray explanation).
  - New command `shortcuts_get`; `AppState.shortcut_status`.

**Learned:**

- tao's `Window::is_focused()` tracks internal focus state and missed the
  focused case with WebView2's child-window focus — the toggle always took
  the "show" branch. Fixed by comparing `GetForegroundWindow()` against the
  window HWND (`windows` crate dep added for src-tauri, Windows-only).
- tauri-plugin-global-shortcut 2.3: plugin is `Builder::new().build()`, the
  manager extension trait is `GlobalShortcutExt`, per-shortcut registration
  via `app.global_shortcut().on_shortcut(...)` in setup.
- eslint flagged 2 latent issues in M0 toast code (`let`→`const` for runes
  state, unused type import) — fixed; the rest of the suite stays green.

**Test evidence:** log shows `global shortcut registered binding="alt+shift+d"`,
toggle debug lines (`visible=true foreground=false` → show, then
`foreground=true` → hide), `main window hidden to tray (close requested)`;
settings table contains `ui.theme="system"`, `ui.accent="ocean"` after live
switching. cargo test 6/6, clippy clean, fmt clean; vitest 3/3, svelte-check 0,
eslint 0, vite build ok.

**Next:** M2 — Desktop Index (user+public desktop discovery with redirect
awareness, debounced watcher, `desktop_items` indexing, open via shell,
search).

## 2026-09-05 — Round 4: M2 icon cache (lazy, bounded)

**Shipped:**

- `desktop/icons.rs`: on-demand shell icon extraction — SHGetFileInfoW
  (SHGFI_ICON|SHGFI_LARGEICON) → GetIconInfo → GetDIBits 32bpp top-down
  (negative biHeight), BGRA→RGBA swap, AND-mask alpha fixup for legacy
  icons; backend LRU cache (256 entries, FIFO-evicting with recency
  refresh); payload = dimensions + base64 RGBA over IPC. `desktop_icon`
  command, allow-listed to indexed paths like `desktop_open`.
- Frontend: `iconCache.ts` (bounded Map 512, recency-refreshing,
  canvas→PNG data-URL encoding — no image crate in the backend) and
  `DesktopIcon.svelte` (snippet-based generic-glyph fallback).
- deps: base64 0.22, windows feature Win32_Graphics_Gdi.

**Verified live (WINDOWS_TESTED):** real desktop grid shows authentic
shell icons — red PDF badges, Word/PPT icons, yellow folders, Zotero /
ZCode / Battlestate / 此电脑 shortcut icons resolved to their targets.
cargo test 17/17 (new: LRU eviction, real notepad.exe extraction,
missing-path→None), clippy 0, fmt clean; svelte-check 0, eslint 0,
vite build ok, vitest 3/3.

**Learned:**

- HICON pixel reading needs a masked DC dance: GetIconInfo gives
  hbmColor+hbmMask; read the color bitmap with a negative-height
  BITMAPINFOHEADER (top-down); legacy icons carry alpha=0 everywhere and
  rely on the 1bpp AND mask (bit 0 = opaque) — fix up per pixel.
- windows 0.61: DeleteObject/GetObjectW take HGDIOBJ — wrap HBITMAP as
  HGDIOBJ(h.0); SHGetFileInfoW returns BOOL-as-usize (0 = fail);
  BITMAPINFOHEADER.biBitCount is u16.
- base64 string length ≠ byte length (the first test assertion compared
  5464 chars against 4096 bytes — caught immediately by the test).
- Dev loop note: tauri dev auto-rebuilds + restarts the app on Rust
  changes and Vite HMRs the webview; the ZCode crash killed the whole
  dev process tree (job objects), a plain relaunch is enough.

**Next:** M2 remainder — virtual collections + drag/drop assignment —
then M3 shell integration on the LVM route.
