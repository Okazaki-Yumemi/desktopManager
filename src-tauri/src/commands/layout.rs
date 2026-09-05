//! Desktop icon-layout commands (M3): capture the live layout through the
//! shell, list/apply/delete saved snapshots. Never moves files — only icon
//! positions.

use tauri::State;

use crate::app::error::{AppError, AppResult};
use crate::app::state::{lock_db, AppState};
use crate::desktop::shell_layout::{self, ApplyReport, LayoutPayload};
use crate::storage::layout_repo::{LayoutRepo, LayoutSummary};

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapturedLayout {
    pub id: i64,
    pub name: String,
    pub item_count: usize,
}

/// Read the live desktop icon layout and store it under `name`.
#[tauri::command]
pub fn layout_capture(state: State<'_, AppState>, name: String) -> AppResult<CapturedLayout> {
    let payload = shell_layout::read_layout()?;
    if payload.items.is_empty() {
        return Err(AppError::Other("未从桌面读到任何图标，已取消保存".into()));
    }
    let count = payload.items.len();
    let json =
        serde_json::to_string(&payload).map_err(|e| AppError::Other(e.to_string()))?;
    let name = name.trim().to_string();
    let mut db = lock_db(&state)?;
    let id = LayoutRepo::new(db.conn()).save(&name, &json)?;
    tracing::info!(id, name = %name, items = count, "layout captured");
    Ok(CapturedLayout {
        id,
        name,
        item_count: count,
    })
}

#[tauri::command]
pub fn layout_list(state: State<'_, AppState>) -> AppResult<Vec<LayoutSummary>> {
    let mut db = lock_db(&state)?;
    LayoutRepo::new(db.conn()).list()
}

/// Restore a saved layout. The canary check runs first and refuses when the
/// shell overrides position writes (auto-arrange).
#[tauri::command]
pub fn layout_apply(state: State<'_, AppState>, id: i64) -> AppResult<ApplyReport> {
    let json = {
        let mut db = lock_db(&state)?;
        LayoutRepo::new(db.conn()).payload_of(id)?
    }
    .ok_or_else(|| AppError::Other("布局不存在".into()))?;
    let payload: LayoutPayload =
        serde_json::from_str(&json).map_err(|e| AppError::Other(e.to_string()))?;
    shell_layout::canary_check()?;
    let report = shell_layout::apply_layout(&payload)?;
    tracing::info!(
        id,
        applied = report.applied,
        missing = report.missing,
        diverged = report.diverged,
        "layout applied"
    );
    Ok(report)
}

#[tauri::command]
pub fn layout_delete(state: State<'_, AppState>, id: i64) -> AppResult<bool> {
    let mut db = lock_db(&state)?;
    let removed = LayoutRepo::new(db.conn()).delete(id)?;
    if removed {
        tracing::info!(id, "layout deleted");
    }
    Ok(removed)
}
