use std::path::Path;

use rusqlite::{Connection, OptionalExtension};

use crate::app::error::{AppError, AppResult};
use crate::app::logging::now_millis;

/// A single schema migration: (version, name, SQL).
type Migration = (i64, &'static str, &'static str);

/// Ordered list of all schema versions. NEVER edit an applied migration —
/// append a new one instead.
const MIGRATIONS: &[Migration] = &[
    (
        1,
        "initial_desktop_core",
        include_str!("migrations/0001_initial_desktop_core.sql"),
    ),
    (
        2,
        "layout_snapshots",
        include_str!("migrations/0002_layout_snapshots.sql"),
    ),
    (3, "scenes", include_str!("migrations/0003_scenes.sql")),
    (
        4,
        "tasks_calendar",
        include_str!("migrations/0004_tasks_calendar.sql"),
    ),
    (
        5,
        "focus_sessions",
        include_str!("migrations/0005_focus_sessions.sql"),
    ),
    (
        6,
        "collection_external_items",
        include_str!("migrations/0006_collection_external_items.sql"),
    ),
];

fn latest_version() -> i64 {
    MIGRATIONS.last().map_or(0, |(v, _, _)| *v)
}

/// Run all pending migrations, each inside its own transaction.
///
/// If an existing database is being upgraded, a timestamped backup copy is
/// written next to it first (see docs — data safety is the top priority).
pub fn run(conn: &mut Connection, db_path: Option<&Path>) -> AppResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at INTEGER NOT NULL
        )",
    )?;

    let current = current_version(conn)?;

    let latest = latest_version();
    if current > latest {
        return Err(AppError::Other(format!(
            "database schema version {current} is newer than this build supports ({latest})"
        )));
    }

    if current > 0 && current < latest {
        if let Some(path) = db_path {
            backup_before_upgrade(conn, path, current)?;
        }
    }

    for (version, name, sql) in MIGRATIONS {
        if *version <= current {
            continue;
        }
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.execute(
            "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![version, name, now_millis()],
        )?;
        tx.commit()?;
        tracing::info!(version, name, "migration applied");
    }
    Ok(())
}

/// Copy the database file (after a WAL checkpoint) to a backup directory
/// beside it. Backup failure aborts the upgrade rather than risking data.
fn backup_before_upgrade(conn: &Connection, db_path: &Path, from_version: i64) -> AppResult<()> {
    let Some(parent) = db_path.parent() else {
        return Ok(());
    };
    let backup_dir = parent.join("backups");
    std::fs::create_dir_all(&backup_dir)?;
    conn.pragma_update(None, "wal_checkpoint", "TRUNCATE")?;
    let target = backup_dir.join(format!(
        "desktopmanager-v{from_version}-{}.db",
        now_millis()
    ));
    std::fs::copy(db_path, &target)?;
    tracing::info!(backup = %target.display(), "database backed up before migration");
    Ok(())
}

/// Version recorded in the database (0 when fresh).
pub fn current_version(conn: &Connection) -> AppResult<i64> {
    let v: Option<Option<i64>> = conn
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
            row.get::<_, Option<i64>>(0)
        })
        .optional()?;
    Ok(v.flatten().unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_database_reaches_latest_version() {
        let mut db = crate::storage::Database::open_in_memory().unwrap();
        assert_eq!(current_version(db.conn()).unwrap(), latest_version());
    }

    #[test]
    fn migrations_are_idempotent() {
        let mut db = crate::storage::Database::open_in_memory().unwrap();
        // Running again must be a no-op, not an error (e.g. duplicate tables).
        super::run(db.conn(), None).unwrap();
        assert_eq!(current_version(db.conn()).unwrap(), latest_version());
    }

    #[test]
    fn all_migrations_have_unique_versions() {
        let mut versions: Vec<i64> = MIGRATIONS.iter().map(|(v, _, _)| *v).collect();
        versions.sort_unstable();
        versions.dedup();
        assert_eq!(versions.len(), MIGRATIONS.len());
    }
}
