//! Scenes: named arrangements of collections (tables from migration 0003).
//! V1 scope: which collections are visible in a scene. The geometry columns
//! (pos/size/collapsed) exist in the schema but stay unused for now; files
//! are never touched.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::app::error::{AppError, AppResult};
use crate::app::logging::now_millis;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Scene {
    pub id: i64,
    pub name: String,
    pub color: Option<String>,
    pub sort_order: i64,
}

/// Visibility of one collection inside a scene.
#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct SceneLayout {
    pub collection_id: i64,
    pub visible: bool,
}

pub struct ScenesRepo<'a> {
    conn: &'a Connection,
}

impl<'a> ScenesRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn list(&self) -> AppResult<Vec<Scene>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, color, sort_order FROM scenes ORDER BY sort_order, id")?;
        let rows = stmt.query_map([], |row| {
            Ok(Scene {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                sort_order: row.get(3)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn create(&self, name: &str, color: Option<&str>) -> AppResult<Scene> {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::Other("场景名称不能为空".into()));
        }
        let dup: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM scenes WHERE name = ?1",
            [name],
            |row| row.get(0),
        )?;
        if dup > 0 {
            return Err(AppError::Other("同名场景已存在".into()));
        }
        let sort_order: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM scenes",
            [],
            |row| row.get(0),
        )?;
        let now = now_millis();
        self.conn.execute(
            "INSERT INTO scenes (name, color, sort_order, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)",
            rusqlite::params![name, color, sort_order, now],
        )?;
        Ok(Scene {
            id: self.conn.last_insert_rowid(),
            name: name.to_string(),
            color: color.map(String::from),
            sort_order,
        })
    }

    pub fn rename(&self, id: i64, name: &str) -> AppResult<()> {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::Other("场景名称不能为空".into()));
        }
        let changed = self.conn.execute(
            "UPDATE scenes SET name = ?2, updated_at = ?3 WHERE id = ?1",
            rusqlite::params![id, name, now_millis()],
        )?;
        if changed == 0 {
            return Err(AppError::Other("场景不存在".into()));
        }
        Ok(())
    }

    pub fn delete(&self, id: i64) -> AppResult<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM scene_layouts WHERE scene_id = ?1", [id])?;
        let changed = tx.execute("DELETE FROM scenes WHERE id = ?1", [id])?;
        if changed == 0 {
            return Err(AppError::Other("场景不存在".into()));
        }
        tx.commit()?;
        Ok(())
    }

    /// Upsert one collection's visibility inside a scene. Both ids must exist.
    pub fn set_visible(&self, scene_id: i64, collection_id: i64, visible: bool) -> AppResult<()> {
        let scene_ok: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM scenes WHERE id = ?1",
            [scene_id],
            |row| row.get(0),
        )?;
        if scene_ok == 0 {
            return Err(AppError::Other("场景不存在".into()));
        }
        let col_ok: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM collections WHERE id = ?1",
            [collection_id],
            |row| row.get(0),
        )?;
        if col_ok == 0 {
            return Err(AppError::Other("集合不存在".into()));
        }
        self.conn.execute(
            "INSERT INTO scene_layouts (scene_id, collection_id, visible)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(scene_id, collection_id) DO UPDATE SET visible = excluded.visible",
            rusqlite::params![scene_id, collection_id, visible as i64],
        )?;
        Ok(())
    }

    /// Visibility rows of a scene, joined against live collections so rows
    /// orphaned by a deleted collection are dropped regardless of the
    /// foreign_keys pragma. Collections without a row default to visible.
    pub fn visibility(&self, scene_id: i64) -> AppResult<Vec<SceneLayout>> {
        let mut stmt = self.conn.prepare(
            "SELECT sl.collection_id, sl.visible
             FROM scene_layouts sl
             JOIN collections c ON c.id = sl.collection_id
             WHERE sl.scene_id = ?1",
        )?;
        let rows = stmt.query_map([scene_id], |row| {
            Ok(SceneLayout {
                collection_id: row.get(0)?,
                visible: row.get::<_, i64>(1)? != 0,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::collections_repo::CollectionsRepo;
    use crate::storage::Database;

    fn repo(db: &mut Database) -> ScenesRepo<'_> {
        ScenesRepo::new(db.conn())
    }

    #[test]
    fn create_list_rename_delete_roundtrip() {
        let mut db = Database::open_in_memory().unwrap();
        let repo = repo(&mut db);
        let a = repo.create("学习", None).unwrap();
        let b = repo.create("科研", Some("#4f6ef7")).unwrap();
        assert!(a.sort_order < b.sort_order);
        assert_eq!(repo.list().unwrap().len(), 2);
        repo.rename(b.id, "研究").unwrap();
        let listed = repo.list().unwrap();
        assert_eq!(listed[1].name, "研究");
        assert_eq!(listed[1].color.as_deref(), Some("#4f6ef7"));
        repo.delete(a.id).unwrap();
        assert_eq!(repo.list().unwrap().len(), 1);
        assert!(repo.delete(a.id).is_err());
    }

    #[test]
    fn duplicate_and_blank_names_are_rejected() {
        let mut db = Database::open_in_memory().unwrap();
        let repo = repo(&mut db);
        repo.create("A", None).unwrap();
        assert!(repo.create("A", None).is_err());
        assert!(repo.create("  ", None).is_err());
        let scene = repo.list().unwrap().remove(0);
        assert!(repo.rename(scene.id, " ").is_err());
    }

    #[test]
    fn visibility_upserts_and_drops_orphans() {
        let mut db = Database::open_in_memory().unwrap();
        let scene = ScenesRepo::new(db.conn()).create("S", None).unwrap();
        let (c1, c2) = {
            let cols = CollectionsRepo::new(db.conn());
            (
                cols.create("C1", "#4f8cff").unwrap(),
                cols.create("C2", "#8b5cf6").unwrap(),
            )
        };
        {
            let repo = ScenesRepo::new(db.conn());
            repo.set_visible(scene.id, c1.id, false).unwrap();
            // Upsert: flipping the same pair updates instead of duplicating.
            repo.set_visible(scene.id, c2.id, false).unwrap();
            repo.set_visible(scene.id, c2.id, true).unwrap();
            let vis = repo.visibility(scene.id).unwrap();
            assert_eq!(vis.len(), 2);
            let get = |id: i64| {
                vis.iter().find(|v| v.collection_id == id).unwrap().visible
            };
            assert!(!get(c1.id));
            assert!(get(c2.id));
            // Unknown ids are rejected.
            assert!(repo.set_visible(scene.id, 999, true).is_err());
            assert!(repo.set_visible(999, c1.id, true).is_err());
        }
        // A collection deleted afterwards must not leak into the map.
        CollectionsRepo::new(db.conn()).delete(c2.id).unwrap();
        assert_eq!(
            ScenesRepo::new(db.conn()).visibility(scene.id).unwrap().len(),
            1
        );
    }
}
