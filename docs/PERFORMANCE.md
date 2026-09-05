# Performance

> Rule: only numbers measured on this machine go in here. Guesswork is not
> performance data. Update after every resource review (charter §47).

## Baseline (M0) — PENDING MEASUREMENT

To be recorded with the first release build smoke test:

| Metric                          | Value | Notes                                |
| ------------------------------- | ----- | ------------------------------------ |
| Startup to usable shell (ms)    | —     | cold start, release build            |
| Idle CPU (%)                    | —     | 60s sample, main window open + tray  |
| Idle RAM, whole process tree    | —     | DesktopManager.exe + all WebView2 children |
| Binary size / installer size    | —     | release build artifacts              |
| Disk writes at idle             | —     | should be ~0 besides log rotation    |

Machine: Windows 11 build 26200, x64 — full specs to be recorded with first
measurement.

## Measured — scan & DB queries at scale (M9, 2026-09-06, MEASURED)

Source: `src-tauri/src/perf_measure.rs`
(`cargo test m9_ -- --ignored --nocapture` in src-tauri). Best-of-N wall
time — N=3 for scan/sync/open, N=5 for list/search. **Debug build**, so
these are upper bounds used to catch order-of-magnitude problems, not a
release claim. Temp dir with `文件_%03d.{txt,lnk,png}` files, temp-file DB
(WAL mode), sync-first is single-shot because it mutates.

| items | scan (FS) ms | sync first ms | sync re-run ms | list_visible ms | search ms | open (pragmas + quick_check + migrations) ms |
| ----: | -----------: | ------------: | -------------: | --------------: | --------: | -------------------------------------------: |
| 0     | 0.03         | 3.01          | 0.99           | 0.01            | 0.02      | 1.89                                          |
| 50    | 0.07         | 3.48          | 1.32           | 0.07            | 0.02      | 1.84                                          |
| 200   | 0.22         | 5.56          | 2.99           | 0.29            | 0.04      | 2.84                                          |
| 500   | 0.54         | 9.01          | 6.74           | 0.73            | 0.19      | 2.01                                          |

Conclusion: every measured path stays under 10 ms at 500 items even in a
debug build. The D22 startup `PRAGMA quick_check` lives inside the "open"
column (~2–3 ms) — negligible, as predicted when it was introduced. **No
optimization is justified by these numbers** (charter: optimize only what
measurement justifies). The release profile (opt-level "s" + LTO) can only
be faster.

Still unmeasured (need the live app / real session): startup-to-usable-shell,
idle CPU/RAM (whole process tree incl. WebView2), shell-icon extraction,
fs-event → UI latency, installer size. The watcher is Tauri-coupled and its
headless burst behaviour is approximated here by scan + sync timing.

## Current cost controls already in place

- No timers/periodic polling anywhere in the frontend or backend yet.
- Logging is level-gated and rotated; release default is info.
- `panic = "abort"`, `opt-level = "s"`, LTO, stripped release profile.
- No heavy UI framework; single accent CSS; no runtime CSS-in-JS.

## Process notes

- Idle CPU target ≈ 0%: event-driven only (fs watcher, window events, DB
  events). `setInterval`-driven UI refreshes are forbidden.
- Memory claims must include WebView2 child processes, not just the main exe.
