pub mod calendar_repo;
pub mod collections_repo;
pub mod desktop_repo;
pub mod focus_repo;
pub mod layout_repo;
pub mod migrations;
pub mod scenes_repo;
pub mod settings_repo;
pub mod tasks_repo;

use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::Connection;

use crate::app::error::{AppError, AppResult};

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

    /// Open with corrupted-file first aid (M8): try a normal open, and if
    /// the file is not a usable database, quarantine it and start fresh.
    ///
    /// Detection covers a file rejected outright (garbage bytes surface as
    /// SQLITE_NOTADB on the first pragma) and a file whose header survives
    /// but whose pages fail `PRAGMA quick_check`. Quarantined files are
    /// renamed, never deleted, so the user can still salvage them manually.
    pub fn open_with_recovery(path: &Path) -> AppResult<(Self, Option<RecoveryReport>)> {
        match Self::open(path) {
            Ok(db) => {
                if db.quick_check_ok() {
                    return Ok((db, None));
                }
                tracing::warn!(
                    db = %path.display(),
                    "quick_check failed; quarantining database file"
                );
                Self::quarantine_and_reopen(path).map(|(db, report)| (db, Some(report)))
            }
            Err(AppError::Db(err)) if is_corruption(&err) => {
                tracing::warn!(
                    db = %path.display(),
                    %err,
                    "database file is corrupt; quarantining"
                );
                Self::quarantine_and_reopen(path).map(|(db, report)| (db, Some(report)))
            }
            Err(err) => Err(err),
        }
    }

    fn quarantine_and_reopen(path: &Path) -> AppResult<(Self, RecoveryReport)> {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or_default();
        let mut quarantined = Vec::new();
        for suspect in [path.to_path_buf(), wal_sibling(path), shm_sibling(path)] {
            if !suspect.exists() {
                continue;
            }
            let mut renamed = suspect.clone().into_os_string();
            renamed.push(format!(".corrupt-{stamp}"));
            let target = PathBuf::from(renamed);
            std::fs::rename(&suspect, &target)?;
            quarantined.push(target);
        }
        let db = Self::open(path)?;
        Ok((db, RecoveryReport { quarantined }))
    }

    fn quick_check_ok(&self) -> bool {
        let result: Result<String, _> = self
            .conn
            .query_row("PRAGMA quick_check", [], |row| row.get(0));
        matches!(result, Ok(text) if text == "ok")
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

/// What `open_with_recovery` did to rescue a broken database file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryReport {
    /// Files moved aside (the db plus any -wal/-shm siblings), original
    /// bytes intact under new names.
    pub quarantined: Vec<PathBuf>,
}

/// Corruption-class SQLite failures worth quarantining over (everything
/// else keeps failing loudly instead of silently wiping state).
fn is_corruption(err: &rusqlite::Error) -> bool {
    matches!(
        err,
        rusqlite::Error::SqliteFailure(ffi, _)
            if matches!(
                ffi.code,
                rusqlite::ErrorCode::NotADatabase | rusqlite::ErrorCode::DatabaseCorrupt
            )
    )
}

fn wal_sibling(path: &Path) -> PathBuf {
    with_suffix(path, "-wal")
}

fn shm_sibling(path: &Path) -> PathBuf {
    with_suffix(path, "-shm")
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut os = path.as_os_str().to_os_string();
    os.push(suffix);
    PathBuf::from(os)
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

    #[test]
    fn recovery_quarantines_garbage_db_and_starts_fresh() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let garbage: &[u8] = b"definitely not a sqlite database";
        std::fs::write(&db_path, garbage).unwrap();

        let (mut db, report) = Database::open_with_recovery(&db_path).unwrap();
        let report = report.expect("garbage file must be quarantined");
        assert_eq!(report.quarantined.len(), 1);
        assert_eq!(std::fs::read(&report.quarantined[0]).unwrap(), garbage);

        // The replacement database is real and usable.
        db.conn()
            .execute(
                "INSERT INTO settings (key, value, updated_at) VALUES ('k', '{}', 1)",
                [],
            )
            .unwrap();
        assert_eq!(count(&mut db, "settings"), 1);
        assert!(db_path.exists());
    }

    #[test]
    fn recovery_clears_corrupt_db_with_stale_siblings() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let garbage: &[u8] = b"garbage main file";
        std::fs::write(&db_path, garbage).unwrap();
        // Pre-existing siblings: after recovery none may remain at the real
        // path (quarantined by us, or cleaned up by SQLite itself — the
        // guarantee is the same).
        std::fs::write(with_suffix(&db_path, "-wal"), b"stale wal").unwrap();
        std::fs::write(with_suffix(&db_path, "-shm"), b"stale shm").unwrap();

        let (mut db, report) = Database::open_with_recovery(&db_path).unwrap();
        let report = report.expect("corrupt db must be quarantined");
        assert_eq!(report.quarantined.len(), 1);
        assert_eq!(std::fs::read(&report.quarantined[0]).unwrap(), garbage);
        // Stale sibling bytes never survive at the real path: SQLite cleans
        // them up during the failed open, or they are quarantined and then
        // replaced by the fresh database's own WAL files.
        let wal_now = std::fs::read(with_suffix(&db_path, "-wal")).ok();
        assert_ne!(wal_now.as_deref(), Some(b"stale wal".as_slice()));

        // The fresh database really works.
        db.conn()
            .execute(
                "INSERT INTO settings (key, value, updated_at) VALUES ('k', '{}', 1)",
                [],
            )
            .unwrap();
        assert_eq!(count(&mut db, "settings"), 1);
    }

    #[test]
    fn recovery_leaves_healthy_db_alone() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        {
            let (mut db, report) = Database::open_with_recovery(&db_path).unwrap();
            assert!(report.is_none());
            db.conn()
                .execute(
                    "INSERT INTO settings (key, value, updated_at) VALUES ('k', '{}', 1)",
                    [],
                )
                .unwrap();
        }

        let (mut db, report) = Database::open_with_recovery(&db_path).unwrap();
        assert!(report.is_none());
        assert_eq!(count(&mut db, "settings"), 1);
    }
}
