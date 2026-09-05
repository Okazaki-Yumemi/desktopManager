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
        // Serve the stored background image to the webview as
        // http://bg.localhost/background.img (Windows maps `bg` there).
        .register_uri_scheme_protocol("bg", |ctx, _request| {
            let not_found = || {
                tauri::http::Response::builder()
                    .status(404)
                    .body(Vec::new())
                    .expect("static response")
            };
            let state = ctx.app_handle().state::<app::state::AppState>();
            let file = state.data_dir.join("background.img");
            let Ok(bytes) = std::fs::read(&file) else {
                return not_found();
            };
            tauri::http::Response::builder()
                .header("Content-Type", commands::background::sniff_mime(&bytes))
                .header("Cache-Control", "no-store")
                .body(bytes)
                .unwrap_or_else(|_| not_found())
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
            commands::desktop::desktop_icon,
            commands::desktop::collections_list,
            commands::desktop::collection_create,
            commands::desktop::collection_rename,
            commands::desktop::collection_delete,
            commands::desktop::collection_assign,
            commands::desktop::collection_unassign,
            commands::desktop::collection_items,
            commands::desktop::collection_assign_external,
            commands::desktop::collection_open,
            commands::background::background_set,
            commands::background::background_clear,
            commands::background::appdata_purge,
            commands::layout::layout_capture,
            commands::layout::layout_list,
            commands::layout::layout_apply,
            commands::layout::layout_delete,
            commands::scene::scenes_list,
            commands::scene::scene_create,
            commands::scene::scene_rename,
            commands::scene::scene_delete,
            commands::scene::scene_set_visibility,
            commands::scene::scene_visibility,
            commands::focus::focus_start,
            commands::focus::focus_running,
            commands::focus::focus_finish,
            commands::focus::focus_interrupt,
            commands::focus::focus_note,
            commands::focus::focus_sessions,
            commands::focus::focus_summary,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|err| {
            eprintln!("failed to run DesktopManager: {err}");
            std::process::exit(1);
        });
}
