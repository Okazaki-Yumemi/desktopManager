use crate::app::error::AppResult;
use crate::app::shortcuts::ShortcutStatus;
use crate::app::state::AppState;

/// Reports the global-shortcut registration outcome so the Settings UI can
/// surface conflicts (another app already owns the binding).
#[tauri::command]
pub fn shortcuts_get(state: tauri::State<'_, AppState>) -> AppResult<ShortcutStatus> {
    let guard = state
        .shortcut_status
        .lock()
        .map_err(|_| crate::app::error::AppError::Other("shortcut status lock poisoned".into()))?;
    Ok(guard.clone())
}
