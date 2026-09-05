//! RFC 5545 (iCalendar) export for calendar events (M6 deferred feature).
//!
//! Pure serialization — no IO — so it is fully unit tested. Datetimes are
//! written as *floating local time* (no TZID): the app stores wall-clock
//! epoch millis and the honest, lossless representation of that is a local
//! time without timezone claims. All-day events use `VALUE=DATE` with an
//! exclusive DTEND (the day after), as the RFC requires.

use chrono::{Datelike, Local, TimeZone, Timelike, Utc};

use crate::storage::calendar_repo::CalendarEvent;

/// Escape a TEXT value per RFC 5545 §3.3.11.
pub fn escape_text(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            ';' => out.push_str("\\;"),
            ',' => out.push_str("\\,"),
            '\n' => out.push_str("\\n"),
            '\r' => {} // folded away: bare CR is not meaningful text
            other => out.push(other),
        }
    }
    out
}

/// Fold one content line to the RFC's 75-octet limit, continuing with a
/// space. Breaks only on char boundaries so UTF-8 stays intact.
pub fn fold_line(line: &str) -> String {
    const FIRST_LIMIT: usize = 74; // 75 octets minus the joining space below
    const REST_LIMIT: usize = 73; // continuation: 1 space + 73 octets

    if line.len() <= FIRST_LIMIT {
        return line.to_string();
    }

    let mut out = String::with_capacity(line.len() + line.len() / FIRST_LIMIT * 3);
    let mut taken = 0usize;
    let mut limit = FIRST_LIMIT;
    for (idx, ch) in line.char_indices() {
        let ch_len = ch.len_utf8();
        if taken + ch_len > limit {
            out.push_str("\r\n ");
            taken = 0;
            limit = REST_LIMIT;
        }
        out.push_str(&line[idx..idx + ch_len]);
        taken += ch_len;
    }
    out
}

/// Millis since epoch → `YYYYMMDDTHHMMSS` in local wall-clock time.
fn local_stamp(millis: i64) -> String {
    let dt = Local.timestamp_millis_opt(millis).single().unwrap_or_else(|| {
        // Ambiguous/nonexistent local times (DST edges) collapse to UTC if
        // they cannot resolve; the export must not panic on them.
        Utc.timestamp_millis_opt(millis).single().unwrap_or_else(Utc::now).into()
    });
    format!(
        "{:04}{:02}{:02}T{:02}{:02}{:02}",
        dt.year(),
        dt.month(),
        dt.day(),
        dt.hour(),
        dt.minute(),
        dt.second()
    )
}

/// Millis since epoch → `YYYYMMDD` (local date).
fn local_date(millis: i64) -> String {
    let dt = Local.timestamp_millis_opt(millis).single();
    match dt {
        Some(dt) => format!("{:04}{:02}{:02}", dt.year(), dt.month(), dt.day()),
        None => "19700101".to_string(),
    }
}

fn event_lines(event: &CalendarEvent, dtstamp: &str) -> Vec<String> {
    let mut lines = vec![
        "BEGIN:VEVENT".to_string(),
        format!("UID:dm-event-{}@desktopmanager.local", event.id),
        format!("DTSTAMP:{dtstamp}"),
    ];
    if event.all_day {
        // DTEND is exclusive in iCalendar: add one day.
        let exclusive_end = Local
            .timestamp_millis_opt(event.ends_at)
            .single()
            .map(|dt| format!("{:04}{:02}{:02}", dt.year(), dt.month(), dt.day()))
            .unwrap_or_else(|| local_date(event.ends_at));
        lines.push(format!("DTSTART;VALUE=DATE:{}", local_date(event.starts_at)));
        lines.push(format!("DTEND;VALUE=DATE:{exclusive_end}"));
    } else {
        lines.push(format!("DTSTART:{}", local_stamp(event.starts_at)));
        lines.push(format!("DTEND:{}", local_stamp(event.ends_at)));
    }
    lines.push(format!("SUMMARY:{}", escape_text(&event.title)));
    if let Some(notes) = event.notes.as_deref() {
        if !notes.trim().is_empty() {
            lines.push(format!("DESCRIPTION:{}", escape_text(notes)));
        }
    }
    lines.push("END:VEVENT".to_string());
    lines
}

