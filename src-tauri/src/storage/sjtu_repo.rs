//! SJTU-synced calendar entries (migration 0009). The table is a read-only
//! projection of the university calendar service. Since D29 a sync session
//! pushes one payload per week (the portal hands out a week at a time), so
//! receives MERGE by external_id instead of replacing the table; rows that
//! ended more than two days ago are pruned with each merge so the projection
//! cannot grow unboundedly across a term. Editing happens on the university
//! side, not here.

use rusqlite::params;
use serde::Serialize;

use crate::app::error::AppResult;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SjtuEvent {
    pub id: i64,
    /// Stable identity from the university payload (event id + start time),
    /// kept for traceability even though the whole table is replaced per sync.
    pub external_id: String,
    pub title: String,
    pub location: Option<String>,
    pub starts_at: i64,
    pub ends_at: i64,
    pub all_day: bool,
    pub status: Option<String>,
    pub recurrence: Option<i64>,
    /// "personal" (course/personal calendar) or "school" (校历 events).
    pub source: String,
    pub calendar_id: Option<String>,
    pub synced_at: i64,
}

pub struct SjtuRepo<'a> {
    conn: &'a rusqlite::Connection,
}

impl<'a> SjtuRepo<'a> {
    pub fn new(conn: &'a rusqlite::Connection) -> Self {
        Self { conn }
    }

    const COLS: &'static str = "id, external_id, title, location, starts_at, ends_at, all_day,
        status, recurrence, source, calendar_id, synced_at";

    fn map(row: &rusqlite::Row<'_>) -> rusqlite::Result<SjtuEvent> {
        Ok(SjtuEvent {
            id: row.get(0)?,
            external_id: row.get(1)?,
            title: row.get(2)?,
            location: row.get(3)?,
            starts_at: row.get(4)?,
            ends_at: row.get(5)?,
            all_day: row.get::<_, i64>(6)? != 0,
            status: row.get(7)?,
            recurrence: row.get(8)?,
            source: row.get(9)?,
            calendar_id: row.get(10)?,
            synced_at: row.get(11)?,
        })
    }

