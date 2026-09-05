pub mod collections_repo;
pub mod desktop_repo;
pub mod migrations;
pub mod settings_repo;

use std::path::Path;
use std::time::Duration;

use rusqlite::Connection;

use crate::app::error::AppResult;

/// A SQLite connection with pragmas applied and schema migrated.
///
/// This is the single entry point to the database. SQL must live in
/// repositories (or migrations) — never in command handlers or UI code.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open (creating if needed), apply pragmas and run pending migrations.
    pub fn open(path: &Path) -> AppResult<Self> {
        let mut conn = Connection::open(path)?;
        Self::configure(&mut conn)?;
        migrations::run(&mut conn, Some(path))?;
        Ok(Self { conn })
    }

    /// In-memory database, used by tests.
    #[cfg(test)]
    pub fn open_in_memory() -> AppResult<Self> {
        let mut conn = Connection::open_in_memory()?;
        Self::configure(&mut conn)?;
        migrations::run(&mut conn, None)?;
        Ok(Self { conn })
    }

    fn configure(conn: &mut Connection) -> AppResult<()> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.busy_timeout(Duration::from_millis(5_000))?;
        Ok(())
    }

    pub fn conn(&mut self) -> &mut Connection {
        &mut self.conn
    }

    /// Flush the WAL into the main file so a raw copy of the db file is
    /// complete (used before backups and purges).
    pub fn checkpoint(&self) -> AppResult<()> {
        self.conn
            .pragma_update(None, "wal_checkpoint", "TRUNCATE")?;
        Ok(())
    }

    /// Delete every collection and its assignments (metadata the user can
    /// rebuild; the desktop index stays).
    pub fn purge_collections(&self) -> AppResult<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM collection_items", [])?;
        tx.execute("DELETE FROM collections", [])?;
        tx.commit()?;
        Ok(())
    }

    /// Reset all user data: collections, index and settings. Real desktop
    /// files are never touched.
    pub fn purge_all(&self) -> AppResult<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM collection_items", [])?;
        tx.execute("DELETE FROM collections", [])?;
        tx.execute("DELETE FROM desktop_items", [])?;
        tx.execute("DELETE FROM settings", [])?;
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;

    fn count(db: &mut Database, table: &str) -> i64 {
        db.conn()
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    }

    fn seed(db: &mut Database) {
        db.conn()
            .execute(
                "INSERT INTO collections (name, color, sort_order, created_at, updated_at)
                 VALUES ('c', '#ffffff', 0, 1, 1)",
                [],
            )
            .unwrap();
        db.conn()
            .execute(
                "INSERT INTO desktop_items (path, source, display_name, kind, first_seen_at,
                                            last_seen_at)
                 VALUES (?1, 'user_desktop', 'x', 'file', 1, 1)",
                params!["C:\\x.txt"],
            )
            .unwrap();
        db.conn()
            .execute(
                "INSERT INTO settings (key, value, updated_at) VALUES ('k', '{}', 1)",
                [],
            )
            .unwrap();
    }

    #[test]
    fn purge_collections_keeps_index_and_settings() {
        let mut db = Database::open_in_memory().unwrap();
        seed(&mut db);
        db.purge_collections().unwrap();
        assert_eq!(count(&mut db, "collections"), 0);
        assert_eq!(count(&mut db, "collection_items"), 0);
        assert_eq!(count(&mut db, "desktop_items"), 1);
        assert_eq!(count(&mut db, "settings"), 1);
    }

    #[test]
    fn purge_all_resets_user_data() {
        let mut db = Database::open_in_memory().unwrap();
        seed(&mut db);
        db.purge_all().unwrap();
        assert_eq!(count(&mut db, "collections"), 0);
        assert_eq!(count(&mut db, "desktop_items"), 0);
        assert_eq!(count(&mut db, "settings"), 0);
    }
}