/// Serialize events into a complete VCALENDAR document with CRLF endings.
pub fn events_to_ics(events: &[CalendarEvent]) -> String {
    let dtstamp = {
        let now = Utc::now();
        format!(
            "{:04}{:02}{:02}T{:02}{:02}{:02}Z",
            now.year(),
            now.month(),
            now.day(),
            now.hour(),
            now.minute(),
            now.second()
        )
    };

    let mut lines = vec![
        "BEGIN:VCALENDAR".to_string(),
        "VERSION:2.0".to_string(),
        "PRODID:-//DesktopManager//Calendar//CN".to_string(),
        "CALSCALE:GREGORIAN".to_string(),
    ];
    for event in events {
        lines.extend(event_lines(event, &dtstamp));
    }
    lines.push("END:VCALENDAR".to_string());

    let folded: Vec<String> = lines.iter().map(|l| fold_line(l)).collect();
    let mut doc = folded.join("\r\n");
    doc.push_str("\r\n");
    doc
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(id: i64, title: &str, starts_at: i64, ends_at: i64, all_day: bool) -> CalendarEvent {
        CalendarEvent {
            id,
            title: title.to_string(),
            starts_at,
            ends_at,
            all_day,
            notes: None,
            color: None,
            task_id: None,
            created_at: 0,
            updated_at: 0,
        }
    }

    #[test]
    fn escape_covers_rfc_specials() {
        assert_eq!(escape_text("a;b,c\nd\\e"), "a\\;b\\,c\\nd\\\\e");
        assert_eq!(escape_text("bare\rcr"), "barecr");
    }

    #[test]
    fn fold_keeps_short_lines_and_breaks_long_ones() {
        assert_eq!(fold_line("short"), "short");
        let long = "x".repeat(200);
        let folded = fold_line(&format!("SUMMARY:{long}"));
        for line in folded.split("\r\n") {
            assert!(line.len() <= 75, "line is {} octets", line.len());
        }
        // Unfolded round trip returns the original.
        assert_eq!(folded.replace("\r\n ", ""), format!("SUMMARY:{long}"));
    }

    #[test]
    fn fold_does_not_split_utf8_characters() {
        let title = "中".repeat(60); // 180 bytes, 60 chars
        let folded = fold_line(&format!("SUMMARY:{title}"));
        assert_eq!(folded.replace("\r\n ", ""), format!("SUMMARY:{title}"));
        for line in folded.split("\r\n") {
            assert!(std::str::from_utf8(line.as_bytes()).is_ok());
        }
    }

    #[test]
    fn document_has_crlf_and_required_properties() {
        let e = event(7, "例会", 1_000_000_000_000, 1_000_003_600_000, false);
        let ics = events_to_ics(&[e]);
        assert!(ics.contains("BEGIN:VCALENDAR\r\n"));
        assert!(ics.contains("PRODID:-//DesktopManager//Calendar//CN\r\n"));
        assert!(ics.contains("UID:dm-event-7@desktopmanager.local\r\n"));
        assert!(ics.contains("SUMMARY:例会\r\n"));
        assert!(ics.ends_with("END:VCALENDAR\r\n"));
        assert!(!ics.contains("DTSTART;VALUE=DATE"));
        assert!(ics.contains("DTSTART:"));
        assert!(ics.contains("DTEND:"));
    }

    #[test]
    fn all_day_uses_date_values_with_exclusive_end() {
        // 2026-09-06 00:00 local → 2026-09-07 00:00 local (exclusive end is
        // stored the same way in the app, so a one-day event exports as
        // DTSTART 20260906 / DTEND 20260907 regardless of DST oddities).
        let e = event(3, "全天", 1_787_630_400_000, 1_787_716_800_000, true);
        let ics = events_to_ics(&[e]);
        assert!(ics.contains("DTSTART;VALUE=DATE:"));
        assert!(ics.contains("DTEND;VALUE=DATE:"));
        // The two dates must differ by exactly one day for a 1-day event.
        let start = ics.split("DTSTART;VALUE=DATE:").nth(1).unwrap();
        let start = &start[..8];
        let end = ics.split("DTEND;VALUE=DATE:").nth(1).unwrap();
        let end = &end[..8];
        assert_ne!(start, end);
    }

    #[test]
    fn notes_become_description_and_skip_blank() {
        let mut e = event(1, "t", 0, 1, false);
        e.notes = Some("line1\nline2, with; specials".to_string());
        let ics = events_to_ics(&[e]);
        assert!(ics.contains("DESCRIPTION:line1\\nline2\\, with\\; specials\r\n"));

        let mut e = event(1, "t", 0, 1, false);
        e.notes = Some("   ".to_string());
        let ics = events_to_ics(&[e]);
        assert!(!ics.contains("DESCRIPTION:"));
    }
}
