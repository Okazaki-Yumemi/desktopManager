//! Desktop index commands: list, search, refresh, open, icons.

use serde::Serialize;
use tauri::{AppHandle, State};

use crate::app::error::{AppError, AppResult};
use crate::app::state::{lock_db, AppState};
use crate::desktop::browse::{list_children, PathEntry};
use crate::desktop::icons::{extract_cached, IconPayload};
use crate::desktop::open::open_with_shell;
use crate::desktop::service;
use crate::storage::collections_repo::{Collection, CollectionsRepo};
use crate::storage::desktop_repo::{DesktopItem, DesktopRepo, SyncOutcome};
use crate::storage::Database;

/// Whether a path may be opened, iconized or browsed: it is visible in the
/// desktop index, held by a collection, or lives inside such a directory
/// (children of an expanded folder reference — D14 extended for browsing).
fn path_allowed(db: &mut Database, path: &str) -> AppResult<bool> {
    let mut cur = std::path::Path::new(path).to_path_buf();
    loop {
        let probe = cur.to_string_lossy().into_owned();
        let allowed = {
            let conn = db.conn();
            let indexed = DesktopRepo::new(&*conn).find_visible(&probe)?;
            let held = CollectionsRepo::new(&*conn).holds_path(&probe)?;
            indexed.is_some() || held
        };
        if allowed {
            return Ok(true);
        }
        match cur.parent() {
            Some(parent) => cur = parent.to_path_buf(),
            None => return Ok(false),
        }
    }
}

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

/// Open an indexed desktop item via the shell. Only paths allowed by the
/// desktop index / collections policy may be opened — the webview cannot
/// point this command at arbitrary locations on disk.
#[tauri::command]
pub fn desktop_open(state: State<'_, AppState>, path: String) -> AppResult<()> {
    let allowed = {
        let mut db = lock_db(&state)?;
        path_allowed(&mut db, &path)?
    };
    if !allowed {
        return Err(AppError::Other("拒绝打开：该路径不在桌面索引或集合允许范围内".into()));
    }
    tracing::info!(path, "opening desktop item");
    open_with_shell(&path)
}

/// Shell icon for an allowed path as base64 RGBA (`None` → UI shows a
/// glyph). Uses the same allow-list as opening.
#[tauri::command]
pub fn desktop_icon(state: State<'_, AppState>, path: String) -> AppResult<Option<IconPayloadDto>> {
    let allowed = {
        let mut db = lock_db(&state)?;
        path_allowed(&mut db, &path)?
    };
    if !allowed {
        return Ok(None);
    }
    Ok(extract_cached(&path)?.map(|p: IconPayload| IconPayloadDto {
        width: p.width,
        height: p.height,
        rgba: p.rgba,
    }))
}

/// Read-only listing of a folder's immediate children (expand a folder
/// reference in place). The folder itself must pass the allow-list.
#[tauri::command]
pub fn browse_children(state: State<'_, AppState>, path: String) -> AppResult<Vec<PathEntry>> {
    let allowed = {
        let mut db = lock_db(&state)?;
        path_allowed(&mut db, &path)?
    };
    if !allowed {
        return Err(AppError::Other(
            "拒绝浏览：该文件夹不在桌面索引或集合允许范围内".into(),
        ));
    }
    tracing::debug!(path, "browsing folder children");
    list_children(&path)
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
    parent_id: Option<i64>,
) -> AppResult<Collection> {
    let mut db = lock_db(&state)?;
    let created = CollectionsRepo::new(db.conn()).create(&name, &color, parent_id)?;
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

/// Assign any absolute path on disk to a collection (drag-in from Explorer).
/// Desktop-indexed paths keep live metadata; the rest snapshot theirs.
#[tauri::command]
pub fn collection_assign_external(
    state: State<'_, AppState>,
    id: i64,
    path: String,
) -> AppResult<bool> {
    let mut db = lock_db(&state)?;
    let created = CollectionsRepo::new(db.conn()).assign_any(id, &path)?;
    if created {
        tracing::info!(collection_id = id, path, "item dragged into collection");
    }
    Ok(created)
}

/// Open an item stored in a collection. Allowed if the path passes the
/// shared allow-list: desktop index, a collection, or inside such a folder
/// (D14, extended for folder browsing).
#[tauri::command]
pub fn collection_open(state: State<'_, AppState>, path: String) -> AppResult<()> {
    let allowed = {
        let mut db = lock_db(&state)?;
        path_allowed(&mut db, &path)?
    };
    if !allowed {
        return Err(AppError::Other(
            "拒绝打开：该项目不在桌面索引或任何集合中".into(),
        ));
    }
    tracing::info!(path, "opening collection item");
    open_with_shell(&path)
}
