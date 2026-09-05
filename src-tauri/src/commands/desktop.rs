//! Desktop index commands: list, search, refresh, open.

use tauri::{AppHandle, State};

use crate::app::error::{AppError, AppResult};
use crate::app::state::{lock_db, AppState};
use crate::desktop::open::open_with_shell;
use crate::desktop::service;
use crate::storage::desktop_repo::{DesktopItem, DesktopRepo, SyncOutcome};

#[tauri::command]
pub fn desktop_list(state: State<'_, AppState>) -> AppResult<Vec<DesktopItem>> {
    let mut db = lock_db(&state)?;
    DesktopRepo::new(db.conn()).list_visible()
}

#[tauri::command]
pub fn desktop_search(state: State<'_, AppState>, query: String) -> AppResult<Vec<DesktopItem>> {
    let mut db = lock_db(&state)?;
    let repo = DesktopRepo::new(db.conn());
    if query.trim().is_empty() {
        return repo.list_visible();
    }
    repo.search(query.trim())
}

/// Manual refresh (toolbar button). Also the repair path if the watcher
/// ever misses something.
#[tauri::command]
pub fn desktop_rescan(app: AppHandle) -> AppResult<SyncOutcome> {
    service::rescan(&app)
}

/// Open an indexed desktop item via the shell. Only paths that are currently
/// present in the index may be opened — the webview cannot point this command
/// at arbitrary locations on disk.
#[tauri::command]
pub fn desktop_open(state: State<'_, AppState>, path: String) -> AppResult<()> {
    let indexed = {
        let mut db = lock_db(&state)?;
        DesktopRepo::new(db.conn()).find_visible(&path)?
    };
    if indexed.is_none() {
        return Err(AppError::Other("拒绝打开：该路径不在桌面索引中".into()));
    }
    tracing::info!(
        path,
        kind = indexed.expect("checked above").kind.as_str(),
        "opening desktop item"
    );
    open_with_shell(&path)
}
