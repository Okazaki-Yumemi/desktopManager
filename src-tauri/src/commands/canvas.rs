//! Canvas LMS integration (M15) — read-only assignments & deadlines.
//!
//! Data path: the user creates a personal access token in Canvas
//! (账户 → 设置 → 新建访问令牌) and pastes it into the 作业 page themselves.
//! The token travels per-call from the frontend and is persisted only there
//! (settings table, local SQLite); the backend keeps no credentials and all
//! Canvas traffic is read-only GETs against the official REST API.

use std::io::Read;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::async_runtime::spawn_blocking;

use crate::app::error::{AppError, AppResult};
use crate::desktop::open::open_with_shell;

/// Overdue assignments stop being listed after this long — a deeper past is
/// a graveyard, not a planner.
const PAST_WINDOW_DAYS: i64 = 30;
/// How far ahead deadlines are fetched (one school year is plenty).
const FUTURE_WINDOW_DAYS: i64 = 365;
const DAY_MS: i64 = 86_400_000;
/// Canvas paginates at per_page=100; follow the Link header a bounded number
/// of times instead of trusting the server to stop.
const MAX_PAGES: usize = 5;
/// Response size guard: a Canvas page never needs more than this.
const MAX_BODY_BYTES: u64 = 8 * 1024 * 1024;

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .user_agent("DesktopManager/1.3 (personal desktop tool)")
        .build()
}

/// Validate + normalize a Canvas base URL: https only, host only, no path.
fn clean_base(base: &str) -> AppResult<String> {
    let trimmed = base.trim().trim_end_matches('/');
    let rest = trimmed
        .strip_prefix("https://")
        .ok_or_else(|| AppError::Other("Canvas 地址必须以 https:// 开头".into()))?;
    let host = rest.split('/').next().unwrap_or("");
    if host.is_empty() || !host.contains('.') || host.contains('@') {
        return Err(AppError::Other(format!("Canvas 地址无效：{base}")));
    }
    Ok(format!("https://{host}"))
}

/// `Link: <url>; rel="next", <url>; rel="last"` → the rel="next" URL.
fn next_link(header: &str) -> Option<String> {
    for part in header.split(',') {
        let mut pieces = part.split(';');
        let url = pieces
            .next()?
            .trim()
            .trim_start_matches('<')
            .trim_end_matches('>');
        if pieces.map(str::trim).any(|p| p == "rel=\"next\"") {
            return Some(url.to_string());
        }
    }
    None
}

fn parse_due(raw: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|t| t.timestamp_millis())
}

fn due_in_window(due_ms: i64, now_ms: i64) -> bool {
    due_ms >= now_ms - PAST_WINDOW_DAYS * DAY_MS && due_ms <= now_ms + FUTURE_WINDOW_DAYS * DAY_MS
}

