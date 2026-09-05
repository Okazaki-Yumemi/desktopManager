//! Focus commands (M5): start/finish/interrupt sessions, notes, day list
//! and daily summary. A running session lives entirely in SQLite
//! (started_at is the clock), so restarts cannot lose a running block.

use tauri::State;

use crate::app::error::AppResult;
use crate::app::state::{lock_db, AppState};
use crate::storage::focus_repo::{FocusDay, FocusRepo, FocusSession};

#[tauri::command]
pub fn focus_start(
    state: State<'_, AppState>,
    kind: String,
    planned_seconds: i64,
    task_id: Option<i64>,
    scene_id: Option<i64>,
) -> AppResult<FocusSession> {
    let mut db = lock_db(&state)?;
    let session = FocusRepo::new(db.conn()).start(&kind, planned_seconds, task_id, scene_id)?;
    tracing::info!(
        session_id = session.id,
        kind = %session.kind,
        planned_s = session.planned_duration_s,
        scene_id = session.scene_id,
        "focus session started"
    );
    Ok(session)
}

#[tauri::command]
pub fn focus_running(state: State<'_, AppState>) -> AppResult<Option<FocusSession>> {
    let mut db = lock_db(&state)?;
    FocusRepo::new(db.conn()).running()
}

#[tauri::command]
pub fn focus_finish(
    state: State<'_, AppState>,
    id: i64,
    status: String,
) -> AppResult<FocusSession> {
    let mut db = lock_db(&state)?;
    FocusRepo::new(db.conn()).finish(id, &status)
}

#[tauri::command]
pub fn focus_interrupt(state: State<'_, AppState>, id: i64) -> AppResult<FocusSession> {
    let mut db = lock_db(&state)?;
    FocusRepo::new(db.conn()).add_interruption(id)
}

#[tauri::command]
pub fn focus_note(state: State<'_, AppState>, id: i64, note: Option<String>) -> AppResult<()> {
    let mut db = lock_db(&state)?;
    FocusRepo::new(db.conn()).set_note(id, note.as_deref())
}

/// All sessions started on `day` (YYYY-MM-DD; the frontend computes the
/// local day string so the backend never needs local-time date math).
#[tauri::command]
pub fn focus_sessions(state: State<'_, AppState>, day: String) -> AppResult<Vec<FocusSession>> {
    let mut db = lock_db(&state)?;
    FocusRepo::new(db.conn()).sessions_of_day(&day)
}

/// Per-day focus totals over the last `days` local days.
#[tauri::command]
pub fn focus_summary(state: State<'_, AppState>, days: i64) -> AppResult<Vec<FocusDay>> {
    let mut db = lock_db(&state)?;
    FocusRepo::new(db.conn()).summary_days(days)
}
