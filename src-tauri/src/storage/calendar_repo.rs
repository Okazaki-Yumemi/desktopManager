//! Local calendar events (migration 0004). Times are epoch millis; the
//! backend never does local-time math — day/week bucketing happens in the
//! UI, and queries take explicit [from, to) ranges.

use rusqlite::params;
use rusqlite::OptionalExtension;
use serde::Serialize;

use crate::app::error::{AppError, AppResult};
use crate::app::logging::now_millis;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarEvent {
    pub id: i64,
    pub title: String,
    pub starts_at: i64,
    pub ends_at: i64,
    pub all_day: bool,
    pub notes: Option<String>,
    pub color: Option<String>,
    pub task_id: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

pub struct CalendarRepo<'a> {
    conn: &'a rusqlite::Connection,
}

impl<'a> CalendarRepo<'a> {
    pub fn new(conn: &'a rusqlite::Connection) -> Self {
        Self { conn }
    }

    fn map(row: &rusqlite::Row<'_>) -> rusqlite::Result<CalendarEvent> {
        Ok(CalendarEvent {
            id: row.get(0)?,
            title: row.get(1)?,
            starts_at: row.get(2)?,
            ends_at: row.get(3)?,
            all_day: row.get::<_, i64>(4)? != 0,
            notes: row.get(5)?,
            color: row.get(6)?,
            task_id: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
        })
    }

    const COLS: &'static str = "id, title, starts_at, ends_at, all_day, notes, color, task_id,
        created_at, updated_at";

