//! Desktop indexing: known-folder discovery, top-level scanning, an
//! event-driven watcher and shell-open. Classification is metadata only —
//! this module never moves, renames or deletes a user file.

pub mod browse;
pub mod discovery;
pub mod icons;
pub mod open;
pub mod scanner;
pub mod service;
pub mod watcher;

/// Desktop icon layout via LVM messages (Windows-only shell surface).
#[cfg(windows)]
pub mod shell_layout;

/// Event emitted to the frontend after the index actually changed.
pub const DESKTOP_CHANGED_EVENT: &str = "desktop:changed";
