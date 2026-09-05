//! Focus sessions (M5): pomodoro / custom / count-up blocks in the
//! focus_sessions table from migration 0005.
//!
//! The database row is the single source of truth for a running session:
//! `started_at` is written once at start and elapsed time is derived from it
//! at read time, so a crash, restart or webview reload cannot lose a running
//! session. Breaks between focus blocks are UI-only and are not persisted.
//! The `interrupted` status exists in the schema but V1 ends sessions as
//! `completed` or `abandoned`; mid-session interruptions are tallied in the
//! `interruptions` counter instead.

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::app::error::{AppError, AppResult};
use crate::app::logging::now_millis;

pub const KIND_POMODORO: &str = "pomodoro";
pub const KIND_CUSTOM: &str = "custom";
pub const KIND_COUNT_UP: &str = "count_up";

const KINDS: [&str; 3] = [KIND_POMODORO, KIND_CUSTOM, KIND_COUNT_UP];
const END_STATUSES: [&str; 2] = ["completed", "abandoned"];

const COLS: &str = "id, task_id, scene_id, kind, planned_duration_s, \
                    actual_duration_s, status, started_at, ended_at, \
                    interruptions, note";

/// One focus block (mirrored by src/types/domain.ts FocusSession).
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FocusSession {
    pub id: i64,
    pub task_id: Option<i64>,
    pub scene_id: Option<i64>,
    pub kind: String,
    pub planned_duration_s: i64,
    pub actual_duration_s: i64,
    pub status: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub interruptions: i64,
    pub note: Option<String>,
}

/// Aggregated focus time for one local-calendar day (YYYY-MM-DD).
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FocusDay {
    pub day: String,
    pub total_s: i64,
    pub sessions: i64,
    pub interruptions: i64,
}

fn row_to_session(row: &rusqlite::Row<'_>) -> rusqlite::Result<FocusSession> {
    Ok(FocusSession {
        id: row.get(0)?,
        task_id: row.get(1)?,
        scene_id: row.get(2)?,
        kind: row.get(3)?,
        planned_duration_s: row.get(4)?,
        actual_duration_s: row.get(5)?,
        status: row.get(6)?,
        started_at: row.get(7)?,
        ended_at: row.get(8)?,
        interruptions: row.get(9)?,
        note: row.get(10)?,
    })
}

pub struct FocusRepo<'a> {
    conn: &'a Connection,
}

