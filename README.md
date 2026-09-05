# DesktopManager

> Lightweight personal desktop workspace manager for Windows.

DesktopManager helps you see what you need and get into what you should be
doing when you sit down at your PC — desktop organization, workspace scenes,
focus sessions, tasks, calendar and a daily overview — while staying light on
resources and 100% local.

**Status:** early foundation (Milestone M0). See [docs/STATE.md](docs/STATE.md)
and [docs/ROADMAP.md](docs/ROADMAP.md).

## Product principles

- **Non-destructive by default.** Desktop "classification" is metadata stored
  in a local database. Your files are never moved unless you explicitly ask
  for it (and even then, with preview + undo).
- **Lightweight.** Near-zero idle CPU, no bundled Chromium (uses the system
  WebView2), lazy loading, event-driven indexing.
- **Local & private.** No telemetry, no analytics, no cloud. By default.

## Tech stack

- [Tauri 2](https://tauri.app) + Rust (backend, Windows shell integration)
- Svelte 5 + TypeScript (frontend, no heavyweight UI framework)
- SQLite with versioned schema migrations (single source of truth)

## Development

Prerequisites: Node.js 20+, pnpm 11 (via corepack), Rust stable (MSVC),
Windows 10/11 with WebView2.

```bash
corepack pnpm install
corepack pnpm tauri dev      # run the desktop app in dev mode
corepack pnpm test           # frontend unit tests (vitest)
corepack pnpm check          # svelte-check
corepack pnpm lint           # eslint

cd src-tauri
cargo fmt --check
cargo clippy
cargo test                   # Rust unit tests
```

## Documentation

| Doc                                             | Purpose                                   |
| ----------------------------------------------- | ----------------------------------------- |
| [docs/STATE.md](docs/STATE.md)                   | Current state; hand-off notes for agents  |
| [docs/ROADMAP.md](docs/ROADMAP.md)               | Milestones M0–M11                         |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)     | Module layout & data flow                 |
| [docs/DECISIONS.md](docs/DECISIONS.md)           | Decision log (ADR-style)                  |
| [docs/ITERATION_LOG.md](docs/ITERATION_LOG.md)   | What changed each iteration               |
| [docs/WINDOWS_SHELL_PROBE.md](docs/WINDOWS_SHELL_PROBE.md) | Windows desktop shell experiments |
| [docs/UI_DESIGN.md](docs/UI_DESIGN.md)           | Design tokens & visual direction          |
| [docs/PERFORMANCE.md](docs/PERFORMANCE.md)       | Measured resource usage                   |

## Privacy

No telemetry by default. All data stays in `%APPDATA%\com.okazakiyumemi.desktopmanager`.
