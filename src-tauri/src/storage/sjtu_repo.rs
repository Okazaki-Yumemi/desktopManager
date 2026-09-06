//! SJTU-synced calendar entries (migration 0009). The table is a read-only
//! projection of the university calendar service: every sync REPLACES all
//! rows in one transaction, so a re-sync can never leave stale rows behind.
//! Editing happens on the university side, not here — this repo has no
//! update/delete-per-row API on purpose.

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

    /// Replace the whole projection with `events` atomically. Users see
    /// either the previous or the new state, never a half-written sync.
    pub fn replace_all(&self, events: &[SjtuEvent]) -> AppResult<usize> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM sjtu_events", [])?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO sjtu_events (external_id, title, location, starts_at, ends_at,
                                           all_day, status, recurrence, source, calendar_id,
                                           synced_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
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
    fn replace_all_is_atomic_and_repeatable() {
        let mut db = Database::open_in_memory().unwrap();
        let r = repo(&mut db);
        let first = vec![
            event("ev:a:1", "第一课", 1_000),
            event("ev:b:2", "第二课", 2_000),
        ];
        assert_eq!(r.replace_all(&first).unwrap(), 2);

        // A second sync with an overlapping-but-different set must fully
        // replace the projection — no leftovers, no duplicates.
        let second = vec![
            event("ev:b:2", "第二课（改）", 2_000),
            event("ev:c:3", "第三课", 3_000),
        ];
        assert_eq!(r.replace_all(&second).unwrap(), 2);
        let all = r.list_all().unwrap();
        assert_eq!(
            all.iter().map(|e| e.external_id.as_str()).collect::<Vec<_>>(),
            vec!["ev:b:2", "ev:c:3"]
        );
        assert_eq!(all[0].title, "第二课（改）");
    }

    #[test]
    fn list_all_orders_by_start_time() {
        let mut db = Database::open_in_memory().unwrap();
        let r = repo(&mut db);
        r.replace_all(&[
            event("ev:c:3", "晚课", 30_000),
            event("ev:a:1", "早课", 10_000),
        ])
        .unwrap();
        let all = r.list_all().unwrap();
        assert_eq!(all[0].title, "早课");
        assert_eq!(all[1].title, "晚课");
    }

    #[test]
    fn clear_empties_the_projection() {
        let mut db = Database::open_in_memory().unwrap();
        let r = repo(&mut db);
        r.replace_all(&[event("ev:a:1", "课", 1_000)]).unwrap();
        assert_eq!(r.clear().unwrap(), 1);
        assert!(r.list_all().unwrap().is_empty());
        assert_eq!(r.clear().unwrap(), 0);
    }
}
