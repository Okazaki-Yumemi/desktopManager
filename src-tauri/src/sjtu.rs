//! SJTU (my.sjtu.edu.cn) calendar payload parsing (M12).
//!
//! The sync webview fetches the university calendar API same-origin and
//! pushes the raw JSON body into `sjtu_receive`. This module maps that
//! payload onto `SjtuEvent` rows. Parsing is deliberately tolerant: unknown
//! fields are ignored, events with unusable times are skipped (and counted),
//! and either the API's `{status, data: {...}}` wrapper or a bare object is
//! accepted. Times are "YYYY-MM-DD HH:MM" without a zone — they are taken as
//! local machine time (the app has no timezone model; see D23/D24).

use chrono::{Local, TimeZone};
use serde::Deserialize;

use crate::app::error::AppError;
use crate::storage::sjtu_repo::SjtuEvent;

/// Hard cap on accepted payload size. A term's calendar is a few hundred KiB
/// at most; anything larger is not a calendar response.
pub const MAX_PAYLOAD_BYTES: usize = 2_000_000;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawEvent {
    event_id: Option<String>,
    title: Option<String>,
    start_time: Option<String>,
    end_time: Option<String>,
    location: Option<String>,
    all_day: Option<bool>,
    status: Option<String>,
    recurrence: Option<i64>,
    calendar_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SchoolCalendar {
    events: Option<Vec<RawEvent>>,
    #[allow(dead_code)]
    weeks: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Data {
    events: Option<Vec<RawEvent>>,
    school_calendar: Option<SchoolCalendar>,
}

#[derive(Debug)]
pub struct ParsedSjtu {
    pub events: Vec<SjtuEvent>,
    pub skipped: usize,
}
/// Map a raw API body onto rows. `synced_at` stamps every row so the UI can
/// show when the projection was refreshed.
pub fn parse_payload(payload: &str, synced_at: i64) -> Result<ParsedSjtu, AppError> {
    if payload.len() > MAX_PAYLOAD_BYTES {
        return Err(AppError::Other(
            "交大日程返回数据过大，已拒绝接收".into(),
        ));
    }
    let value: serde_json::Value = serde_json::from_str(payload)
        .map_err(|err| AppError::Other(format!("交大日程返回不是有效 JSON：{err}")))?;
    // Accept both the wrapper `{status, msg, data}` and a bare `{events, ...}`.
    let node = if value
        .get("data")
        .and_then(|d| d.as_object())
        .is_some_and(|d| d.contains_key("events") || d.contains_key("schoolCalendar"))
    {
        &value["data"]
    } else {
        &value
    };
    let data: Data = serde_json::from_value(node.clone())
        .map_err(|err| AppError::Other(format!("交大日程数据结构无法识别：{err}")))?;

    let mut events = Vec::new();
    let mut skipped = 0usize;
    for raw in data.events.unwrap_or_default() {
        push_event(&mut events, &mut skipped, raw, "personal", synced_at);
    }
    if let Some(school) = data.school_calendar {
        for raw in school.events.unwrap_or_default() {
            push_event(&mut events, &mut skipped, raw, "school", synced_at);
        }
    }
    Ok(ParsedSjtu { events, skipped })
}

fn push_event(
    out: &mut Vec<SjtuEvent>,
    skipped: &mut usize,
    raw: RawEvent,
    source: &str,
    synced_at: i64,
) {
    let title = raw
        .title
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty());
    let (Some(title), Some(start_s), Some(end_s)) = (title, raw.start_time, raw.end_time) else {
        *skipped += 1;
        return;
    };
    let (Some(starts_at), Some(ends_at)) = (parse_local_ms(&start_s), parse_local_ms(&end_s))
    else {
        *skipped += 1;
        return;
    };
    if ends_at <= starts_at {
        *skipped += 1;
        return;
    }
    let location = raw
        .location
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty());
    // School-calendar events are day spans (00:00 → 23:59); treat any such
    // pair as an all-day entry even when the flag is absent.
    let all_day = raw.all_day.unwrap_or(false) || is_full_day(&start_s, &end_s);
    let external_id = match source {
        "school" => format!("school:{title}:{start_s}"),
        _ => format!("ev:{}:{start_s}", raw.event_id.as_deref().unwrap_or("anon")),
    };
    out.push(SjtuEvent {
        id: 0,
        external_id,
        title,
        location,
        starts_at,
        ends_at,
        all_day,
        status: raw.status,
        recurrence: raw.recurrence,
        source: source.to_string(),
        calendar_id: raw.calendar_id,
        synced_at,
    });
}

