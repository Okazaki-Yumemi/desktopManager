//! Calendar event commands (M6): thin wrappers over calendar_repo.

use tauri::State;

use crate::app::error::AppResult;
use crate::app::state::{lock_db, AppState};
use crate::storage::calendar_repo::{CalendarEvent, CalendarRepo};

#[tauri::command]
pub fn event_list_range(
    state: State<'_, AppState>,
    from: i64,
    to: i64,
) -> AppResult<Vec<CalendarEvent>> {
    let mut db = lock_db(&state)?;
    CalendarRepo::new(db.conn()).list_range(from, to)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn event_create(
    state: State<'_, AppState>,
    title: String,
    starts_at: i64,
    ends_at: i64,
    all_day: bool,
    notes: Option<String>,
    color: Option<String>,
    task_id: Option<i64>,
) -> AppResult<CalendarEvent> {
    let mut db = lock_db(&state)?;
    let event = CalendarRepo::new(db.conn()).create(
        &title,
        starts_at,
        ends_at,
        all_day,
        notes.as_deref(),
        color.as_deref(),
        task_id,
    )?;
    tracing::info!(id = event.id, "calendar event created");
    Ok(event)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn event_update(
    state: State<'_, AppState>,
    id: i64,
    title: String,
    starts_at: i64,
    ends_at: i64,
    all_day: bool,
    notes: Option<String>,
    color: Option<String>,
    task_id: Option<i64>,
) -> AppResult<()> {
    let mut db = lock_db(&state)?;
    CalendarRepo::new(db.conn()).update(
        id,
        &title,
        starts_at,
        ends_at,
        all_day,
        notes.as_deref(),
        color.as_deref(),
        task_id,
    )
}

/// Move an event's window (drag a block on the week grid).
#[tauri::command]
pub fn event_reschedule(
    state: State<'_, AppState>,
    id: i64,
    starts_at: i64,
    ends_at: i64,
) -> AppResult<()> {
    let mut db = lock_db(&state)?;
    CalendarRepo::new(db.conn()).reschedule(id, starts_at, ends_at)
}

#[tauri::command]
pub fn event_delete(state: State<'_, AppState>, id: i64) -> AppResult<bool> {
    let mut db = lock_db(&state)?;
    CalendarRepo::new(db.conn()).delete(id)
}
