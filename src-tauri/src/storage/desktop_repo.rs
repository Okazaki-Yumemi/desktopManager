//! Repository for the `desktop_items` index. This is the only place that
//! knows the index's SQL.

use std::collections::HashMap;

use rusqlite::{params, Connection};
use serde::Serialize;

use crate::app::error::AppResult;
use crate::app::logging::now_millis;
use crate::desktop::scanner::ScannedItem;

/// One indexed desktop entry as exposed to the frontend (camelCase payload).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopItem {
    pub id: i64,
    pub path: String,
    pub source: String,
    pub display_name: String,
    pub kind: String,
    pub ext: Option<String>,
    pub size_bytes: Option<i64>,
    pub modified_at: Option<i64>,
    pub missing: bool,
}

/// What one sync pass changed. Zero values mean "nothing to tell the UI".
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncOutcome {
    pub added: usize,
    pub updated: usize,
    pub removed: usize,
}

impl SyncOutcome {
    pub fn changed(&self) -> bool {
        self.added + self.updated + self.removed > 0
    }
}

pub struct DesktopRepo<'a> {
    conn: &'a Connection,
}

const ITEM_COLUMNS: &str =
    "id, path, source, display_name, kind, ext, size_bytes, modified_at, missing";

impl<'a> DesktopRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Replace the index with freshly scanned reality, in one transaction:
    /// readers see either the old or the new index, never a half-updated mix.
    /// Items that disappeared stay as `missing = 1` history rows (layout
    /// restore in M3 needs to know what used to exist); they are not shown.
    pub fn sync_scan(&self, items: &[ScannedItem]) -> AppResult<SyncOutcome> {
        let now = now_millis();
        let mut outcome = SyncOutcome::default();
        let tx = self.conn.unchecked_transaction()?;

        let mut current: HashMap<String, (Option<i64>, Option<i64>, String)> = HashMap::new();
        {
            let mut stmt = tx.prepare(
                "SELECT path, size_bytes, modified_at, display_name
                 FROM desktop_items WHERE missing = 0",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<i64>>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?;
            for row in rows {
                let (path, size, mtime, name) = row?;
                current.insert(path, (size, mtime, name));
            }
        }

        for item in items {
            match current.get(&item.path) {
                Some((size, mtime, name)) => {
                    let content_changed = *size != item.size_bytes || *mtime != item.modified_at;
                    let renamed = *name != item.display_name;
                    tx.execute(
                        "UPDATE desktop_items SET last_seen_at = ?1, missing = 0,
                             size_bytes = ?2, modified_at = ?3, display_name = ?4,
                             kind = ?5, ext = ?6, source = ?7
                         WHERE path = ?8",
                        params![
                            now,
                            item.size_bytes,
                            item.modified_at,
                            item.display_name,
                            item.kind,
                            item.ext,
                            item.source,
                            item.path,
                        ],
                    )?;
                    if content_changed || renamed {
                        outcome.updated += 1;
                    }
                }
                None => {
                    tx.execute(
                        "INSERT INTO desktop_items
                             (path, source, display_name, kind, ext, size_bytes,
                              modified_at, first_seen_at, last_seen_at, missing)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, 0)
                         ON CONFLICT(path) DO UPDATE SET
                             missing = 0, last_seen_at = ?8, source = ?2,
                             display_name = ?3, kind = ?4, ext = ?5,
                             size_bytes = ?6, modified_at = ?7",
                        params![
                            item.path,
                            item.source,
                            item.display_name,
                            item.kind,
                            item.ext,
                            item.size_bytes,
                            item.modified_at,
                            now,
                        ],
                    )?;
                    outcome.added += 1;
                }
            }
        }

        for path in current.keys() {
            if !items.iter().any(|it| &it.path == path) {
                tx.execute(
                    "UPDATE desktop_items SET missing = 1 WHERE path = ?1",
                    params![path],
                )?;
                outcome.removed += 1;
            }
        }

        tx.commit()?;
        Ok(outcome)
    }

    /// Everything currently present on disk (the UI list).
    pub fn list_visible(&self) -> AppResult<Vec<DesktopItem>> {
        self.query_items(
            "SELECT {cols} FROM desktop_items WHERE missing = 0
                          ORDER BY kind, display_name COLLATE NOCASE",
        )
    }

    /// Case-insensitive substring search over display names.
    pub fn search(&self, query: &str) -> AppResult<Vec<DesktopItem>> {
        // LIKE wildcards in user input must match literally.
        let escaped = query
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let pattern = format!("%{escaped}%");
        let sql = format!(
            "SELECT {ITEM_COLUMNS} FROM desktop_items
             WHERE missing = 0 AND display_name LIKE ?1 ESCAPE '\\'
             ORDER BY kind, display_name COLLATE NOCASE"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let items = stmt
            .query_map([pattern], map_item)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(items)
    }

    /// Look up one visible item by exact path (open allow-list).
    pub fn find_visible(&self, path: &str) -> AppResult<Option<DesktopItem>> {
        let sql =
            format!("SELECT {ITEM_COLUMNS} FROM desktop_items WHERE missing = 0 AND path = ?1");
        let mut stmt = self.conn.prepare(&sql)?;
        let mut rows = stmt.query_map([path], map_item)?;
        match rows.next() {
            Some(item) => Ok(Some(item?)),
            None => Ok(None),
        }
    }

    fn query_items(&self, sql: &str) -> AppResult<Vec<DesktopItem>> {
        let sql = sql.replace("{cols}", ITEM_COLUMNS);
        let mut stmt = self.conn.prepare(&sql)?;
        let items = stmt
            .query_map([], map_item)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(items)
    }
}

