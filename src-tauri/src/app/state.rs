use std::path::PathBuf;
use std::sync::Mutex;

use tauri::Manager;

use super::error::{AppError, AppResult};
use super::shortcuts::ShortcutStatus;
use crate::desktop::discovery::DesktopSource;
use crate::storage::Database;

/// Central application state managed by Tauri.
pub struct AppState {
    pub db: Mutex<Database>,
    pub data_dir: PathBuf,
    pub log_dir: PathBuf,
    /// Outcome of global-shortcut registration at startup (conflict-safe).
    pub shortcut_status: Mutex<ShortcutStatus>,
    /// Desktop folders to index, discovered once at startup.
    pub desktop_sources: Vec<DesktopSource>,
}

impl AppState {
    pub fn init<R: tauri::Runtime>(
        app: &tauri::App<R>,
        desktop_sources: Vec<DesktopSource>,
    ) -> AppResult<Self> {
        let data_dir = app.path().app_data_dir()?;
        std::fs::create_dir_all(&data_dir)?;

        let log_dir = data_dir.join("logs");
        let log_guard = super::logging::init(app)?;
        app.manage(log_guard);

        let db_path = data_dir.join(DB_FILE_NAME);
        let (db, recovery) = Database::open_with_recovery(&db_path)?;
        if let Some(report) = recovery.as_ref() {
            tracing::warn!(
                quarantined = ?report.quarantined,
                "corrupt database quarantined; started with a fresh database"
            );
        }
        tracing::info!(db = %db_path.display(), "database opened");

        let shortcut_status = Mutex::new(ShortcutStatus {
            binding: super::shortcuts::COMMAND_PALETTE_SHORTCUT.to_string(),
            registered: false,
            error: None,
        });

        Ok(Self {
            db: Mutex::new(db),
            data_dir,
            log_dir,
            shortcut_status,
            desktop_sources,
        })
    }

    pub fn set_shortcut_status(&self, status: ShortcutStatus) {
        if let Ok(mut guard) = self.shortcut_status.lock() {
            *guard = status;
        }
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
