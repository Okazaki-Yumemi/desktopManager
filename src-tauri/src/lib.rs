//! DesktopManager — lightweight personal desktop workspace manager.
//!
//! Module layout (see docs/ARCHITECTURE.md):
//! - [`app`]: application state, error type, logging, tray + global shortcuts
//! - [`desktop`]: desktop folder discovery, scanning, watcher, shell-open
//! - [`storage`]: SQLite open/migrate + repositories (the only place SQL lives)
//! - [`commands`]: Tauri command handlers exposed to the frontend

mod app;
mod commands;
mod desktop;
mod storage;

use tauri::{Manager, WindowEvent};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let sources = desktop::discovery::discover_desktop_sources();
            let state = app::state::AppState::init(app, sources)?;
            // Conflict-tolerant: a taken binding lands in Settings, not in a
            // failed startup.
            let status = app::shortcuts::register_command_palette(app.handle());
            state.set_shortcut_status(status);
            app.manage(state);
            app::shell::create_tray(app)?;
            // Build the desktop index now and keep it live via fs events.
            desktop::service::init(app.handle());
            tracing::info!("DesktopManager started");
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Resident tool: closing the window hides it to the tray.
                api.prevent_close();
                let _ = window.hide();
                tracing::info!("main window hidden to tray (close requested)");
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info::app_info,
            commands::settings::settings_get,
            commands::settings::settings_set,
            commands::shortcuts::shortcuts_get,
            commands::desktop::desktop_list,
            commands::desktop::desktop_search,
            commands::desktop::desktop_rescan,
            commands::desktop::desktop_open,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|err| {
            eprintln!("failed to run DesktopManager: {err}");
            std::process::exit(1);
        });
}
