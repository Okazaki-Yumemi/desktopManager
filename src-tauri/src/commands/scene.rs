//! Scene commands (M4): CRUD plus per-scene collection visibility.
//! Scenes are pure metadata over collections — nothing on disk changes.

use tauri::State;

use crate::app::state::{lock_db, AppState};
use crate::app::error::AppResult;
use crate::storage::scenes_repo::{Scene, SceneLayout, ScenesRepo};

#[tauri::command]
pub fn scenes_list(state: State<'_, AppState>) -> AppResult<Vec<Scene>> {
    let mut db = lock_db(&state)?;
    ScenesRepo::new(db.conn()).list()
}

#[tauri::command]
pub fn scene_create(
    state: State<'_, AppState>,
    name: String,
    color: Option<String>,
) -> AppResult<Scene> {
    let mut db = lock_db(&state)?;
    let scene = ScenesRepo::new(db.conn()).create(&name, color.as_deref())?;
    tracing::info!(scene_id = scene.id, name = %scene.name, "scene created");
    Ok(scene)
}

#[tauri::command]
pub fn scene_rename(state: State<'_, AppState>, id: i64, name: String) -> AppResult<()> {
    let mut db = lock_db(&state)?;
    ScenesRepo::new(db.conn()).rename(id, &name)
}

#[tauri::command]
pub fn scene_delete(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    let mut db = lock_db(&state)?;
    ScenesRepo::new(db.conn()).delete(id)?;
    tracing::info!(scene_id = id, "scene deleted");
    Ok(())
}

#[tauri::command]
pub fn scene_set_visibility(
    state: State<'_, AppState>,
    id: i64,
    collection_id: i64,
    visible: bool,
) -> AppResult<()> {
    let mut db = lock_db(&state)?;
    ScenesRepo::new(db.conn()).set_visible(id, collection_id, visible)
}

#[tauri::command]
pub fn scene_visibility(state: State<'_, AppState>, id: i64) -> AppResult<Vec<SceneLayout>> {
    let mut db = lock_db(&state)?;
    ScenesRepo::new(db.conn()).visibility(id)
}