fn submission_done(workflow_state: Option<&str>) -> bool {
    matches!(
        workflow_state,
        Some("submitted") | Some("graded") | Some("pending_review")
    )
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CanvasCourse {
    pub id: i64,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CanvasAssignment {
    pub id: i64,
    pub course_id: i64,
    pub name: String,
    /// Epoch millis; assignments without a due date are never listed.
    pub due_at: i64,
    pub html_url: String,
    pub points_possible: Option<f64>,
    pub submitted: bool,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CanvasSnapshot {
    pub fetched_at: i64,
    pub courses: Vec<CanvasCourse>,
    pub assignments: Vec<CanvasAssignment>,
    /// Courses whose assignment fetch failed mid-sync (partial success is
    /// honest; the frontend surfaces the count).
    pub skipped_courses: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CanvasProfile {
    pub name: String,
    pub login_id: Option<String>,
}

#[derive(Deserialize)]
struct RawCourse {
    id: i64,
    name: Option<String>,
}

#[derive(Deserialize)]
struct RawSubmission {
    workflow_state: Option<String>,
}

#[derive(Deserialize)]
struct RawAssignment {
    id: i64,
    course_id: i64,
    name: Option<String>,
    due_at: Option<String>,
    html_url: Option<String>,
    points_possible: Option<f64>,
    published: Option<bool>,
    submission: Option<RawSubmission>,
}

/// Active courses, following Canvas's Link pagination up to MAX_PAGES.
fn fetch_courses(agent: &ureq::Agent, base: &str, token: &str) -> AppResult<Vec<CanvasCourse>> {
    let mut out = Vec::new();
    let mut url = format!("{base}/api/v1/courses?enrollment_state=active&per_page=100");
    for _ in 0..MAX_PAGES {
        let resp = agent
            .get(&url)
            .set("Authorization", &format!("Bearer {token}"))
            .set("Accept", "application/json")
            .call()
            .map_err(map_ureq)?;
        let next = resp.header("Link").and_then(next_link);
        let body = read_body(resp)?;
        let raw: Vec<RawCourse> = serde_json::from_str(&body)
            .map_err(|err| AppError::Other(format!("Canvas 课程列表解析失败：{err}")))?;
        for c in raw {
            if let Some(name) = c.name {
                if !name.trim().is_empty() {
                    out.push(CanvasCourse { id: c.id, name });
                }
            }
        }
        match next {
            Some(n) => url = n,
            None => break,
        }
    }
    Ok(out)
}

fn map_ureq(err: ureq::Error) -> AppError {
    match err {
        ureq::Error::Status(code, _) => AppError::Other(match code {
            401 => "Canvas 令牌无效或已过期（HTTP 401），请到 Canvas → 账户 → 设置 重新生成访问令牌".into(),
            403 => "Canvas 拒绝访问（HTTP 403）：令牌权限不足或被管理员限制".into(),
            404 => "Canvas 接口不存在（HTTP 404），请检查 Canvas 地址".into(),
            429 => "请求过于频繁（HTTP 429），请稍后再试".into(),
            _ => format!("Canvas 返回 HTTP {code}"),
        }),
        ureq::Error::Transport(t) => AppError::Other(format!(
            "网络错误：{t}（若不在校园网环境，请先连接校园网或 VPN）"
        )),
    }
}

fn read_body(resp: ureq::Response) -> AppResult<String> {
    let mut body = String::new();
    resp.into_reader()
        .take(MAX_BODY_BYTES)
        .read_to_string(&mut body)
        .map_err(|err| AppError::Other(format!("读取 Canvas 响应失败：{err}")))?;
    if body.len() as u64 >= MAX_BODY_BYTES {
        return Err(AppError::Other("Canvas 响应过大，已中止".into()));
    }
    Ok(body)
}

/// One course's due-dated assignments, earliest first. Single page of 100
/// ordered by due_at: the head of that ordering is exactly the DDL window.
fn fetch_course_assignments(
    agent: &ureq::Agent,
    base: &str,
    token: &str,
    course_id: i64,
    now_ms: i64,
) -> AppResult<Vec<CanvasAssignment>> {
    let url = format!(
        "{base}/api/v1/courses/{course_id}/assignments?order_by=due_at&per_page=100&include%5B%5D=submission"
    );
    let resp = agent
        .get(&url)
        .set("Authorization", &format!("Bearer {token}"))
        .set("Accept", "application/json")
        .call()
        .map_err(map_ureq)?;
    let body = read_body(resp)?;
    let raw: Vec<RawAssignment> = serde_json::from_str(&body)
        .map_err(|err| AppError::Other(format!("Canvas 作业列表解析失败：{err}")))?;
    Ok(raw
        .into_iter()
        .filter_map(|a| {
            if a.published == Some(false) {
                return None;
            }
            let due_ms = parse_due(a.due_at.as_deref()?)?;
            if !due_in_window(due_ms, now_ms) {
                return None;
            }
            let name = a.name?;
            let html_url = a.html_url?;
            Some(CanvasAssignment {
                id: a.id,
                course_id: a.course_id,
                name,
                due_at: due_ms,
                html_url,
                points_possible: a.points_possible,
                submitted: submission_done(a.submission.and_then(|s| s.workflow_state).as_deref()),
            })
        })
        .collect())
}

fn sync_blocking(base: &str, token: &str) -> AppResult<CanvasSnapshot> {
    let agent = agent();
    let now = chrono::Utc::now().timestamp_millis();
    let courses = fetch_courses(&agent, base, token)?;
    let mut assignments = Vec::new();
    let mut failures = 0usize;
    let mut last_err: Option<AppError> = None;
    for course in &courses {
        match fetch_course_assignments(&agent, base, token, course.id, now) {
            Ok(mut list) => assignments.append(&mut list),
            Err(err) => {
                failures += 1;
                last_err = Some(err);
            }
        }
    }
    // Every course failing is not a "partial success" — surface the error.
    if failures > 0 && assignments.is_empty() && !courses.is_empty() {
        return Err(last_err.unwrap_or_else(|| AppError::Other("Canvas 同步失败".into())));
    }
    assignments.sort_by_key(|a| a.due_at);
    Ok(CanvasSnapshot {
        fetched_at: chrono::Utc::now().timestamp_millis(),
        courses,
        assignments,
        skipped_courses: failures,
    })
}

async fn run_blocking<T, F>(f: F) -> AppResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> AppResult<T> + Send + 'static,
{
    spawn_blocking(f)
        .await
        .map_err(|err| AppError::Other(format!("后台任务失败：{err}")))?
}

/// Verify a token by reading the caller's own profile. Used when saving a
/// token so a typo never lands in settings.
#[tauri::command]
pub async fn canvas_test_connection(base: String, token: String) -> AppResult<CanvasProfile> {
    let base = clean_base(&base)?;
    run_blocking(move || {
        let agent = agent();
        let url = format!("{base}/api/v1/users/self/profile");
        let resp = agent
            .get(&url)
            .set("Authorization", &format!("Bearer {token}"))
            .set("Accept", "application/json")
            .call()
            .map_err(map_ureq)?;
        let body = read_body(resp)?;
        #[derive(Deserialize)]
        struct RawProfile {
            name: Option<String>,
            login_id: Option<String>,
        }
        let raw: RawProfile = serde_json::from_str(&body)
            .map_err(|err| AppError::Other(format!("Canvas 个人信息解析失败：{err}")))?;
        Ok(CanvasProfile {
            name: raw
                .name
                .filter(|n| !n.trim().is_empty())
                .unwrap_or_else(|| "Canvas 用户".into()),
            login_id: raw.login_id,
        })
    })
    .await
}

/// Full read-only sync: active courses + their due-dated assignments.
#[tauri::command]
pub async fn canvas_sync(base: String, token: String) -> AppResult<CanvasSnapshot> {
    let base = clean_base(&base)?;
    run_blocking(move || sync_blocking(&base, &token)).await
}

/// Open an assignment's Canvas URL in the default browser. Only same-host
/// https links from the snapshot are accepted — never arbitrary URLs.
#[tauri::command]
pub fn canvas_open_link(base: String, url: String) -> AppResult<()> {
    let base = clean_base(&base)?;
    if !(url.starts_with(&format!("{base}/")) || url == base) {
        return Err(AppError::Other("已拒绝打开非 Canvas 来源的链接".into()));
    }
    open_with_shell(&url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_url_is_https_host_only() {
        assert_eq!(
            clean_base("https://oc.sjtu.edu.cn/").unwrap(),
            "https://oc.sjtu.edu.cn"
        );
        assert_eq!(
            clean_base("  https://oc.sjtu.edu.cn/courses ").unwrap(),
            "https://oc.sjtu.edu.cn"
        );
        assert!(clean_base("http://oc.sjtu.edu.cn").is_err());
        assert!(clean_base("https://").is_err());
        assert!(clean_base("https://user@host.com").is_err());
    }

    #[test]
    fn link_header_next_is_found() {
        let header = r#"<https://oc.sjtu.edu.cn/api/v1/courses?page=2&per_page=100>; rel="next", <https://oc.sjtu.edu.cn/api/v1/courses?page=1&per_page=100>; rel="prev""#;
        assert_eq!(
            next_link(header).as_deref(),
            Some("https://oc.sjtu.edu.cn/api/v1/courses?page=2&per_page=100")
        );
        assert_eq!(
            next_link(r#"<https://x/api?page=1>; rel="prev""#),
            None
        );
    }

    #[test]
    fn due_parsing_and_window() {
        let now: i64 = 1_760_000_000_000;
        assert_eq!(parse_due("2026-09-20T15:59:59Z"), Some(1_789_919_999_000));
        assert_eq!(parse_due("not-a-date"), None);
        assert!(due_in_window(now, now));
        assert!(due_in_window(now - 29 * DAY_MS, now));
        assert!(!due_in_window(now - 31 * DAY_MS, now));
        assert!(due_in_window(now + FUTURE_WINDOW_DAYS * DAY_MS, now));
        assert!(!due_in_window(now + (FUTURE_WINDOW_DAYS + 1) * DAY_MS, now));
    }

    #[test]
    fn workflow_states_mean_submitted() {
        assert!(submission_done(Some("submitted")));
        assert!(submission_done(Some("graded")));
        assert!(submission_done(Some("pending_review")));
        assert!(!submission_done(Some("unsubmitted")));
        assert!(!submission_done(None));
    }

    /// Manual probe against the real host with a bogus token: proves the TLS
    /// stack + the 401 → honest-message mapping. `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn live_401_probe() {
        let agent = agent();
        let err = agent
            .get("https://oc.sjtu.edu.cn/api/v1/users/self/profile")
            .set("Authorization", "Bearer definitely-not-a-token")
            .set("Accept", "application/json")
            .call()
            .err()
            .expect("bogus token must fail");
        let mapped = map_ureq(err).to_string();
        assert!(mapped.contains("401"), "expected 401 mapping, got: {mapped}");
    }
}