impl<'a> FocusRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    fn get(&self, id: i64) -> AppResult<FocusSession> {
        self.conn
            .query_row(
                &format!("SELECT {COLS} FROM focus_sessions WHERE id = ?1"),
                [id],
                row_to_session,
            )
            .optional()?
            .ok_or_else(|| AppError::Other("专注会话不存在".into()))
    }

    /// Start a new session; refused while another one is running so zombie
    /// running rows can never accumulate. `task_id`/`scene_id` are validated
    /// when present (the foreign keys may not be enforced by the pragma).
    /// Count-up sessions store planned_duration_s = 0.
    pub fn start(
        &self,
        kind: &str,
        planned_duration_s: i64,
        task_id: Option<i64>,
        scene_id: Option<i64>,
    ) -> AppResult<FocusSession> {
        if !KINDS.contains(&kind) {
            return Err(AppError::Other("未知的专注类型".into()));
        }
        let planned = if kind == KIND_COUNT_UP {
            0
        } else {
            planned_duration_s
        };
        if kind != KIND_COUNT_UP && planned <= 0 {
            return Err(AppError::Other("专注时长必须大于 0 秒".into()));
        }
        if self.running()?.is_some() {
            return Err(AppError::Other("已有进行中的专注会话，请先结束它".into()));
        }
        if let Some(task_id) = task_id {
            let ok: i64 =
                self.conn
                    .query_row("SELECT COUNT(*) FROM tasks WHERE id = ?1", [task_id], |r| {
                        r.get(0)
                    })?;
            if ok == 0 {
                return Err(AppError::Other("任务不存在".into()));
            }
        }
        if let Some(scene_id) = scene_id {
            let ok: i64 = self.conn.query_row(
                "SELECT COUNT(*) FROM scenes WHERE id = ?1",
                [scene_id],
                |r| r.get(0),
            )?;
            if ok == 0 {
                return Err(AppError::Other("场景不存在".into()));
            }
        }
        self.conn.execute(
            "INSERT INTO focus_sessions
                 (task_id, scene_id, kind, planned_duration_s, status, started_at)
             VALUES (?1, ?2, ?3, ?4, 'running', ?5)",
            rusqlite::params![task_id, scene_id, kind, planned, now_millis()],
        )?;
        self.get(self.conn.last_insert_rowid())
    }

    /// The current running session, if any — the recovery path after a
    /// restart: elapsed time keeps counting from started_at.
    pub fn running(&self) -> AppResult<Option<FocusSession>> {
        self.conn
            .query_row(
                &format!(
                    "SELECT {COLS} FROM focus_sessions
                     WHERE status = 'running' ORDER BY id DESC LIMIT 1"
                ),
                [],
                row_to_session,
            )
            .optional()
            .map_err(Into::into)
    }

    /// End a session as completed or abandoned; actual duration is derived
    /// from the wall clock (ended_at - started_at) in whole seconds.
    pub fn finish(&self, id: i64, status: &str) -> AppResult<FocusSession> {
        if !END_STATUSES.contains(&status) {
            return Err(AppError::Other("未知的结束状态".into()));
        }
        let session = self.get(id)?;
        if session.status != "running" {
            return Err(AppError::Other("该会话已经结束".into()));
        }
        let ended_at = now_millis();
        let actual = ((ended_at - session.started_at) / 1000).max(0);
        self.conn.execute(
            "UPDATE focus_sessions
             SET status = ?2, ended_at = ?3, actual_duration_s = ?4
             WHERE id = ?1",
            rusqlite::params![id, status, ended_at, actual],
        )?;
        tracing::info!(session_id = id, status, actual_s = actual, "focus finished");
        self.get(id)
    }

    /// Tally one mid-session interruption; the session keeps running.
    pub fn add_interruption(&self, id: i64) -> AppResult<FocusSession> {
        let session = self.get(id)?;
        if session.status != "running" {
            return Err(AppError::Other("会话已结束，无法记录打断".into()));
        }
        self.conn.execute(
            "UPDATE focus_sessions SET interruptions = interruptions + 1 WHERE id = ?1",
            [id],
        )?;
        self.get(id)
    }

    /// Free-text note; blank clears it. Allowed on finished sessions too.
    pub fn set_note(&self, id: i64, note: Option<&str>) -> AppResult<()> {
        self.get(id)?;
        let note = note.map(str::trim).filter(|n| !n.is_empty());
        self.conn.execute(
            "UPDATE focus_sessions SET note = ?2 WHERE id = ?1",
            rusqlite::params![id, note],
        )?;
        Ok(())
    }

    /// All sessions that started on the given local day (YYYY-MM-DD). The
    /// day string is computed by the frontend so this crate never needs
    /// local-time date math.
    pub fn sessions_of_day(&self, day: &str) -> AppResult<Vec<FocusSession>> {
        let bytes = day.as_bytes();
        if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
            return Err(AppError::Other("日期格式应为 YYYY-MM-DD".into()));
        }
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {COLS} FROM focus_sessions
             WHERE date(started_at / 1000, 'unixepoch', 'localtime') = ?1
             ORDER BY started_at DESC"
        ))?;
        let rows = stmt.query_map([day], row_to_session)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Per-day totals over the last `days` local days. Running sessions are
    /// excluded — their actual duration is only known once finished.
    pub fn summary_days(&self, days: i64) -> AppResult<Vec<FocusDay>> {
        let days = days.clamp(1, 366);
        let mut stmt = self.conn.prepare(
            "SELECT date(started_at / 1000, 'unixepoch', 'localtime') AS day,
                    SUM(actual_duration_s), COUNT(*), SUM(interruptions)
             FROM focus_sessions
             WHERE status != 'running'
               AND day >= date('now', 'localtime', printf('-%d day', ?1))
             GROUP BY day
             ORDER BY day DESC",
        )?;
        let rows = stmt.query_map([days], |row| {
            Ok(FocusDay {
                day: row.get(0)?,
                total_s: row.get::<_, Option<i64>>(1)?.unwrap_or(0),
                sessions: row.get(2)?,
                interruptions: row.get::<_, Option<i64>>(3)?.unwrap_or(0),
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::scenes_repo::ScenesRepo;
    use crate::storage::Database;

    const DAY_MS: i64 = 24 * 60 * 60 * 1000;

    fn repo(db: &mut Database) -> FocusRepo<'_> {
        FocusRepo::new(db.conn())
    }

    #[test]
    fn start_finish_roundtrip() {
        let mut db = Database::open_in_memory().unwrap();
        let repo = repo(&mut db);
        assert!(repo.running().unwrap().is_none());

        let s = repo.start(KIND_POMODORO, 25 * 60, None, None).unwrap();
        assert_eq!(s.status, "running");
        assert_eq!(s.planned_duration_s, 1500);
        assert_eq!(s.actual_duration_s, 0);
        assert_eq!(s.interruptions, 0);
        assert!(repo.running().unwrap().is_some());

        let done = repo.finish(s.id, "completed").unwrap();
        assert_eq!(done.status, "completed");
        assert!(done.ended_at.unwrap() >= done.started_at);
        assert!(done.actual_duration_s < 5);
        assert!(repo.running().unwrap().is_none());
        // Finishing twice, or unknown statuses, are refused.
        assert!(repo.finish(s.id, "completed").is_err());
        assert!(repo.finish(s.id, "interrupted").is_err());
    }

    #[test]
    fn double_start_refused_and_validation() {
        let mut db = Database::open_in_memory().unwrap();
        let scene = ScenesRepo::new(db.conn()).create("S", None).unwrap();
        let repo = repo(&mut db);
        let s = repo.start(KIND_CUSTOM, 60, None, None).unwrap();
        assert!(repo.start(KIND_POMODORO, 60, None, None).is_err());
        // Bad kind, non-positive planned time, dangling ids.
        assert!(repo.start("nonsense", 60, None, None).is_err());
        assert!(repo.start(KIND_POMODORO, 0, None, None).is_err());
        assert!(repo.start(KIND_POMODORO, 60, Some(999), None).is_err());
        assert!(repo.start(KIND_POMODORO, 60, None, Some(999)).is_err());
        repo.finish(s.id, "abandoned").unwrap();
        let ok = repo.start(KIND_POMODORO, 60, None, Some(scene.id)).unwrap();
        assert_eq!(ok.scene_id, Some(scene.id));
    }

    #[test]
    fn count_up_stores_zero_planned() {
        let mut db = Database::open_in_memory().unwrap();
        let repo = repo(&mut db);
        let s = repo.start(KIND_COUNT_UP, 0, None, None).unwrap();
        assert_eq!(s.planned_duration_s, 0);
        repo.finish(s.id, "completed").unwrap();
    }

    #[test]
    fn interruptions_tally_only_while_running() {
        let mut db = Database::open_in_memory().unwrap();
        let repo = repo(&mut db);
        let s = repo.start(KIND_POMODORO, 60, None, None).unwrap();
        let s = repo.add_interruption(s.id).unwrap();
        let s = repo.add_interruption(s.id).unwrap();
        assert_eq!(s.interruptions, 2);
        repo.finish(s.id, "completed").unwrap();
        assert!(repo.add_interruption(s.id).is_err());
    }

    #[test]
    fn note_trims_and_clears() {
        let mut db = Database::open_in_memory().unwrap();
        let repo = repo(&mut db);
        let s = repo.start(KIND_POMODORO, 60, None, None).unwrap();
        repo.set_note(s.id, Some("  写论文第二章  ")).unwrap();
        assert_eq!(repo.get(s.id).unwrap().note.as_deref(), Some("写论文第二章"));
        repo.set_note(s.id, Some("   ")).unwrap();
        assert_eq!(repo.get(s.id).unwrap().note, None);
        assert!(repo.set_note(999, Some("x")).is_err());
    }

    #[test]
    fn day_listing_and_summary_buckets() {
        let mut db = Database::open_in_memory().unwrap();

        // A session that finished today.
        let today = {
            let repo = FocusRepo::new(db.conn());
            let s = repo.start(KIND_POMODORO, 60, None, None).unwrap();
            repo.finish(s.id, "completed").unwrap()
        };

        // A finished session faked to two days ago with a full hour.
        let two_days_ago = now_millis() - 2 * DAY_MS;
        let old = {
            let repo = FocusRepo::new(db.conn());
            let old = repo.start(KIND_CUSTOM, 60, None, None).unwrap();
            repo.finish(old.id, "completed").unwrap();
            old
        };
        db.conn()
            .execute(
                "UPDATE focus_sessions
                 SET started_at = ?2, ended_at = ?3, actual_duration_s = 3600
                 WHERE id = ?1",
                rusqlite::params![old.id, two_days_ago, two_days_ago + 3_600_000],
            )
            .unwrap();

        {
            let repo = FocusRepo::new(db.conn());
            // The running session must not appear in the summary.
            let running = repo.start(KIND_COUNT_UP, 0, None, None).unwrap();

            let summary = repo.summary_days(7).unwrap();
            let total_s: i64 = summary.iter().map(|d| d.total_s).sum();
            let sessions: i64 = summary.iter().map(|d| d.sessions).sum();
            // today(< 5s) + 3600s fake, running excluded.
            assert!((3600..3660).contains(&total_s), "total_s = {total_s}");
            assert_eq!(sessions, 2);
            assert!(!summary.is_empty() && summary.len() <= 2);

            // Day listing agrees with the summary buckets: the listing
            // includes running sessions, the summary only finished ones.
            for day in &summary {
                let listed = repo.sessions_of_day(&day.day).unwrap();
                let finished = listed
                    .iter()
                    .filter(|s| s.status != "running")
                    .count() as i64;
                assert_eq!(finished, day.sessions);
            }
            let today_day = summary
                .iter()
                .find(|d| d.total_s < 3600)
                .map(|d| d.day.clone())
                .unwrap();
            assert!(repo
                .sessions_of_day(&today_day)
                .unwrap()
                .iter()
                .any(|s| s.id == today.id));

            repo.finish(running.id, "abandoned").unwrap();
            // Bad day format is rejected before hitting SQL.
            assert!(repo.sessions_of_day("2026/09/05").is_err());
        }
    }
}
