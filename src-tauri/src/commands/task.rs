//! Task commands (M6): thin wrappers over tasks_repo.

use tauri::State;

use crate::app::error::AppResult;
use crate::app::state::{lock_db, AppState};
use crate::storage::tasks_repo::{Task, TasksRepo};

#[tauri::command]
pub fn task_list(state: State<'_, AppState>) -> AppResult<Vec<Task>> {
    let mut db = lock_db(&state)?;
    TasksRepo::new(db.conn()).list()
}

#[tauri::command]
pub fn task_create(
    state: State<'_, AppState>,
    title: String,
    notes: Option<String>,
    priority: i64,
    due_at: Option<i64>,
    estimated_minutes: Option<i64>,
    tags: Vec<String>,
) -> AppResult<Task> {
    let mut db = lock_db(&state)?;
    let task = TasksRepo::new(db.conn()).create(
        &title,
        notes.as_deref(),
        priority,
        due_at,
        estimated_minutes,
        &tags,
    )?;
    tracing::info!(id = task.id, "task created");
    Ok(task)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn task_update(
    state: State<'_, AppState>,
    id: i64,
    title: String,
    notes: Option<String>,
    priority: i64,
    due_at: Option<i64>,
    estimated_minutes: Option<i64>,
    tags: Vec<String>,
) -> AppResult<()> {
    let mut db = lock_db(&state)?;
    TasksRepo::new(db.conn()).update(
        id,
        &title,
        notes.as_deref(),
        priority,
        due_at,
        estimated_minutes,
        &tags,
    )
}

#[tauri::command]
pub fn task_set_status(state: State<'_, AppState>, id: i64, status: String) -> AppResult<Task> {
    let mut db = lock_db(&state)?;
    let task = TasksRepo::new(db.conn()).set_status(id, &status)?;
    tracing::info!(id, status = %task.status, "task status changed");
    Ok(task)
}

#[tauri::command]
pub fn task_delete(state: State<'_, AppState>, id: i64) -> AppResult<bool> {
    let mut db = lock_db(&state)?;
    TasksRepo::new(db.conn()).delete(id)
}
