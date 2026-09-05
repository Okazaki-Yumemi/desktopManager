//! Global shortcut registration with conflict-tolerant status reporting.

use serde::Serialize;
use tauri::{AppHandle, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use super::shell::toggle_main_window;

/// Binding for the command palette (M6). Until the palette exists it toggles
/// the main window, which is the palette's first action anyway.
pub const COMMAND_PALETTE_SHORTCUT: &str = "alt+shift+d";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutStatus {
    pub binding: String,
    pub registered: bool,
    pub error: Option<String>,
}

/// Register the global shortcut. Registration failures (another app already
/// owns the key, e.g. PowerToys) must not abort startup: the status is
/// surfaced in Settings instead. Never hand-write shortcut strings — see D7.
pub fn register_command_palette<R: Runtime>(app: &AppHandle<R>) -> ShortcutStatus {
    let mut status = ShortcutStatus {
        binding: COMMAND_PALETTE_SHORTCUT.to_string(),
        registered: false,
        error: None,
    };
    let result =
        app.global_shortcut()
            .on_shortcut(COMMAND_PALETTE_SHORTCUT, |app, _shortcut, event| {
                if event.state == ShortcutState::Pressed {
                    toggle_main_window(app);
                }
            });
    match result {
        Ok(()) => {
            status.registered = true;
            tracing::info!(
                binding = COMMAND_PALETTE_SHORTCUT,
                "global shortcut registered"
            );
        }
        Err(e) => {
            status.error = Some(e.to_string());
            tracing::warn!(
                binding = COMMAND_PALETTE_SHORTCUT,
                "global shortcut registration failed: {e}"
            );
        }
    }
    status
}
