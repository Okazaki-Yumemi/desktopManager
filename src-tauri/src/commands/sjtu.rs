//! SJTU calendar sync commands (M12).
//!
//! Data path: a dedicated webview window opens the university portal, the
//! user logs into jAccount there (credentials never touch this app — they
//! live in the system WebView profile), and an injected same-origin script
//! fetches the calendar JSON and pushes it here through `sjtu_receive`.
//! That command is the ONLY IPC surface granted to the remote page
//! (see capabilities/sjtu-remote.json and permissions/allow-sjtu-receive).

use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

use crate::app::error::{AppError, AppResult};
use crate::app::state::{lock_db, AppState};
use crate::storage::settings_repo::SettingsRepo;
use crate::storage::sjtu_repo::{SjtuEvent, SjtuRepo};
use crate::sjtu;

pub const DEFAULT_CALENDAR_URL: &str = "https://my.sjtu.edu.cn/ui/calendar/";
pub const LAST_SYNC_KEY: &str = "sjtu.lastSyncAt";
pub const CALENDAR_URL_KEY: &str = "sjtu.calendarUrl";

/// Runs on every page of the sync window. On the portal origin it fetches
/// the calendar API same-origin (the credentials stay inside the WebView)
/// and pushes the JSON body through the receive-only command. Candidate
/// endpoints are tried in order: same-origin prefixes first (no CORS/SameSite
/// concerns), then the dedicated calendar service host.
const INIT_SCRIPT: &str = r#"
(function () {
  if (location.host !== "my.sjtu.edu.cn") return;
  var CANDIDATES = [
    "/ui/api/calendar",
    "/ui/api/event/list",
    "https://calendar.sjtu.edu.cn/api/event/list"
  ];
  function looksValid(j) {
    return !!(j && j.data && (Array.isArray(j.data.events) ||
      (j.data.schoolCalendar && Array.isArray(j.data.schoolCalendar.events))));
  }
  function tryFetch() {
    var attempt = 0;
    function next() {
      if (attempt >= CANDIDATES.length) return;
      var url = CANDIDATES[attempt++];
      fetch(url, { credentials: "include", headers: { Accept: "application/json" } })
        .then(function (r) { return r.ok ? r.text() : ""; })
        .then(function (t) {
          if (!t || t.length > 2000000) { next(); return; }
          var j = null;
          try { j = JSON.parse(t); } catch (e) { j = null; }
          if (looksValid(j)) {
            var tauri = window.__TAURI_INTERNALS__;
            if (tauri && typeof tauri.invoke === "function") {
              tauri.invoke("sjtu_receive", { payload: t }).catch(function () {});
            }
          } else {
            next();
          }
        })
        .catch(next);
    }
    next();
  }
  setTimeout(tryFetch, 1200);
  // Retry once in case the SPA hydrates slowly; a redundant receive is
  // harmless (the projection is fully replaced).
  setTimeout(function () { tryFetch(); }, 6000);
})();
"#;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SjtuSyncReport {
    pub count: usize,
    pub skipped: usize,
    pub synced_at: i64,
}

/// Receive-only target for the university page. Payload is size-capped and
/// strictly parsed before anything touches the database.
#[tauri::command]
pub fn sjtu_receive(
    app: AppHandle,
    state: State<'_, AppState>,
    payload: String,
) -> AppResult<SjtuSyncReport> {
    let synced_at = crate::app::logging::now_millis();
    let parsed = sjtu::parse_payload(&payload, synced_at)?;
    let report = {
        let mut db = lock_db(&state)?;
        let count = SjtuRepo::new(db.conn()).replace_all(&parsed.events)?;
        SettingsRepo::new(db.conn()).set(LAST_SYNC_KEY, &serde_json::json!(synced_at))?;
        SjtuSyncReport {
            count,
            skipped: parsed.skipped,
            synced_at,
        }
    };
    tracing::info!(count = report.count, skipped = report.skipped, "SJTU calendar synced");
    app.emit("sjtu-synced", &report)?;
    close_sync_window_after(&app, Duration::from_millis(1500));
    Ok(report)
}

#[tauri::command]
pub fn sjtu_list(state: State<'_, AppState>) -> AppResult<Vec<SjtuEvent>> {
    let mut db = lock_db(&state)?;
    SjtuRepo::new(db.conn()).list_all()
}

#[tauri::command]
pub fn sjtu_clear(app: AppHandle, state: State<'_, AppState>) -> AppResult<usize> {
    let removed = {
        let mut db = lock_db(&state)?;
        SjtuRepo::new(db.conn()).clear()?
    };
    tracing::info!(removed, "SJTU calendar cleared");
    app.emit(
        "sjtu-synced",
        &SjtuSyncReport {
            count: 0,
            skipped: 0,
            synced_at: crate::app::logging::now_millis(),
        },
    )?;
    Ok(removed)
}

/// Open (or refocus) the sync window on the portal calendar page. The URL
/// may be overridden via the `sjtu.calendarUrl` setting but must stay on an
/// sjtu.edu.cn https host — the injected bridge and the capability are
/// scoped to that domain.
#[tauri::command]
pub fn sjtu_open_sync(app: AppHandle, state: State<'_, AppState>) -> AppResult<String> {
    let url = calendar_url(&state)?;
    if let Some(window) = app.get_webview_window("sjtu") {
        let _ = window.show();
        let _ = window.set_focus();
        window.eval(format!("window.location.assign({url:?});"))?;
        return Ok("navigated".into());
    }
    WebviewWindowBuilder::new(&app, "sjtu", WebviewUrl::External(url.clone()))
        .title("交大日程 · 登录 jAccount")
        .inner_size(1080.0, 800.0)
        .min_inner_size(560.0, 500.0)
        .initialization_script(INIT_SCRIPT)
        .build()?;
    tracing::info!(url = %url, "SJTU sync window opened");
    Ok("opened".into())
}

/// Close the sync window once a sync has landed (the toast on the main
/// window is the user-facing confirmation). Login sessions keep it open.
fn close_sync_window_after(app: &AppHandle, delay: Duration) {
    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(delay);
        if let Some(window) = handle.get_webview_window("sjtu") {
            let _ = window.close();
        }
    });
}

fn calendar_url(state: &State<'_, AppState>) -> AppResult<tauri::Url> {
    let raw: Option<String> = {
        let mut db = lock_db(state)?;
        SettingsRepo::new(db.conn()).get_string(CALENDAR_URL_KEY)?
    };
    let raw = raw.unwrap_or_else(|| DEFAULT_CALENDAR_URL.to_string());
    let url: tauri::Url = raw
        .parse()
        .map_err(|_| AppError::Other("交大日历地址无效".into()))?;
    let host_ok = url
        .host_str()
        .map(|h| h == "my.sjtu.edu.cn" || h.ends_with(".sjtu.edu.cn"))
        .unwrap_or(false);
    if url.scheme() != "https" || !host_ok {
        return Err(AppError::Other(
            "交大日历地址必须是 https://*.sjtu.edu.cn 下的页面".into(),
        ));
    }
    Ok(url)
}
