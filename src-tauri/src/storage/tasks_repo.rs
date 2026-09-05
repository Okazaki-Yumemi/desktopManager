//! Lightweight task list (migration 0004). Pure local metadata — nothing
//! here touches the filesystem. Status flows todo → doing → done; done
//! stamps completed_at and un-done clears it.

use rusqlite::params;
use rusqlite::OptionalExtension;
use serde::Serialize;

use crate::app::error::{AppError, AppResult};
use crate::app::logging::now_millis;

pub const STATUSES: [&str; 3] = ["todo", "doing", "done"];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub notes: Option<String>,
    pub status: String,
    pub priority: i64,
    pub due_at: Option<i64>,
    pub estimated_minutes: Option<i64>,
    pub tags: Vec<String>,
    pub created_at: i64,
    pub completed_at: Option<i64>,
    pub updated_at: i64,
}

pub struct TasksRepo<'a> {
    conn: &'a rusqlite::Connection,
}

impl<'a> TasksRepo<'a> {
    pub fn new(conn: &'a rusqlite::Connection) -> Self {
        Self { conn }
    }

    fn map(row: &rusqlite::Row<'_>) -> rusqlite::Result<Task> {
        let tags_json: Option<String> = row.get(7)?;
        let tags = tags_json
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();
        Ok(Task {
            id: row.get(0)?,
            title: row.get(1)?,
            notes: row.get(2)?,
            status: row.get(3)?,
            priority: row.get(4)?,
            due_at: row.get(5)?,
            estimated_minutes: row.get(6)?,
            tags,
            created_at: row.get(8)?,
            completed_at: row.get(9)?,
            updated_at: row.get(10)?,
        })
    }

    const COLS: &'static str = "id, title, notes, status, priority, due_at, estimated_minutes,
        tags, created_at, completed_at, updated_at";

