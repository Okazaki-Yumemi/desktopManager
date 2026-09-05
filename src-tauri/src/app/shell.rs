//! Tray icon + window show/hide behaviors (the resident "app shell").

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Runtime,
};

/// Show (and focus) the main window from any context.
pub fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}

/// True when this window is the foreground window. tao's `is_focused()`
/// tracks internal focus events and can miss WebView2's child-window focus,
/// so compare against GetForegroundWindow directly.
#[cfg(windows)]
fn is_foreground<R: Runtime>(win: &tauri::WebviewWindow<R>) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    match win.hwnd() {
        Ok(h) => (unsafe { GetForegroundWindow() }) == h,
        Err(_) => false,
    }
}

/// Tray left-click and the global shortcut both toggle the window: if it is
/// the foreground window, hide it; otherwise bring it to front.
pub fn toggle_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(win) = app.get_webview_window("main") {
        let visible = win.is_visible().unwrap_or(false);
        let foreground = is_foreground(&win);
        tracing::debug!(visible, foreground, "toggle main window requested");
        if visible && foreground {
            let _ = win.hide();
        } else {
            let _ = win.show();
            let _ = win.unminimize();
            let _ = win.set_focus();
        }
    }
}

/// Build the tray icon with its menu. UI language is Chinese by user decision
/// (see docs/DECISIONS.md D9).
pub fn create_tray<R: Runtime>(app: &tauri::App<R>) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    TrayIconBuilder::with_id("main-tray")
        .icon(
            app.default_window_icon()
                .expect("app ships a default window icon")
                .clone(),
        )
        .tooltip("DesktopManager")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "quit" => {
                tracing::info!("quit requested from tray menu");
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_main_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}
