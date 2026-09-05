//! Event-driven desktop watcher: ReadDirectoryChangesW under the hood, no
//! polling. Filesystem changes are debounced — a burst (copying 20 files)
//! triggers exactly one rescan after the folder stays quiet for a moment.

use std::time::Duration;

use notify::{RecursiveMode, Watcher};
use tauri::{AppHandle, Manager};

use super::service;

/// Quiet period before a rescan. Small enough to feel instant, long enough
/// to swallow copy storms.
pub const QUIET_PERIOD: Duration = Duration::from_millis(500);

/// Spawn the watcher thread. It lives for the whole app lifetime and
/// reconnects itself if the OS watcher ever fails.
pub fn spawn(app: AppHandle) {
    std::thread::Builder::new()
        .name("desktop-watcher".into())
        .spawn(move || run(app))
        .expect("spawn desktop watcher thread");
}

fn run(app: AppHandle) {
    loop {
        match watch_until_failure(&app) {
            Ok(()) => return, // channel closed only when the app is shutting down
            Err(err) => {
                tracing::warn!(%err, "desktop watcher failed, reconnecting in 5s");
                std::thread::sleep(Duration::from_secs(5));
            }
        }
    }
}

fn watch_until_failure(app: &AppHandle) -> notify::Result<()> {
    let sources = app
        .state::<crate::app::state::AppState>()
        .desktop_sources
        .clone();

    let (tx, rx) = std::sync::mpsc::channel::<()>();
    let mut watcher = notify::recommended_watcher(move |res| match res {
        Ok(_) => {
            // Only the signal matters; the rescan reads reality from disk.
            let _ = tx.send(());
        }
        Err(err) => tracing::debug!(%err, "unusable fs event ignored"),
    })?;

    for source in &sources {
        watcher.watch(&source.root, RecursiveMode::NonRecursive)?;
        tracing::debug!(dir = %source.root.display(), "watching desktop folder");
    }

    loop {
        match rx.recv() {
            Ok(()) => {}
            Err(_) => return Ok(()), // sender gone → shutting down
        }
        // Drain until the folder has been quiet for QUIET_PERIOD.
        loop {
            match rx.recv_timeout(QUIET_PERIOD) {
                Ok(()) => continue,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => break,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
            }
        }
        if let Err(err) = service::rescan(app) {
            tracing::warn!(%err, "desktop rescan after fs event failed");
        }
    }
}