    /// Merge one pushed payload into the projection atomically: new external
    /// ids are inserted, known ones updated in place. Rows that ended more
    /// than seven days before `now_ms` are pruned in the same transaction —
    /// the last week stays viewable in the calendar (the whole point of the
    /// Sunday sync is next week, but looking back one week is normal), while
    /// a term of weekly pushes stays bounded. Users see either the previous
    /// or the new state, never a half sync.
    pub fn upsert_events(&self, events: &[SjtuEvent], now_ms: i64) -> AppResult<usize> {
        const DAY_MS: i64 = 86_400_000;
        const RETAIN_PAST_DAYS: i64 = 7;
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO sjtu_events (external_id, title, location, starts_at, ends_at,
                                           all_day, status, recurrence, source, calendar_id,
                                           synced_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                 ON CONFLICT(external_id) DO UPDATE SET
                   title = excluded.title,
                   location = excluded.location,
                   starts_at = excluded.starts_at,
                   ends_at = excluded.ends_at,
                   all_day = excluded.all_day,
                   status = excluded.status,
                   recurrence = excluded.recurrence,
                   source = excluded.source,
                   calendar_id = excluded.calendar_id,
                   synced_at = excluded.synced_at",
            )?;
            for e in events {
                stmt.execute(params![
                    e.external_id,
                    e.title,
                    e.location,
                    e.starts_at,
                    e.ends_at,
                    e.all_day as i64,
                    e.status,
                    e.recurrence,
                    e.source,
                    e.calendar_id,
                    e.synced_at,
                ])?;
            }
        }
        tx.execute(
            "DELETE FROM sjtu_events WHERE ends_at < ?1",
            [now_ms - RETAIN_PAST_DAYS * DAY_MS],
        )?;
        tx.commit()?;
        Ok(events.len())
    }

    /// Every synced entry, soonest first.
    pub fn list_all(&self) -> AppResult<Vec<SjtuEvent>> {
        let mut stmt = self
            .conn
            .prepare_cached(&format!(
                "SELECT {} FROM sjtu_events ORDER BY starts_at, title",
                Self::COLS
            ))?;
        let rows = stmt.query_map([], Self::map)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Drop the projection (Settings "clear" button). Re-syncing rebuilds it.
    pub fn clear(&self) -> AppResult<usize> {
        Ok(self.conn.execute("DELETE FROM sjtu_events", [])?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;

    fn repo(db: &mut Database) -> SjtuRepo<'_> {
        SjtuRepo::new(db.conn())
    }

    fn event(external_id: &str, title: &str, starts_at: i64) -> SjtuEvent {
        SjtuEvent {
            id: 0,
            external_id: external_id.to_string(),
            title: title.to_string(),
            location: None,
            starts_at,
            ends_at: starts_at + 3_600_000,
            all_day: false,
            status: None,
            recurrence: None,
            source: "personal".into(),
            calendar_id: None,
            synced_at: 1,
        }
    }

    #[test]
    fn upsert_merges_weeks_updates_and_prunes() {
        const DAY: i64 = 86_400_000;
        let mut db = Database::open_in_memory().unwrap();
        let r = repo(&mut db);
        let t1 = 100 * DAY;

        // Week 1 push: two current events.
        let week1 = vec![
            event("ev:a:1", "第一周旧课", t1),
            event("ev:b:2", "下周同一门课", t1 + 7 * DAY),
        ];
        assert_eq!(r.upsert_events(&week1, t1).unwrap(), 2);

        // Ten days later the next pushes arrive: b is re-pushed with an
        // updated title (must update in place, not duplicate), c is new, and
        // a — which ended more than two days before `now` — must be pruned.
        let week2 = vec![
            event("ev:b:2", "下周同一门课（时间调整）", t1 + 7 * DAY),
            event("ev:c:3", "第三周新课", t1 + 14 * DAY),
        ];
        assert_eq!(r.upsert_events(&week2, t1 + 10 * DAY).unwrap(), 2);

        let all = r.list_all().unwrap();
        assert_eq!(
            all.iter().map(|e| e.external_id.as_str()).collect::<Vec<_>>(),
            vec!["ev:b:2", "ev:c:3"],
            "merge is a union of pushes minus the stale past"
        );
        assert_eq!(all[0].title, "下周同一门课（时间调整）");
    }

    #[test]
    fn upsert_keeps_events_that_ended_recently() {
        const DAY: i64 = 86_400_000;
        let mut db = Database::open_in_memory().unwrap();
        let r = repo(&mut db);
        let now = 100 * DAY;
        // Ended 3 days ago: within the seven-day retention window.
        let mut recent = event("ev:y:1", "三天前下课", now - 3 * DAY);
        recent.ends_at = now - 3 * DAY + 3_600_000;
        // Ended 9 days ago: outside it.
        let mut stale = event("ev:o:2", "九天前下课", now - 9 * DAY);
        stale.ends_at = now - 9 * DAY + 3_600_000;
        r.upsert_events(&[recent, stale], now).unwrap();
        let all = r.list_all().unwrap();
        assert_eq!(
            all.iter().map(|e| e.external_id.as_str()).collect::<Vec<_>>(),
            vec!["ev:y:1"]
        );
    }

    #[test]
    fn list_all_orders_by_start_time() {
        let mut db = Database::open_in_memory().unwrap();
        let r = repo(&mut db);
        r.upsert_events(
            &[
                event("ev:c:3", "晚课", 30_000),
                event("ev:a:1", "早课", 10_000),
            ],
            100_000,
        )
        .unwrap();
        let all = r.list_all().unwrap();
        assert_eq!(all[0].title, "早课");
        assert_eq!(all[1].title, "晚课");
    }

    #[test]
    fn clear_empties_the_projection() {
        let mut db = Database::open_in_memory().unwrap();
        let r = repo(&mut db);
        r.upsert_events(&[event("ev:a:1", "课", 1_000)], 100_000)
            .unwrap();
        assert_eq!(r.clear().unwrap(), 1);
        assert!(r.list_all().unwrap().is_empty());
        assert_eq!(r.clear().unwrap(), 0);
    }
}