    /// Events overlapping [from, to), soonest first. All-day events are
    /// included when their day range intersects the window (all-day events
    /// store starts_at = day start, ends_at = day end in local ms, decided
    /// by the UI).
    pub fn list_range(&self, from: i64, to: i64) -> AppResult<Vec<CalendarEvent>> {
        if to <= from {
            return Err(AppError::Other("时间范围无效".into()));
        }
        let mut stmt = self.conn.prepare_cached(&format!(
            "SELECT {} FROM calendar_events
             WHERE starts_at < ?2 AND ends_at > ?1
             ORDER BY all_day DESC, starts_at",
            Self::COLS
        ))?;
        let rows = stmt.query_map(params![from, to], Self::map)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Every event, soonest first (used by ICS export — no time window).
    pub fn list_all(&self) -> AppResult<Vec<CalendarEvent>> {
        let mut stmt = self.conn.prepare_cached(&format!(
            "SELECT {} FROM calendar_events ORDER BY starts_at",
            Self::COLS
        ))?;
        let rows = stmt.query_map([], Self::map)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    fn validate(&self, title: &str, starts_at: i64, ends_at: i64) -> AppResult<String> {
        let title = title.trim();
        if title.is_empty() {
            return Err(AppError::Other("日程标题不能为空".into()));
        }
        if ends_at <= starts_at {
            return Err(AppError::Other("结束时间必须晚于开始时间".into()));
        }
        Ok(title.to_owned())
    }

    fn check_task(&self, task_id: Option<i64>) -> AppResult<()> {
        if let Some(tid) = task_id {
            let known: i64 = self
                .conn
                .query_row("SELECT COUNT(*) FROM tasks WHERE id = ?1", [tid], |r| {
                    r.get(0)
                })?;
            if known == 0 {
                return Err(AppError::Other("关联的任务不存在".into()));
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create(
        &self,
        title: &str,
        starts_at: i64,
        ends_at: i64,
        all_day: bool,
        notes: Option<&str>,
        color: Option<&str>,
        task_id: Option<i64>,
    ) -> AppResult<CalendarEvent> {
        let title = self.validate(title, starts_at, ends_at)?;
        self.check_task(task_id)?;
        let now = now_millis();
        self.conn.execute(
            "INSERT INTO calendar_events (title, starts_at, ends_at, all_day, notes, color,
                                          task_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
            params![
                title,
                starts_at,
                ends_at,
                all_day as i64,
                notes.map(str::trim).filter(|s| !s.is_empty()),
                color.map(str::trim).filter(|s| !s.is_empty()),
                task_id,
                now
            ],
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(self.get(id)?.expect("row just inserted"))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &self,
        id: i64,
        title: &str,
        starts_at: i64,
        ends_at: i64,
        all_day: bool,
        notes: Option<&str>,
        color: Option<&str>,
        task_id: Option<i64>,
    ) -> AppResult<()> {
        let title = self.validate(title, starts_at, ends_at)?;
        self.check_task(task_id)?;
        let changed = self.conn.execute(
            "UPDATE calendar_events SET title = ?2, starts_at = ?3, ends_at = ?4, all_day = ?5,
                    notes = ?6, color = ?7, task_id = ?8, updated_at = ?9
             WHERE id = ?1",
            params![
                id,
                title,
                starts_at,
                ends_at,
                all_day as i64,
                notes.map(str::trim).filter(|s| !s.is_empty()),
                color.map(str::trim).filter(|s| !s.is_empty()),
                task_id,
                now_millis()
            ],
        )?;
        if changed == 0 {
            return Err(AppError::Other("日程不存在".into()));
        }
        Ok(())
    }

    /// Quick time-block edit: move an event's window without touching its
    /// other fields.
    pub fn reschedule(&self, id: i64, starts_at: i64, ends_at: i64) -> AppResult<()> {
        if ends_at <= starts_at {
            return Err(AppError::Other("结束时间必须晚于开始时间".into()));
        }
        let changed = self.conn.execute(
            "UPDATE calendar_events SET starts_at = ?2, ends_at = ?3, updated_at = ?4
             WHERE id = ?1",
            params![id, starts_at, ends_at, now_millis()],
        )?;
        if changed == 0 {
            return Err(AppError::Other("日程不存在".into()));
        }
        Ok(())
    }

    pub fn delete(&self, id: i64) -> AppResult<bool> {
        let changed = self
            .conn
            .execute("DELETE FROM calendar_events WHERE id = ?1", [id])?;
        Ok(changed > 0)
    }

    pub fn get(&self, id: i64) -> AppResult<Option<CalendarEvent>> {
        self.conn
            .query_row(
                &format!("SELECT {} FROM calendar_events WHERE id = ?1", Self::COLS),
                [id],
                Self::map,
            )
            .optional()
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;

    fn repo(db: &mut Database) -> CalendarRepo<'_> {
        CalendarRepo::new(db.conn())
    }

    #[test]
    fn create_and_range_query() {
        let mut db = Database::open_in_memory().unwrap();
        let r = repo(&mut db);
        let e1 = r
            .create("早会", 1_000, 2_000, false, Some(" 站会 "), Some("#ff0000"), None)
            .unwrap();
        assert_eq!(e1.notes.as_deref(), Some("站会"));
        let e2 = r.create("全天", 5_000, 6_000, true, None, None, None).unwrap();
        assert!(e2.all_day);

        assert!(r.list_range(0, 1_000).unwrap().is_empty()); // [from, to)
        let in_window = r.list_range(1_500, 1_600).unwrap();
        assert_eq!(in_window.len(), 1); // only the overlapping one
        let wide = r.list_range(0, 10_000).unwrap();
        assert_eq!(wide.len(), 2);
        assert_eq!(wide[0].id, e2.id); // all-day first, then starts_at

        assert!(r.create("", 0, 1, false, None, None, None).is_err());
        assert!(r.create("x", 2_000, 2_000, false, None, None, None).is_err());
        assert!(r.list_range(5_000, 5_000).is_err());
    }

    #[test]
    fn task_link_respects_foreign_key() {
        let mut db = Database::open_in_memory().unwrap();
        let tid: i64 = {
            let conn = db.conn();
            conn.execute(
                "INSERT INTO tasks (title, status, priority, created_at, updated_at)
                 VALUES ('写周报', 'todo', 0, 1, 1)",
                [],
            )
            .unwrap();
            conn.last_insert_rowid()
        };
        let e = repo(&mut db)
            .create("做周报", 100, 200, false, None, None, Some(tid))
            .unwrap();
        assert_eq!(e.task_id, Some(tid));
        assert!(repo(&mut db)
            .create("x", 100, 200, false, None, None, Some(9999))
            .is_err());

        repo(&mut db).update(e.id, "改期", 300, 400, false, None, None, None).unwrap();
        let updated = repo(&mut db).get(e.id).unwrap().unwrap();
        assert_eq!(updated.starts_at, 300);
        assert!(updated.task_id.is_none());

        repo(&mut db).reschedule(e.id, 500, 600).unwrap();
        assert_eq!(repo(&mut db).get(e.id).unwrap().unwrap().starts_at, 500);
        assert!(repo(&mut db).reschedule(e.id, 900, 900).is_err());

        assert!(repo(&mut db).delete(e.id).unwrap());
        assert!(repo(&mut db).get(e.id).unwrap().is_none());
    }
}
