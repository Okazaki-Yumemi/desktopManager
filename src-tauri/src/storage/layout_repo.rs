//! Persistence for desktop icon-layout snapshots (table from migration 0002,
//! labels added in 0007). Payloads are opaque JSON here; their shape lives in
//! `desktop::shell_layout`.

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::app::error::{AppError, AppResult};
use crate::app::logging::now_millis;

/// Row projection for the layouts list.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutSummary {
    pub id: i64,
    pub name: String,
    pub created_at: i64,
    pub item_count: i64,
}

pub struct LayoutRepo<'a> {
    conn: &'a Connection,
}

impl<'a> LayoutRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Store a payload under a unique, non-empty name. Returns the row id.
    pub fn save(&self, name: &str, payload: &str) -> AppResult<i64> {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::Other("布局名称不能为空".into()));
        }
        let dup: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM layout_snapshots WHERE name = ?1",
            [name],
            |row| row.get(0),
        )?;
        if dup > 0 {
            return Err(AppError::Other("同名布局已存在".into()));
        }
        self.conn.execute(
            "INSERT INTO layout_snapshots (name, created_at, reason, payload)
             VALUES (?1, ?2, 'manual', ?3)",
            rusqlite::params![name, now_millis(), payload],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// All snapshots, newest first. Item counts come from parsing payloads;
    /// unreadable payloads count as 0 rather than hiding the row.
    pub fn list(&self) -> AppResult<Vec<LayoutSummary>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, created_at, payload
             FROM layout_snapshots ORDER BY created_at DESC, id DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let payload: String = row.get(3)?;
            let item_count = serde_json::from_str::<crate::desktop::shell_layout::LayoutPayload>(
                &payload,
            )
            .map(|p| p.items.len() as i64)
            .unwrap_or(0);
            Ok(LayoutSummary {
                id: row.get(0)?,
                name: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                created_at: row.get(2)?,
                item_count,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn payload_of(&self, id: i64) -> AppResult<Option<String>> {
        Ok(self
            .conn
            .query_row(
                "SELECT payload FROM layout_snapshots WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .optional()?)
    }

    /// Returns whether a row was removed.
    pub fn delete(&self, id: i64) -> AppResult<bool> {
        let changed = self
            .conn
            .execute("DELETE FROM layout_snapshots WHERE id = ?1", [id])?;
        Ok(changed > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;

    fn repo(db: &mut Database) -> LayoutRepo<'_> {
        LayoutRepo::new(db.conn())
    }

    fn payload(items: &[( &str, i32, i32)]) -> String {
        serde_json::to_string(&crate::desktop::shell_layout::LayoutPayload {
            items: items
                .iter()
                .map(|(n, x, y)| crate::desktop::shell_layout::LayoutItem {
                    name: (*n).into(),
                    x: *x,
                    y: *y,
                })
                .collect(),
        })
        .unwrap()
    }

    #[test]
    fn save_list_roundtrip() {
        let mut db = Database::open_in_memory().unwrap();
        let repo = repo(&mut db);
        let id = repo.save("工作布局", &payload(&[("a.txt", 10, 20), ("b.lnk", 30, 40)])).unwrap();
        let list = repo.list().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, id);
        assert_eq!(list[0].name, "工作布局");
        assert_eq!(list[0].item_count, 2);
        let got = repo.payload_of(id).unwrap().unwrap();
        assert!(got.contains("a.txt"));
    }

    #[test]
    fn duplicate_names_and_blank_names_are_rejected() {
        let mut db = Database::open_in_memory().unwrap();
        let repo = repo(&mut db);
        repo.save("A", &payload(&[])).unwrap();
        assert!(repo.save("A", &payload(&[])).is_err());
        assert!(repo.save("   ", &payload(&[])).is_err());
    }

    #[test]
    fn payload_of_missing_is_none_and_delete_reports() {
        let mut db = Database::open_in_memory().unwrap();
        let repo = repo(&mut db);
        let id = repo.save("B", &payload(&[("x", 1, 2)])).unwrap();
        assert!(repo.payload_of(id + 1).unwrap().is_none());
        assert!(repo.delete(id).unwrap());
        assert!(!repo.delete(id).unwrap());
        assert!(repo.list().unwrap().is_empty());
    }
}
