//! Desktop index commands: list, search, refresh, open, icons.

use serde::Serialize;
use tauri::{AppHandle, State};

use crate::app::error::{AppError, AppResult};
use crate::app::state::{lock_db, AppState};
use crate::desktop::icons::{extract_cached, IconPayload};
use crate::desktop::open::open_with_shell;
use crate::desktop::service;
use crate::storage::collections_repo::{Collection, CollectionsRepo};
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

/// Shell icon for an indexed item as base64 RGBA (`None` → UI shows a glyph).
/// Restricted to indexed paths, like `desktop_open`.
#[tauri::command]
pub fn desktop_icon(state: State<'_, AppState>, path: String) -> AppResult<Option<IconPayloadDto>> {
    let indexed = {
        let mut db = lock_db(&state)?;
        DesktopRepo::new(db.conn()).find_visible(&path)?
    };
    if indexed.is_none() {
        return Ok(None);
    }
    Ok(extract_cached(&path)?.map(|p: IconPayload| IconPayloadDto {
        width: p.width,
        height: p.height,
        rgba: p.rgba,
    }))
}

/// Serde-facing mirror of `IconPayload` (camelCase).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IconPayloadDto {
    pub width: i32,
    pub height: i32,
    pub rgba: String,
}

// --- Virtual collections (metadata only; never touches real files) ---

#[tauri::command]
pub fn collections_list(state: State<'_, AppState>) -> AppResult<Vec<Collection>> {
    let mut db = lock_db(&state)?;
    CollectionsRepo::new(db.conn()).list()
}

#[tauri::command]
pub fn collection_create(
    state: State<'_, AppState>,
    name: String,
    color: String,
) -> AppResult<Collection> {
    let mut db = lock_db(&state)?;
    let created = CollectionsRepo::new(db.conn()).create(&name, &color)?;
    tracing::info!(id = created.id, name = %created.name, "collection created");
    Ok(created)
}

#[tauri::command]
pub fn collection_rename(state: State<'_, AppState>, id: i64, name: String) -> AppResult<()> {
    let mut db = lock_db(&state)?;
    CollectionsRepo::new(db.conn()).rename(id, &name)
}

#[tauri::command]
pub fn collection_delete(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    let mut db = lock_db(&state)?;
    CollectionsRepo::new(db.conn()).delete(id)?;
    tracing::info!(id, "collection deleted");
    Ok(())
}

/// Assign an indexed item to a collection. Returns whether it was new.
#[tauri::command]
pub fn collection_assign(state: State<'_, AppState>, id: i64, path: String) -> AppResult<bool> {
    let mut db = lock_db(&state)?;
    let created = CollectionsRepo::new(db.conn()).assign(id, &path)?;
    if created {
        tracing::info!(collection_id = id, path, "item assigned to collection");
    }
    Ok(created)
}

#[tauri::command]
pub fn collection_unassign(state: State<'_, AppState>, id: i64, path: String) -> AppResult<bool> {
    let mut db = lock_db(&state)?;
    CollectionsRepo::new(db.conn()).unassign(id, &path)
}

#[tauri::command]
pub fn collection_items(state: State<'_, AppState>, id: i64) -> AppResult<Vec<DesktopItem>> {
    let mut db = lock_db(&state)?;
    CollectionsRepo::new(db.conn()).items(id)
}
