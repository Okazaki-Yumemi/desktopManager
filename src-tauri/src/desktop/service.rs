//! Scan orchestration shared by startup, the watcher thread and the manual
//! refresh command. Emits [`DESKTOP_CHANGED_EVENT`] only when the index
//! actually changed, so an idle desktop stays silent.

use std::time::Instant;

use tauri::{AppHandle, Emitter, Manager};

use super::{scanner, watcher, DESKTOP_CHANGED_EVENT};
use crate::app::error::AppResult;
use crate::app::state::{lock_db, AppState};
use crate::storage::desktop_repo::{DesktopRepo, SyncOutcome};

/// Scan all desktop sources, sync the index and notify the UI on change.
pub fn rescan(app: &AppHandle) -> AppResult<SyncOutcome> {
    let started = Instant::now();
    let state = app.state::<AppState>();

    let mut items = Vec::new();
    for source in &state.desktop_sources {
        items.extend(scanner::scan_desktop_dir(&source.root, source.source));
    }

    let outcome = {
        let mut db = lock_db(&state)?;
        DesktopRepo::new(db.conn()).sync_scan(&items)?
    };

    tracing::debug!(
        elapsed_ms = started.elapsed().as_millis() as u64,
        total = items.len(),
        added = outcome.added,
        updated = outcome.updated,
        removed = outcome.removed,
        "desktop index synced"
    );
    if outcome.changed() {
        let _ = app.emit(DESKTOP_CHANGED_EVENT, outcome);
    }
    Ok(outcome)
}

/// Startup hook: build the first index synchronously (so the first UI load
/// already sees it) and start the event-driven watcher. Index problems never
/// block startup — the app stays usable without the index (fallback-first).
pub fn init(app: &AppHandle) {
    match rescan(app) {
        Ok(outcome) => {
            tracing::info!(added = outcome.added, "initial desktop index built");
        }
        Err(err) => {
            tracing::warn!(%err, "initial desktop scan failed — retrying on fs events");
        }
    }
    watcher::spawn(app.clone());
}
