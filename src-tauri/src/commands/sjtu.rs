//! SJTU calendar sync commands (M12).
//!
//! Data path: a dedicated webview window opens the university portal, the
//! user logs into jAccount there (credentials never touch this app — they
//! live in the system WebView profile), and an injected same-origin script
//! fetches the calendar JSON and pushes it here through `sjtu_receive`.
//! That command is the ONLY IPC surface granted to the remote page
//! (see capabilities/sjtu-remote.json and permissions/allow-sjtu-receive).

use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};
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

/// Runs on every document of the sync window, top page AND iframes: the
/// portal embeds the calendar app from calendar.sjtu.edu.cn in a cross-origin
/// iframe, and its API paths are not stable public contract. So instead of
/// guessing endpoints we passively hook the page's own fetch/XHR and forward
/// the first response matching the calendar JSON shape through the
/// receive-only command. Nothing is requested beyond what the page itself
/// loads; same-origin candidate fetches remain as a fallback.
const INIT_SCRIPT: &str = r#"
(function () {
  var h = location.host;
  if (h !== "my.sjtu.edu.cn" && h !== "calendar.sjtu.edu.cn") return;
  var MAX = 2000000;
  // The portal hands out one week per fetch. Since the point of the sync is
  // planning ahead (arranging next Monday on Sunday!), after the first
  // capture we replay the captured request with its date params shifted
  // forward, week by week, and push every distinct response. The backend
  // merges by external id (D29), so overlapping weeks are harmless.
  var WEEKS_AHEAD = 8;
  var seen = {};
  var template = null;
  var autofetching = false;
  function finger(t) { return t.length + ":" + t.slice(0, 120); }
  function push(t) {
    if (typeof t !== "string" || !t || t.length > MAX) return false;
    if (t.charCodeAt(0) !== 0x7b) return false;
    var f = finger(t);
    if (seen[f]) return false;
    var j = null;
    try { j = JSON.parse(t); } catch (e) { return false; }
    if (!(j && j.data && (Array.isArray(j.data.events) ||
        (j.data.schoolCalendar && Array.isArray(j.data.schoolCalendar.events))))) return false;
    seen[f] = true;
    var tauri = window.__TAURI_INTERNALS__;
    if (tauri && typeof tauri.invoke === "function") {
      tauri.invoke("sjtu_receive", { payload: t }).catch(function () {});
    }
    return true;
  }
  function noteCapture(t, u) {
    if (push(t) && u && !template) {
      template = u;
      setTimeout(autofetch, 800);
    }
  }
  // Shift date-like tokens forward by k weeks: ISO dates in the path or the
  // query, epoch seconds/millis in the query. Returns null when the URL
  // carries nothing shiftable (the user then just navigates weeks manually).
  function shiftedUrl(u, k) {
    var m = /^(https?:\/\/[^?#]+)(\?[^#]*)?(#.*)?$/.exec(u);
    if (!m) return null;
    var changed = false;
    function shiftToken(s) {
      var iso = /^(\d{4})-(\d{2})-(\d{2})$/.exec(s);
      if (iso) {
        var d = new Date(+iso[1], +iso[2] - 1, +iso[3]);
        d.setDate(d.getDate() + 7 * k);
        var p = function (n) { return (n < 10 ? "0" : "") + n; };
        return d.getFullYear() + "-" + p(d.getMonth() + 1) + "-" + p(d.getDate());
      }
      if (/^\d{13}$/.test(s)) return String(+s + 7 * k * 86400000);
      if (/^\d{10}$/.test(s)) return String(+s + 7 * k * 86400);
      return null;
    }
    var path = m[1].replace(/\d{4}-\d{2}-\d{2}/g, function (tok) {
      var s = shiftToken(tok);
      if (s) { changed = true; return s; }
      return tok;
    });
    var query = m[2] || "";
    if (query.length > 1) {
      var parts = query.slice(1).split("&").map(function (kv) {
        var i = kv.indexOf("=");
        if (i === -1) return kv;
        var v = null;
        try { v = shiftToken(decodeURIComponent(kv.slice(i + 1))); } catch (e) {}
        if (v) { changed = true; return kv.slice(0, i + 1) + encodeURIComponent(v); }
        return kv;
      });
      query = "?" + parts.join("&");
    }
    return changed ? path + query + (m[3] || "") : null;
  }
  function autofetch() {
    if (!template || autofetching) return;
    if (!shiftedUrl(template, 1)) return;
    autofetching = true;
    var k = 1;
    function step() {
      if (k > WEEKS_AHEAD) return;
      var u = shiftedUrl(template, k++);
      if (!u) return;
      fetch(u, { credentials: "include", headers: { Accept: "application/json" } })
        .then(function (r) { return r.ok ? r.text() : ""; })
        .then(function (t) { push(t); })
        .catch(function () {})
        .then(step);
    }
    step();
  }
  if (typeof window.fetch === "function") {
    var origFetch = window.fetch;
    window.fetch = function () {
      var input = arguments[0];
      var p = origFetch.apply(this, arguments);
      try {
        p.then(function (r) {
          try {
            var ct = "";
            try { ct = (r.headers && r.headers.get && r.headers.get("content-type")) || ""; } catch (e) {}
            if (ct && ct.indexOf("json") === -1 && ct.indexOf("text") === -1) return;
            var u = "";
            try { u = (input && typeof input.url === "string") ? input.url : String(input || ""); } catch (e) {}
            r.clone().text().then(function (t) { noteCapture(t, u); }).catch(function () {});
          } catch (e) {}
        }).catch(function () {});
      } catch (e) {}
      return p;
    };
  }
  (function () {
    var origOpen = XMLHttpRequest.prototype.open;
    var origSend = XMLHttpRequest.prototype.send;
    XMLHttpRequest.prototype.open = function (m, u) {
      return origOpen.apply(this, arguments);
    };
    XMLHttpRequest.prototype.send = function () {
      var xhr = this;
      try {
        xhr.addEventListener("load", function () {
          try {
            var t = "";
            try { t = xhr.responseText || ""; } catch (e) {
              try {
                if (xhr.response && typeof xhr.response === "object") t = JSON.stringify(xhr.response);
              } catch (e2) {}
            }
            var u = "";
            try { u = xhr.responseURL || ""; } catch (e) {}
            noteCapture(t, u);
          } catch (e) {}
        });
      } catch (e) {}
      return origSend.apply(this, arguments);
    };
  })();
  // Same-origin fallback in case the SPA serves cached data and never
  // refetches this session. Cross-origin attempts are pointless (CORS).
  var CANDIDATES = h === "my.sjtu.edu.cn"
    ? ["/ui/api/calendar", "/ui/api/event/list"]
    : ["/api/event/list"];
  function anySeen() {
    for (var k in seen) { if (Object.prototype.hasOwnProperty.call(seen, k)) return true; }
    return false;
  }
  function tryFetch() {
    if (anySeen()) return;
    var attempt = 0;
    function next() {
      if (anySeen() || attempt >= CANDIDATES.length) return;
      var url = CANDIDATES[attempt++];
      fetch(url, { credentials: "include", headers: { Accept: "application/json" } })
        .then(function (r) { return r.ok ? r.text() : ""; })
        .then(function (t) { noteCapture(t, url); if (!anySeen()) next(); })
        .catch(next);
    }
    next();
  }
  setTimeout(tryFetch, 1500);
  setTimeout(function () { tryFetch(); }, 7000);
})();
"#;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SjtuSyncReport {
    /// Rows in THIS push (one week's payload).
    pub count: usize,
    /// Cumulative rows pushed in the current sync-window session — the
    /// portal is fetched one week at a time, so a session is several pushes.
    pub total: usize,
    pub skipped: usize,
    pub synced_at: i64,
}

/// Monotonic bookkeeping for the multi-push sync session (D29): `total`
/// feeds the progress toast, and the last-receive stamp debounces the
/// window's auto-close until the pushes have settled.
static SESSION_TOTAL: AtomicUsize = AtomicUsize::new(0);
static LAST_RECEIVE: AtomicI64 = AtomicI64::new(0);

/// How long the sync window lingers after the latest push before closing —
/// long enough for the eight automatic future-week fetches (and any manual
/// week navigation) to land, without making the user wait once they stop.
const CLOSE_QUIET_MS: u64 = 6_000;

/// Receive-only target for the university page. Payload is size-capped and
/// strictly parsed before anything touches the database. Called once per
/// captured week; pushes merge by external id (D29).
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
        let count = SjtuRepo::new(db.conn()).upsert_events(&parsed.events, synced_at)?;
        SettingsRepo::new(db.conn()).set(LAST_SYNC_KEY, &serde_json::json!(synced_at))?;
        let total = SESSION_TOTAL.fetch_add(count, Ordering::SeqCst) + count;
        SjtuSyncReport {
            count,
            total,
            skipped: parsed.skipped,
            synced_at,
        }
    };
    LAST_RECEIVE.store(synced_at, Ordering::SeqCst);
    tracing::info!(count = report.count, total = report.total, skipped = report.skipped, "SJTU calendar push merged");
    app.emit("sjtu-synced", &report)?;
    schedule_close_when_quiet(&app, synced_at);
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
            total: 0,
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
/// This command MUST stay `async`: window creation from a synchronous
/// command runs on the main thread and deadlocks WebView2 initialization
/// on Windows — the window frame appears but never paints (white) and
/// never becomes interactive.
#[tauri::command]
pub async fn sjtu_open_sync(app: AppHandle, state: State<'_, AppState>) -> AppResult<String> {
    // A fresh window is a fresh session: the progress toast counts from zero.
    SESSION_TOTAL.store(0, Ordering::SeqCst);
    let url = calendar_url(&state)?;
    if let Some(window) = app.get_webview_window("sjtu") {
        let _ = window.show();
        let _ = window.set_focus();
        window.eval(format!("window.location.assign({url:?});"))?;
        return Ok("navigated".into());
    }
    tracing::info!(url = %url, "SJTU sync window opening");
    WebviewWindowBuilder::new(&app, "sjtu", WebviewUrl::External(url.clone()))
        .title("交大日程 · 登录 jAccount")
        .inner_size(1080.0, 800.0)
        .min_inner_size(560.0, 500.0)
        // Never steal foreground focus from whatever the user is doing.
        .focused(false)
        .initialization_script(INIT_SCRIPT)
        .build()?;
    tracing::info!(url = %url, "SJTU sync window opened");
    Ok("opened".into())
}

/// Close the sync window once the pushes have settled (the toast on the
/// main window is the user-facing confirmation). Login sessions keep it
/// open. Each receive re-arms the timer by stamping LAST_RECEIVE; the
/// closer only fires when its own stamp is still the newest one. The close
/// itself must run on the main thread — window teardown from a background
/// thread races the event loop on Windows.
fn schedule_close_when_quiet(app: &AppHandle, stamp: i64) {
    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(CLOSE_QUIET_MS));
        if LAST_RECEIVE.load(Ordering::SeqCst) != stamp {
            return; // a newer push re-armed the quiet timer
        }
        let closer = handle.clone();
        let _ = handle.run_on_main_thread(move || {
            if let Some(window) = closer.get_webview_window("sjtu") {
                let _ = window.close();
            }
        });
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
