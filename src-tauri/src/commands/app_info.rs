use serde::Serialize;
use tauri::State;

use crate::app::error::AppResult;
use crate::app::state::{lock_db, AppState};

/// Diagnostic snapshot of the app instance, used by the UI to verify the
/// backend is wired up (and useful in support/debug scenarios).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub data_dir: String,
    pub db_path: String,
    pub log_dir: String,
    pub os: String,
    pub schema_version: i64,
}

#[tauri::command]
pub fn app_info(state: State<'_, AppState>) -> AppResult<AppInfo> {
    let schema_version = {
        let mut db = lock_db(&state)?;
        crate::storage::migrations::current_version(db.conn())?
    };
    Ok(AppInfo {
        name: "DesktopManager".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        data_dir: state.data_dir.display().to_string(),
        db_path: state
            .data_dir
            .join(crate::app::state::DB_FILE_NAME)
            .display()
            .to_string(),
        log_dir: state.log_dir.display().to_string(),
        os: format!("{} ({})", std::env::consts::OS, std::env::consts::ARCH),
        schema_version,
    })
}
