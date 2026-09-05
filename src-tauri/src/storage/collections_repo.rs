//! Virtual collections: named groups of desktop items.
//!
//! Assignment is pure metadata (`collection_items` rows) — the real files
//! on the desktop are never touched. Items are referenced by path, which is
//! the natural key of the desktop index.

use rusqlite::params;
use rusqlite::OptionalExtension;
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
    pub parent_id: Option<i64>,
    pub item_count: i64,
}

/// Deepest allowed nesting: root + 4 levels of children.
const MAX_DEPTH: usize = 5;

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
            "SELECT c.id, c.name, c.color, c.parent_id, COUNT(ci.id)
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
                parent_id: row.get(3)?,
                item_count: row.get(4)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn create(&self, name: &str, color: &str, parent_id: Option<i64>) -> AppResult<Collection> {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::Other("集合名称不能为空".into()));
        }
        if parent_id.is_some() {
            self.check_depth(parent_id)?;
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
            "INSERT INTO collections (name, color, parent_id, sort_order, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            params![name, color, parent_id, sort_order, now_millis()],
        )?;
        Ok(Collection {
            id: self.conn.last_insert_rowid(),
            name: name.to_owned(),
            color: color.to_owned(),
            parent_id,
            item_count: 0,
        })
    }

    /// Validate the parent chain of a would-be child: the parent must exist
    /// and the resulting depth must stay within `MAX_DEPTH`. The walk also
    /// terminates on (corrupt) cycles via the depth cap.
    fn check_depth(&self, parent_id: Option<i64>) -> AppResult<()> {
        let mut cur = parent_id;
        let mut hops = 0_usize;
        while let Some(pid) = cur {
            let row: Option<Option<i64>> = self
                .conn
                .query_row(
                    "SELECT parent_id FROM collections WHERE id = ?1",
                    [pid],
                    |row| row.get(0),
                )
                .optional()?;
            let Some(next) = row else {
                return Err(AppError::Other("父集合不存在".into()));
            };
            hops += 1;
            if hops >= MAX_DEPTH {
                return Err(AppError::Other(format!(
                    "集合层级过深（最多 {MAX_DEPTH} 层）"
                )));
            }
            cur = next;
        }
        Ok(())
    }

    pub fn rename(&self, id: i64, name: &str) -> AppResult<()> {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::Other("集合名称不能为空".into()));
        }
        let dup: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM collections WHERE name = ?1 AND id != ?2",
            params![name, id],
            |row| row.get(0),
        )?;
        if dup > 0 {
            return Err(AppError::Other(format!("同名集合已存在：{name}")));
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

    /// Delete a collection, its whole subtree, and all their assignments.
    /// The recursive CTE keeps this correct regardless of the foreign_keys
    /// pragma.
    pub fn delete(&self, id: i64) -> AppResult<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM collection_items WHERE collection_id IN (
                 WITH RECURSIVE sub(id) AS (
                     SELECT id FROM collections WHERE id = ?1
                     UNION ALL
                     SELECT c.id FROM collections c JOIN sub ON c.parent_id = sub.id
                 )
                 SELECT id FROM sub
             )",
            [id],
        )?;
        let changed = tx.execute(
            "DELETE FROM collections WHERE id IN (
                 WITH RECURSIVE sub(id) AS (
                     SELECT id FROM collections WHERE id = ?1
                     UNION ALL
                     SELECT c.id FROM collections c JOIN sub ON c.parent_id = sub.id
                 )
                 SELECT id FROM sub
             )",
            [id],
        )?;
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

    /// Assign any path that exists on disk (shortcut, file or folder) and
    /// is not necessarily on the desktop. Snapshot metadata is stored on the
    /// assignment row; if the path is (or later becomes) desktop-indexed,
    /// live metadata wins at read time.
    /// Returns whether a new assignment row was created.
    pub fn assign_external(&self, collection_id: i64, item_path: &str) -> AppResult<bool> {
        let path = std::path::Path::new(item_path);
        if !path.is_absolute() {
            return Err(AppError::Other("路径必须是绝对路径".into()));
        }
        let meta = std::fs::metadata(path)
            .map_err(|_| AppError::Other(format!("路径不存在：{item_path}")))?;

        let is_dir = meta.is_dir();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase());
        let kind = match (&is_dir, ext.as_deref()) {
            (true, _) => "folder",
            (false, Some("lnk") | Some("url")) => "shortcut",
            (false, _) => "file",
        };
        let label = match (is_dir, ext.as_deref()) {
            (false, Some("lnk") | Some("url")) => {
                path.file_stem().and_then(|s| s.to_str()).map(str::to_owned)
            }
            _ => path.file_name().and_then(|s| s.to_str()).map(str::to_owned),
        }
        .ok_or_else(|| AppError::Other(format!("无法解析名称：{item_path}")))?;
        let size_bytes = (!is_dir).then_some(meta.len() as i64);
        let modified_at = meta
            .modified()
            .ok()
            .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64);

        let changed = self.conn.execute(
            "INSERT OR IGNORE INTO collection_items
                (collection_id, item_path, sort_order, added_at,
                 label, kind, ext, size_bytes, modified_at)
             VALUES (?1, ?2,
                     (SELECT COALESCE(MAX(sort_order), -1) + 1
                      FROM collection_items WHERE collection_id = ?1),
                     ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                collection_id,
                item_path,
                now_millis(),
                label,
                kind,
                ext,
                size_bytes,
                modified_at
            ],
        )?;
        Ok(changed > 0)
    }

    /// Assign a path whatever it is: desktop-indexed paths go through the
    /// live assignment, everything else through the external snapshot.
    pub fn assign_any(&self, collection_id: i64, item_path: &str) -> AppResult<bool> {
        let known: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM desktop_items WHERE path = ?1",
            [item_path],
            |row| row.get(0),
        )?;
        if known > 0 {
            self.assign(collection_id, item_path)
        } else {
            self.assign_external(collection_id, item_path)
        }
    }

    /// Remove an assignment. Returns whether a row was removed.
    pub fn unassign(&self, collection_id: i64, item_path: &str) -> AppResult<bool> {
        let changed = self.conn.execute(
            "DELETE FROM collection_items WHERE collection_id = ?1 AND item_path = ?2",
            params![collection_id, item_path],
        )?;
        Ok(changed > 0)
    }

    /// Whether any collection currently holds this path (external-item
    /// open/icon allow-list).
    pub fn holds_path(&self, item_path: &str) -> AppResult<bool> {
        let n: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM collection_items WHERE item_path = ?1",
            [item_path],
            |row| row.get(0),
        )?;
        Ok(n > 0)
    }

    /// Items of a collection, in assignment order: desktop-indexed paths use
    /// live metadata (hidden while missing from disk), everything else uses
    /// the snapshot stored at assignment time.
    pub fn items(&self, collection_id: i64) -> AppResult<Vec<DesktopItem>> {
        let mut stmt = self.conn.prepare(
            "SELECT COALESCE(d.id, -ci.id),
                    COALESCE(d.path, ci.item_path),
                    COALESCE(d.source, 'external'),
                    COALESCE(d.display_name, ci.label),
                    COALESCE(d.kind, ci.kind),
                    COALESCE(d.ext, ci.ext),
                    COALESCE(d.size_bytes, ci.size_bytes),
                    COALESCE(d.modified_at, ci.modified_at),
                    0
             FROM collection_items ci
             LEFT JOIN desktop_items d ON d.path = ci.item_path
             WHERE ci.collection_id = ?1 AND (d.id IS NULL OR d.missing = 0)
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

        let c1 = repo(&mut db).create("工作", "#ff0000", None).unwrap();
        let c2 = repo(&mut db).create("随手记", "#00ff00", None).unwrap();
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
        let c = repo(&mut db).create("工作", "#ff0000", None).unwrap();
        let err = repo(&mut db)
            .assign(c.id, "C:\\Windows\\notepad.exe")
            .unwrap_err();
        assert!(err.to_string().contains("桌面索引"));
    }

    #[test]
    fn duplicate_names_are_rejected_but_rename_works() {
        let mut db = Database::open_in_memory().unwrap();
        repo(&mut db).create("工作", "#ff0000", None).unwrap();
        assert!(repo(&mut db).create("工作", "#ff0000", None).is_err());
        assert!(repo(&mut db).create("  工作  ", "#ff0000", None).is_err()); // trim-insensitive

        let c = repo(&mut db).create("临时", "#ff0000", None).unwrap();
        repo(&mut db).rename(c.id, "草稿").unwrap();
        let listed = repo(&mut db).list().unwrap();
        assert_eq!(listed[1].name, "草稿");
        assert!(repo(&mut db).rename(9999, "x").is_err());
        // Rename must not collide with another collection's name.
        assert!(repo(&mut db).rename(c.id, "工作").is_err());
        repo(&mut db).rename(c.id, "  草稿  ").unwrap(); // trim, same name OK
    }

    #[test]
    fn sub_collections_nest_and_depth_is_capped() {
        let mut db = Database::open_in_memory().unwrap();
        let root = repo(&mut db).create("根", "#ff0000", None).unwrap();
        let child = repo(&mut db).create("子", "#00ff00", Some(root.id)).unwrap();
        let grand = repo(&mut db).create("孙", "#0000ff", Some(child.id)).unwrap();

        let listed = repo(&mut db).list().unwrap();
        assert_eq!(listed[1].parent_id, Some(root.id));
        assert_eq!(listed[2].parent_id, Some(child.id));

        // Missing parent and too-deep chains are rejected.
        assert!(repo(&mut db).create("x", "#ff0000", Some(9999)).is_err());
        let mut leaf = grand.id;
        for i in 3..MAX_DEPTH {
            let next = repo(&mut db)
                .create(&format!("l{i}"), "#ff0000", Some(leaf))
                .unwrap();
            leaf = next.id;
        }
        assert!(repo(&mut db)
            .create("太深了", "#ff0000", Some(leaf))
            .unwrap_err()
            .to_string()
            .contains("层级过深"));
    }

    #[test]
    fn deleting_a_parent_removes_the_whole_subtree() {
        let mut db = Database::open_in_memory().unwrap();
        seed_item(&mut db, "C:\\d\\a.pdf", "a.pdf");
        let root = repo(&mut db).create("根", "#ff0000", None).unwrap();
        let child = repo(&mut db).create("子", "#00ff00", Some(root.id)).unwrap();
        let grand = repo(&mut db).create("孙", "#0000ff", Some(child.id)).unwrap();
        repo(&mut db).assign(grand.id, "C:\\d\\a.pdf").unwrap();

        repo(&mut db).delete(root.id).unwrap();
        assert_eq!(repo(&mut db).list().unwrap().len(), 0);
        let orphans: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM collection_items", [], |r| r.get(0))
            .unwrap();
        assert_eq!(orphans, 0);
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

        let c = repo(&mut db).create("工作", "#ff0000", None).unwrap();
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

    #[test]
    fn external_assign_snapshots_metadata_and_missing_paths_reject() {
        let mut db = Database::open_in_memory().unwrap();
        let dir = std::env::temp_dir().join(format!("dm-coll-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let lnk = dir.join("我的工具.lnk");
        std::fs::write(&lnk, b"stub").unwrap();

        let c = repo(&mut db).create("工具", "#ff0000", None).unwrap();
        assert!(repo(&mut db)
            .assign_external(c.id, &lnk.to_string_lossy())
            .unwrap());

        let items = repo(&mut db).items(c.id).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].source, "external");
        assert_eq!(items[0].display_name, "我的工具"); // stem, D10
        assert_eq!(items[0].kind, "shortcut");
        assert_eq!(items[0].ext.as_deref(), Some("lnk"));
        assert_eq!(items[0].size_bytes, Some(4));

        // Folder snapshot + relative/missing rejection.
        let sub = dir.join("子目录");
        std::fs::create_dir_all(&sub).unwrap();
        assert!(repo(&mut db)
            .assign_external(c.id, &sub.to_string_lossy())
            .unwrap());
        let items = repo(&mut db).items(c.id).unwrap();
        assert_eq!(items[1].kind, "folder");
        assert_eq!(items[1].display_name, "子目录");
        assert_eq!(items[1].size_bytes, None);
        assert!(repo(&mut db)
            .assign_external(c.id, "C:\\definitely\\missing.lnk")
            .is_err());
        assert!(repo(&mut db)
            .assign_external(c.id, "relative/path.lnk")
            .is_err());

        // Mixed: indexed path assigned normally keeps live metadata precedence.
        seed_item(&mut db, "C:\\d\\a.pdf", "a.pdf");
        assert!(repo(&mut db).assign(c.id, "C:\\d\\a.pdf").unwrap());
        let items = repo(&mut db).items(c.id).unwrap();
        assert_eq!(items[2].source, "user_desktop");

        assert!(repo(&mut db).holds_path(&lnk.to_string_lossy()).unwrap());
        assert!(!repo(&mut db).holds_path("C:\\not\\in\\any.lnk").unwrap());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
