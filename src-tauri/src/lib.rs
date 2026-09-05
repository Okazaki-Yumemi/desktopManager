//! DesktopManager — lightweight personal desktop workspace manager.
//!
//! Module layout (see docs/ARCHITECTURE.md):
//! - [`app`]: application state, error type, logging
//! - [`storage`]: SQLite open/migrate + repositories (the only place SQL lives)
//! - [`commands`]: Tauri command handlers exposed to the frontend

mod app;
mod commands;
mod storage;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let state = app::state::AppState::init(app)?;
            app.manage(state);
            tracing::info!("DesktopManager started");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info::app_info,
            commands::settings::settings_get,
            commands::settings::settings_set,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|err| {
            eprintln!("failed to run DesktopManager: {err}");
            std::process::exit(1);
        });
}