fn map_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<DesktopItem> {
    Ok(DesktopItem {
        id: row.get(0)?,
        path: row.get(1)?,
        source: row.get(2)?,
        display_name: row.get(3)?,
        kind: row.get(4)?,
        ext: row.get(5)?,
        size_bytes: row.get(6)?,
        modified_at: row.get(7)?,
        missing: row.get::<_, i64>(8)? != 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::discovery::USER_DESKTOP;
    use crate::storage::Database;
    use std::fs;
    use std::path::Path;

    fn scanned(dir: &Path, name: &str) -> ScannedItem {
        crate::desktop::scanner::scan_desktop_dir(dir, USER_DESKTOP)
            .into_iter()
            .find(|it| it.path.ends_with(name))
            .unwrap()
    }

    fn sql_i64(db: &mut Database, sql: &str) -> i64 {
        db.conn().query_row(sql, [], |r| r.get(0)).unwrap()
    }

    #[test]
    fn sync_is_idempotent_and_tracks_changes() {
        let mut db = Database::open_in_memory().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("a.txt"), "v1").unwrap();

        // First pass: one add.
        let items = vec![scanned(tmp.path(), "a.txt")];
        let out = DesktopRepo::new(db.conn()).sync_scan(&items).unwrap();
        assert_eq!(
            out,
            SyncOutcome {
                added: 1,
                updated: 0,
                removed: 0
            }
        );

        // Same reality again: nothing changed.
        let out = DesktopRepo::new(db.conn()).sync_scan(&items).unwrap();
        assert_eq!(out, SyncOutcome::default());

        // Content changed (size differs): one update.
        fs::write(tmp.path().join("a.txt"), "version-2-longer").unwrap();
        let items = vec![scanned(tmp.path(), "a.txt")];
        let out = DesktopRepo::new(db.conn()).sync_scan(&items).unwrap();
        assert_eq!(
            out,
            SyncOutcome {
                added: 0,
                updated: 1,
                removed: 0
            }
        );
        assert_eq!(
            DesktopRepo::new(db.conn()).list_visible().unwrap()[0].size_bytes,
            Some(16)
        );

        // File vanished: marked missing, not listed anymore, still on record.
        fs::remove_file(tmp.path().join("a.txt")).unwrap();
        let out = DesktopRepo::new(db.conn()).sync_scan(&[]).unwrap();
        assert_eq!(
            out,
            SyncOutcome {
                added: 0,
                updated: 0,
                removed: 1
            }
        );
        assert!(DesktopRepo::new(db.conn())
            .list_visible()
            .unwrap()
            .is_empty());
        assert_eq!(sql_i64(&mut db, "SELECT missing FROM desktop_items"), 1);

        // It comes back: counted as added, single row (no duplicate).
        fs::write(tmp.path().join("a.txt"), "back").unwrap();
        let items = vec![scanned(tmp.path(), "a.txt")];
        let out = DesktopRepo::new(db.conn()).sync_scan(&items).unwrap();
        assert_eq!(
            out,
            SyncOutcome {
                added: 1,
                updated: 0,
                removed: 0
            }
        );
        assert_eq!(sql_i64(&mut db, "SELECT COUNT(*) FROM desktop_items"), 1);
    }

    #[test]
    fn search_is_substring_and_literal() {
        let mut db = Database::open_in_memory().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("报告_2026.txt"), "x").unwrap();
        fs::write(tmp.path().join("其他.txt"), "x").unwrap();
        DesktopRepo::new(db.conn())
            .sync_scan(&crate::desktop::scanner::scan_desktop_dir(
                tmp.path(),
                USER_DESKTOP,
            ))
            .unwrap();
        let repo = DesktopRepo::new(db.conn());

        assert_eq!(repo.search("报告").unwrap().len(), 1);
        assert_eq!(repo.search("2026").unwrap().len(), 1);
        // `_` is a literal underscore here, not a LIKE wildcard.
        assert_eq!(repo.search("报告_2026").unwrap().len(), 1);
        assert_eq!(repo.search("__").unwrap().len(), 0);
        assert_eq!(repo.search("不存在").unwrap().len(), 0);
    }

    #[test]
    fn find_visible_rejects_unknown_paths() {
        let mut db = Database::open_in_memory().unwrap();
        let repo = DesktopRepo::new(db.conn());
        assert_eq!(repo.find_visible("C:\\Windows\\notepad.exe").unwrap(), None);
    }
}
