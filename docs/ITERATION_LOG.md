
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
