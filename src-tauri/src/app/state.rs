use std::path::PathBuf;
use std::sync::Mutex;

use tauri::Manager;

use super::error::{AppError, AppResult};
use crate::storage::Database;

/// Central application state managed by Tauri.
pub struct AppState {
    pub db: Mutex<Database>,
    pub data_dir: PathBuf,
    pub log_dir: PathBuf,
}

impl AppState {
    pub fn init<R: tauri::Runtime>(app: &tauri::App<R>) -> AppResult<Self> {
        let data_dir = app.path().app_data_dir()?;
        std::fs::create_dir_all(&data_dir)?;

        let log_dir = data_dir.join("logs");
        let log_guard = super::logging::init(app)?;
        app.manage(log_guard);

        let db_path = data_dir.join(DB_FILE_NAME);
        let db = Database::open(&db_path)?;
        tracing::info!(db = %db_path.display(), "database opened");

        Ok(Self {
            db: Mutex::new(db),
            data_dir,
            log_dir,
        })
    }
}

pub const DB_FILE_NAME: &str = "desktopmanager.db";

/// Lock the database, mapping a poisoned mutex into a recoverable error
/// instead of panicking.
pub fn lock_db<'a>(
    state: &'a tauri::State<'a, AppState>,
) -> AppResult<std::sync::MutexGuard<'a, Database>> {
    state
        .db
        .lock()
        .map_err(|_| AppError::Other("database lock poisoned by an earlier panic".into()))
}
