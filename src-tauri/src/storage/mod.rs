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
}