fn is_full_day(start: &str, end: &str) -> bool {
    start.ends_with(" 00:00") && end.ends_with(" 23:59")
}

/// Local wall-clock time without a zone → epoch millis. Plain dates become
/// midnight. Events landing in a DST gap (none in Asia/Shanghai today) are
/// skipped by the caller via `None`.
fn parse_local_ms(s: &str) -> Option<i64> {
    let s = s.trim();
    let naive = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S"))
        .map(Some)
        .unwrap_or_else(|_| {
            chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .ok()
                .and_then(|d| d.and_hms_opt(0, 0, 0))
        })?;
    match Local.from_local_datetime(&naive) {
        chrono::LocalResult::Single(dt) | chrono::LocalResult::Ambiguous(dt, _) => {
            Some(dt.timestamp_millis())
        }
        chrono::LocalResult::None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The exact response shape captured from the university portal.
    const SAMPLE: &str = r#"{
        "status": 200,
        "msg": "success",
        "success": true,
        "data": {
            "schoolCalendar": {
                "weeks": [
                    {"weekEn": "Week 1", "week": "第一周", "titleEn": "Autumn Term",
                     "startTime": "2026-09-14 00:00", "endTime": "2026-09-20 23:59",
                     "title": "秋季学期"}
                ],
                "calendarId": "schoolCalendar",
                "events": [
                    {"titleEn": "开学", "startTime": "2026-09-14 00:00",
                     "endTime": "2026-09-14 23:59", "title": "开学"}
                ]
            },
            "events": [
                {"eventId": "a5ee9877-de99-43c9-99fa-58a2384b4312",
                 "title": "职业生涯发展与规划", "recurrence": 1, "allDay": false,
                 "calendarId": "27fde391-1dc5-43b1-bbe3-956c7991bfe6",
                 "claimIcon": false, "titleEn": "Career Development and Planning",
                 "locationEn": null, "location": "上院309",
                 "startTime": "2026-09-17 16:00", "endTime": "2026-09-17 17:40",
                 "status": "在忙"},
                {"eventId": "d06dfe76-4f6e-4477-b9f3-189041c522b0",
                 "title": "习近平新时代中国特色社会主义思想概论", "recurrence": 1,
                 "allDay": false, "calendarId": "27fde391-1dc5-43b1-bbe3-956c7991bfe6",
                 "claimIcon": false, "titleEn": null, "locationEn": null,
                 "location": "东上院115", "startTime": "2026-09-18 12:55",
                 "endTime": "2026-09-18 15:40", "status": "在忙"},
                {"eventId": "e86bb2f5-b208-477c-9a8a-d3048e680c92",
                 "title": "马克思主义基本原理", "recurrence": 1, "allDay": false,
                 "calendarId": "27fde391-1dc5-43b1-bbe3-956c7991bfe6",
                 "claimIcon": false, "titleEn": null, "locationEn": null,
                 "startTime": "2026-09-16 18:00", "location": "上院215",
                 "endTime": "2026-09-16 20:20", "status": "在忙"},
                {"eventId": "e9c323ba-d7d1-41db-a873-95439d2e52f2",
                 "title": "橄榄球", "recurrence": 1, "allDay": false,
                 "calendarId": "27fde391-1dc5-43b1-bbe3-956c7991bfe6",
                 "claimIcon": false, "titleEn": null, "locationEn": null,
                 "startTime": "2026-09-15 10:00", "location": ".",
                 "endTime": "2026-09-15 11:40", "status": "在忙"}
            ]
        }
    }"#;

    fn local_ms(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> i64 {
        Local
            .from_local_datetime(&chrono::NaiveDate::from_ymd_opt(y, mo, d)
                .unwrap()
                .and_hms_opt(h, mi, 0)
                .unwrap())
            .single()
            .unwrap()
            .timestamp_millis()
    }

    #[test]
    fn sample_payload_maps_all_events() {
        let parsed = parse_payload(SAMPLE, 77).unwrap();
        // 4 personal events + 1 school event; the weeks array is ignored.
        assert_eq!(parsed.events.len(), 5);
        assert_eq!(parsed.skipped, 0);

        let first = &parsed.events[0];
        assert_eq!(first.title, "职业生涯发展与规划");
        assert_eq!(first.location.as_deref(), Some("上院309"));
        assert_eq!(first.source, "personal");
        assert_eq!(first.starts_at, local_ms(2026, 9, 17, 16, 0));
        assert_eq!(first.ends_at, local_ms(2026, 9, 17, 17, 40));
        assert_eq!(first.recurrence, Some(1));
        assert_eq!(first.status.as_deref(), Some("在忙"));
        assert_eq!(first.synced_at, 77);
        assert!(!first.all_day);
        assert_eq!(
            first.external_id,
            "ev:a5ee9877-de99-43c9-99fa-58a2384b4312:2026-09-17 16:00"
        );

        // School-calendar day spans become all-day entries.
        let school = parsed
            .events
            .iter()
            .find(|e| e.source == "school")
            .expect("school event must be mapped");
        assert_eq!(school.title, "开学");
        assert!(school.all_day);
        assert_eq!(school.starts_at, local_ms(2026, 9, 14, 0, 0));
        assert!(school.external_id.starts_with("school:开学:"));
    }

    #[test]
    fn bare_payload_without_wrapper_is_accepted() {
        let bare = r#"{"events": [{"eventId": "x", "title": "课", "startTime": "2026-09-17 16:00", "endTime": "2026-09-17 17:40"}]}"#;
        let parsed = parse_payload(bare, 1).unwrap();
        assert_eq!(parsed.events.len(), 1);
        assert_eq!(parsed.events[0].title, "课");
    }

    #[test]
    fn unusable_events_are_skipped_and_counted() {
        let raw = r#"{"events": [
            {"eventId": "a", "title": "  ", "startTime": "2026-09-17 16:00", "endTime": "2026-09-17 17:40"},
            {"eventId": "b", "title": "缺时间"},
            {"eventId": "c", "title": "坏时间", "startTime": "不是时间", "endTime": "2026-09-17 17:40"},
            {"eventId": "d", "title": "倒置", "startTime": "2026-09-17 18:00", "endTime": "2026-09-17 17:40"},
            {"eventId": "e", "title": "好课", "startTime": "2026-09-17 16:00", "endTime": "2026-09-17 17:40"}
        ]}"#;
        let parsed = parse_payload(raw, 1).unwrap();
        assert_eq!(parsed.events.len(), 1);
        assert_eq!(parsed.skipped, 4);
        assert_eq!(parsed.events[0].title, "好课");
    }

    #[test]
    fn invalid_json_is_a_clear_error() {
        let err = parse_payload("not json", 1).unwrap_err();
        assert!(err.to_string().contains("JSON"));
    }

    #[test]
    fn oversized_payload_is_rejected() {
        let big = format!("\"{}\"", "x".repeat(MAX_PAYLOAD_BYTES + 1));
        assert!(parse_payload(&big, 1).is_err());
    }

    #[test]
    fn seconds_format_and_plain_dates_parse() {
        let raw = r#"{"events": [
            {"eventId": "a", "title": "带秒", "startTime": "2026-09-17 16:00:00", "endTime": "2026-09-17 17:40:00"},
            {"eventId": "b", "title": "纯日期", "startTime": "2026-09-17", "endTime": "2026-09-18"}
        ]}"#;
        let parsed = parse_payload(raw, 1).unwrap();
        assert_eq!(parsed.events.len(), 2);
        assert_eq!(parsed.events[0].starts_at, local_ms(2026, 9, 17, 16, 0));
        // A plain date pair is not "00:00 → 23:59" text, so not all-day.
        assert!(!parsed.events[1].all_day);
    }
}