    /// All tasks: doing first, then todo by (due, priority desc, created),
    /// done last. This is the default "work on next" order.
    pub fn list(&self) -> AppResult<Vec<Task>> {
        let mut stmt = self.conn.prepare_cached(&format!(
            "SELECT {} FROM tasks
             ORDER BY CASE status WHEN 'doing' THEN 0 WHEN 'todo' THEN 1 ELSE 2 END,
                       due_at IS NULL, due_at, priority DESC, created_at",
            Self::COLS
        ))?;
        let rows = stmt.query_map([], Self::map)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn get(&self, id: i64) -> AppResult<Option<Task>> {
        self.conn
            .query_row(
                &format!("SELECT {} FROM tasks WHERE id = ?1", Self::COLS),
                [id],
                Self::map,
            )
            .optional()
            .map_err(Into::into)
    }

    /// Create a task. `tags` must be plain strings; everything else is
    /// trimmed/validated. Returns the stored row.
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        &self,
        title: &str,
        notes: Option<&str>,
        priority: i64,
        due_at: Option<i64>,
        estimated_minutes: Option<i64>,
        tags: &[String],
    ) -> AppResult<Task> {
        let title = title.trim();
        if title.is_empty() {
            return Err(AppError::Other("任务标题不能为空".into()));
        }
        if !(0..=3).contains(&priority) {
            return Err(AppError::Other("优先级必须是 0–3".into()));
        }
        if estimated_minutes.is_some_and(|m| m <= 0) {
            return Err(AppError::Other("预计分钟数必须为正".into()));
        }
        let mut clean_tags: Vec<String> = tags
            .iter()
            .map(|t| t.trim().to_owned())
            .filter(|t| !t.is_empty())
            .collect();
        clean_tags.sort();
        clean_tags.dedup();
        let tags_json = serde_json::to_string(&clean_tags)
            .map_err(|_| AppError::Other("标签序列化失败".into()))?;
        let now = now_millis();
        self.conn.execute(
            "INSERT INTO tasks (title, notes, status, priority, due_at, estimated_minutes,
                                tags, created_at, completed_at, updated_at)
             VALUES (?1, ?2, 'todo', ?3, ?4, ?5, ?6, ?7, NULL, ?7)",
            params![
                title,
                notes.map(str::trim).filter(|s| !s.is_empty()),
                priority,
                due_at,
                estimated_minutes,
                tags_json,
                now
            ],
        )?;
        Ok(self
            .get(self.conn.last_insert_rowid())?
            .expect("row just inserted"))
    }

    /// Edit mutable fields; status is intentionally out of this path —
    /// use set_status so completed_at stays consistent.
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &self,
        id: i64,
        title: &str,
        notes: Option<&str>,
        priority: i64,
        due_at: Option<i64>,
        estimated_minutes: Option<i64>,
        tags: &[String],
    ) -> AppResult<()> {
        if self.get(id)?.is_none() {
            return Err(AppError::Other("任务不存在".into()));
        }
        let title = title.trim();
        if title.is_empty() {
            return Err(AppError::Other("任务标题不能为空".into()));
        }
        if !(0..=3).contains(&priority) {
            return Err(AppError::Other("优先级必须是 0–3".into()));
        }
        if estimated_minutes.is_some_and(|m| m <= 0) {
            return Err(AppError::Other("预计分钟数必须为正".into()));
        }
        let mut clean_tags: Vec<String> = tags
            .iter()
            .map(|t| t.trim().to_owned())
            .filter(|t| !t.is_empty())
            .collect();
        clean_tags.sort();
        clean_tags.dedup();
        let tags_json = serde_json::to_string(&clean_tags)
            .map_err(|_| AppError::Other("标签序列化失败".into()))?;
        let changed = self.conn.execute(
            "UPDATE tasks SET title = ?2, notes = ?3, priority = ?4, due_at = ?5,
                    estimated_minutes = ?6, tags = ?7, updated_at = ?8
             WHERE id = ?1",
            params![
                id,
                title,
                notes.map(str::trim).filter(|s| !s.is_empty()),
                priority,
                due_at,
                estimated_minutes,
                tags_json,
                now_millis()
            ],
        )?;
        if changed == 0 {
            return Err(AppError::Other("任务不存在".into()));
        }
        Ok(())
    }

    /// Move a task between todo/doing/done; done stamps completed_at,
    /// leaving done clears it.
    pub fn set_status(&self, id: i64, status: &str) -> AppResult<Task> {
        if !STATUSES.contains(&status) {
            return Err(AppError::Other(format!("未知任务状态：{status}")));
        }
        let now = now_millis();
        let changed = self.conn.execute(
            "UPDATE tasks SET status = ?2,
                    completed_at = CASE WHEN ?2 = 'done' THEN ?3 ELSE NULL END,
                    updated_at = ?3
             WHERE id = ?1",
            params![id, status, now],
        )?;
        if changed == 0 {
            return Err(AppError::Other("任务不存在".into()));
        }
        Ok(self.get(id)?.expect("row just updated"))
    }

    pub fn delete(&self, id: i64) -> AppResult<bool> {
        let changed = self.conn.execute("DELETE FROM tasks WHERE id = ?1", [id])?;
        Ok(changed > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;

    fn repo(db: &mut Database) -> TasksRepo<'_> {
        TasksRepo::new(db.conn())
    }

    #[test]
    fn create_update_and_ordering() {
        let mut db = Database::open_in_memory().unwrap();
        let a = repo(&mut db)
            .create("写周报", Some("  覆盖本周  "), 1, Some(1000), Some(30), &["工作".into()])
            .unwrap();
        assert_eq!(a.status, "todo");
        assert_eq!(a.notes.as_deref(), Some("覆盖本周")); // trimmed
        assert_eq!(a.tags, vec!["工作"]);

        let b = repo(&mut db).create("摸鱼", None, 2, None, None, &[]).unwrap();
        repo(&mut db).set_status(b.id, "doing").unwrap();

        let listed = repo(&mut db).list().unwrap();
        assert_eq!(listed[0].id, b.id); // doing first
        assert_eq!(listed[1].id, a.id);

        repo(&mut db)
            .update(a.id, "写月报", None, 2, None, None, &["工作".into(), " 写作 ".into()])
            .unwrap();
        let a2 = repo(&mut db).get(a.id).unwrap().unwrap();
        assert_eq!(a2.title, "写月报");
        assert_eq!(a2.tags.len(), 2); // deduped + sorted
        assert!(a2.notes.is_none());

        assert!(repo(&mut db).create("   ", None, 0, None, None, &[]).is_err());
        assert!(repo(&mut db).create("x", None, 9, None, None, &[]).is_err());
        assert!(repo(&mut db).create("x", None, 0, None, Some(0), &[]).is_err());
    }

    #[test]
    fn status_transitions_stamp_completed_at() {
        let mut db = Database::open_in_memory().unwrap();
        let t = repo(&mut db).create("任务", None, 0, None, None, &[]).unwrap();
        assert!(t.completed_at.is_none());

        let done = repo(&mut db).set_status(t.id, "done").unwrap();
        assert!(done.completed_at.is_some());

        let back = repo(&mut db).set_status(t.id, "todo").unwrap();
        assert!(back.completed_at.is_none());

        assert!(repo(&mut db).set_status(t.id, "paused").is_err());
        assert!(repo(&mut db).set_status(9999, "done").is_err());
    }

    #[test]
    fn delete_removes_row() {
        let mut db = Database::open_in_memory().unwrap();
        let t = repo(&mut db).create("临时", None, 0, None, None, &[]).unwrap();
        assert!(repo(&mut db).delete(t.id).unwrap());
        assert!(!repo(&mut db).delete(t.id).unwrap());
        assert!(repo(&mut db).get(t.id).unwrap().is_none());
    }
}
