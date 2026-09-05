use serde_json::Value;
use tauri::State;

use crate::app::error::AppResult;
use crate::app::state::{lock_db, AppState};
use crate::storage::settings_repo::SettingsRepo;

#[tauri::command]
pub fn settings_get(state: State<'_, AppState>, key: String) -> AppResult<Option<Value>> {
    let mut db = lock_db(&state)?;
    let value = SettingsRepo::new(db.conn()).get(&key);
    if let Err(err) = &value {
        tracing::warn!(key, %err, "settings_get failed");
    }
    value
}

#[tauri::command]
pub fn settings_set(state: State<'_, AppState>, key: String, value: Value) -> AppResult<()> {
    let mut db = lock_db(&state)?;
    let result = SettingsRepo::new(db.conn()).set(&key, &value);
    match &result {
        Ok(()) => tracing::info!(key, "setting saved"),
        Err(err) => tracing::warn!(key, %err, "settings_set failed"),
    }
    result
}
