//! Virtual collections: named groups of desktop items.
//!
//! Assignment is pure metadata (`collection_items` rows) — the real files
//! on the desktop are never touched. Items are referenced by path, which is
//! the natural key of the desktop index.

use rusqlite::params;
use serde::Serialize;

use crate::app::error::{AppError, AppResult};
use crate::app::logging::now_millis;
use crate::storage::desktop_repo::{map_item, DesktopItem};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub item_count: i64,
}

pub struct CollectionsRepo<'a> {
    conn: &'a rusqlite::Connection,
}

impl<'a> CollectionsRepo<'a> {
    pub fn new(conn: &'a rusqlite::Connection) -> Self {
        Self { conn }
    }

    /// All collections with item counts, in stable creation order.
    pub fn list(&self) -> AppResult<Vec<Collection>> {
        let mut stmt = self.conn.prepare(
            "SELECT c.id, c.name, c.color, COUNT(ci.id)
             FROM collections c
             LEFT JOIN collection_items ci ON ci.collection_id = c.id
             GROUP BY c.id
             ORDER BY c.sort_order, c.id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Collection {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                item_count: row.get(3)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn create(&self, name: &str, color: &str) -> AppResult<Collection> {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::Other("集合名称不能为空".into()));
        }
        let exists: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM collections WHERE name = ?1",
            [name],
            |row| row.get(0),
        )?;
        if exists > 0 {
            return Err(AppError::Other(format!("同名集合已存在：{name}")));
        }
        let sort_order: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM collections",
            [],
            |row| row.get(0),
        )?;
        self.conn.execute(
            "INSERT INTO collections (name, color, sort_order, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)",
            params![name, color, sort_order, now_millis()],
        )?;
        Ok(Collection {
            id: self.conn.last_insert_rowid(),
            name: name.to_owned(),
            color: color.to_owned(),
            item_count: 0,
        })
    }

    pub fn rename(&self, id: i64, name: &str) -> AppResult<()> {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::Other("集合名称不能为空".into()));
        }
        let changed = self.conn.execute(
            "UPDATE collections SET name = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, name, now_millis()],
        )?;
        if changed == 0 {
            return Err(AppError::Other("集合不存在".into()));
        }
        Ok(())
    }

    /// Delete a collection and its assignments. Children are removed
    /// explicitly so this holds regardless of the foreign_keys pragma.
    pub fn delete(&self, id: i64) -> AppResult<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM collection_items WHERE collection_id = ?1",
            [id],
        )?;
        let changed = tx.execute("DELETE FROM collections WHERE id = ?1", [id])?;
        if changed == 0 {
            return Err(AppError::Other("集合不存在".into()));
        }
        tx.commit()?;
        Ok(())
    }

    /// Assign an item path to a collection. The path must exist in the
    /// desktop index — same allow-list policy as opening and icons.
    /// Returns whether a new assignment row was created.
    pub fn assign(&self, collection_id: i64, item_path: &str) -> AppResult<bool> {
        let known: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM desktop_items WHERE path = ?1",
            [item_path],
            |row| row.get(0),
        )?;
        if known == 0 {
            return Err(AppError::Other("该路径不在桌面索引中".into()));
        }
        let changed = self.conn.execute(
            "INSERT OR IGNORE INTO collection_items (collection_id, item_path, sort_order, added_at)
             VALUES (?1, ?2, (SELECT COALESCE(MAX(sort_order), -1) + 1
                              FROM collection_items WHERE collection_id = ?1), ?3)",
            params![collection_id, item_path, now_millis()],
        )?;
        Ok(changed > 0)
    }

    /// Remove an assignment. Returns whether a row was removed.
    pub fn unassign(&self, collection_id: i64, item_path: &str) -> AppResult<bool> {
        let changed = self.conn.execute(
            "DELETE FROM collection_items WHERE collection_id = ?1 AND item_path = ?2",
            params![collection_id, item_path],
        )?;
        Ok(changed > 0)
    }

    /// Visible items of a collection, in assignment order. Items that have
    /// disappeared from the desktop are filtered out (they stay indexed and
    /// re-join automatically when they reappear).
    pub fn items(&self, collection_id: i64) -> AppResult<Vec<DesktopItem>> {
        let mut stmt = self.conn.prepare(
            "SELECT d.id, d.path, d.source, d.display_name, d.kind, d.ext,
                    d.size_bytes, d.modified_at, d.missing
             FROM collection_items ci
             JOIN desktop_items d ON d.path = ci.item_path
             WHERE ci.collection_id = ?1 AND d.missing = 0
             ORDER BY ci.sort_order, ci.added_at",
        )?;
        let rows = stmt.query_map([collection_id], map_item)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;

    fn repo(db: &mut Database) -> CollectionsRepo<'_> {
        CollectionsRepo::new(db.conn())
    }

    fn seed_item(db: &mut Database, path: &str, name: &str) {
        db.conn()
            .execute(
                "INSERT INTO desktop_items (path, source, display_name, kind, ext, size_bytes,
                                            modified_at, first_seen_at, last_seen_at)
                 VALUES (?1, 'user_desktop', ?2, 'file', NULL, NULL, NULL, 1, 1)",
                params![path, name],
            )
            .unwrap();
    }

    #[test]
    fn create_list_and_assign_roundtrip() {
        let mut db = Database::open_in_memory().unwrap();
        seed_item(&mut db, "C:\\d\\a.pdf", "a.pdf");
        seed_item(&mut db, "C:\\d\\b.txt", "b.txt");

        let c1 = repo(&mut db).create("工作", "#ff0000").unwrap();
        let c2 = repo(&mut db).create("随手记", "#00ff00").unwrap();
        assert_eq!(c1.item_count, 0);
        assert_eq!(c2.name, "随手记");

        assert!(repo(&mut db).assign(c1.id, "C:\\d\\a.pdf").unwrap());
        assert!(!repo(&mut db).assign(c1.id, "C:\\d\\a.pdf").unwrap()); // idempotent
        assert!(repo(&mut db).assign(c1.id, "C:\\d\\b.txt").unwrap());

        let listed = repo(&mut db).list().unwrap();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].name, "工作");
        assert_eq!(listed[0].item_count, 2);
        assert_eq!(listed[1].item_count, 0);

        let items = repo(&mut db).items(c1.id).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].path, "C:\\d\\a.pdf"); // assignment order
        assert_eq!(items[1].path, "C:\\d\\b.txt");
    }

    #[test]
    fn assign_rejects_paths_outside_the_index() {
        let mut db = Database::open_in_memory().unwrap();
        let c = repo(&mut db).create("工作", "#ff0000").unwrap();
        let err = repo(&mut db)
            .assign(c.id, "C:\\Windows\\notepad.exe")
            .unwrap_err();
        assert!(err.to_string().contains("桌面索引"));
    }

    #[test]
    fn duplicate_names_are_rejected_but_rename_works() {
        let mut db = Database::open_in_memory().unwrap();
        repo(&mut db).create("工作", "#ff0000").unwrap();
        assert!(repo(&mut db).create("工作", "#ff0000").is_err());
        assert!(repo(&mut db).create("  工作  ", "#ff0000").is_err()); // trim-insensitive

        let c = repo(&mut db).create("临时", "#ff0000").unwrap();
        repo(&mut db).rename(c.id, "草稿").unwrap();
        let listed = repo(&mut db).list().unwrap();
        assert_eq!(listed[1].name, "草稿");
        assert!(repo(&mut db).rename(9999, "x").is_err());
    }

    #[test]
    fn delete_removes_assignments_and_missing_items_are_hidden() {
        let mut db = Database::open_in_memory().unwrap();
        seed_item(&mut db, "C:\\d\\a.pdf", "a.pdf");
        seed_item(&mut db, "C:\\d\\gone.txt", "gone.txt");
        db.conn()
            .execute(
                "UPDATE desktop_items SET missing = 1 WHERE path = ?1",
                ["C:\\d\\gone.txt"],
            )
            .unwrap();

        let c = repo(&mut db).create("工作", "#ff0000").unwrap();
        repo(&mut db).assign(c.id, "C:\\d\\a.pdf").unwrap();
        repo(&mut db).assign(c.id, "C:\\d\\gone.txt").unwrap();
        assert_eq!(repo(&mut db).items(c.id).unwrap().len(), 1); // missing filtered

        repo(&mut db).delete(c.id).unwrap();
        assert_eq!(repo(&mut db).list().unwrap().len(), 0);
        let orphans: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM collection_items", [], |r| r.get(0))
            .unwrap();
        assert_eq!(orphans, 0);
        assert!(repo(&mut db).delete(9999).is_err());
    }
}
