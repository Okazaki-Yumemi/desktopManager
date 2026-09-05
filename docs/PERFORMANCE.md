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

## Planned measurement scenarios (M2+, desktop indexing)

Desktop item counts: 0 / 50 / 200 / 500. Track: initial index time, event→UI
latency after a file change, steady-state memory.

## Current cost controls already in place

- No timers/periodic polling anywhere in the frontend or backend yet.
- Logging is level-gated and rotated; release default is info.
- `panic = "abort"`, `opt-level = "s"`, LTO, stripped release profile.
- No heavy UI framework; single accent CSS; no runtime CSS-in-JS.

## Process notes

- Idle CPU target ≈ 0%: event-driven only (fs watcher, window events, DB
  events). `setInterval`-driven UI refreshes are forbidden.
- Memory claims must include WebView2 child processes, not just the main exe.
