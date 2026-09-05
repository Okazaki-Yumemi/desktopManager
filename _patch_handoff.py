import io

# 1. STATE.md — replace "Last updated" and append morning handoff section at the end
path = "docs/STATE.md"
src = io.open(path, encoding="utf-8").read()
old = "> Update after every significant work session. Last updated: 2026-09-06."
assert src.count(old) == 1

handoff = """

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

Every round passed its gates before commit: cargo test (54/54 at R18),
clippy 0, svelte-check 0, eslint 0, plus a degraded browser smoke via the
vite dev harness. **No round was verified inside the real Tauri window**
(the screen was locked all night; WebView a11y was unreachable), so treat
frontend work as TESTED — not WINDOWS_TESTED/USER_VERIFIED.

Suggested manual acceptance list (统一验收), roughly 10 minutes:

1. **外观 settings** (M7 + R14/R15): toggle 主题/强调色（含自定义取色器）/
   外观风格/密度/毛玻璃强度/动效/图标大小 — each applies instantly and
   survives an app restart (persistence goes through the settings table).
2. **桌面页**: icon size 小 changes the grid packing; 大 visibly widens
   cells to 3 columns at the usual window size.
3. **日历页**: month view navigation + click-select + dblclick-create;
   导出 ICS button → check `…/app-data/exports/calendar-*.ics` exists and
   opens in Outlook/Google Calendar (floating local times — D23).
4. **专注页**: run a 25/5 preset shortened session, let it hit the end →
   chime plays (audible? the locked screen made this unverifiable) and the
   session auto-completes.
5. **Corrupt-DB drill** (R16, optional but valuable): quit the app, write a
   few bytes of garbage into `desktopmanager.db`, restart → app starts with
   a fresh DB and `desktopmanager.db.corrupt-<ts>` holds the old bytes; the
   log records the quarantine.
6. **Release build** (R19): installer under `src-tauri/target/release/bundle/`
   if the overnight build finished — install over the existing copy and
   confirm data survives.

After acceptance, the natural next steps are in ROADMAP: M10 product polish
from real use, then the remaining M8/M9/M11 live items (idle CPU/RAM,
icon extraction, event→UI latency, multi-monitor/DPI/sleep-wake drills).
"""
src = src.replace(old, "> Update after every significant work session. Last updated: 2026-09-06 (overnight run)." + handoff)
io.open(path, "w", encoding="utf-8", newline="").write(src)
print("patched STATE.md handoff")
