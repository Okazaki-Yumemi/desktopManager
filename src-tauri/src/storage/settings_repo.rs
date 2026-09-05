use rusqlite::Connection;

use crate::app::error::AppResult;
use crate::app::logging::now_millis;

/// Key/value settings persisted as JSON in the `settings` table.
pub struct SettingsRepo<'a> {
    conn: &'a Connection,
}

impl<'a> SettingsRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn get(&self, key: &str) -> AppResult<Option<serde_json::Value>> {
        let raw: Option<String> = self
            .conn
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
                row.get(0)
            })
            .map(Some)
            .or_else(|err| match err {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })?;
        match raw {
            None => Ok(None),
            Some(text) => match serde_json::from_str(&text) {
                Ok(value) => Ok(Some(value)),
                Err(err) => {
                    // Corrupt JSON must not take the app down; treat as unset.
                    tracing::warn!(key, %err, "settings value is not valid JSON, ignoring");
                    Ok(None)
                }
            },
        }
    }

    pub fn set(&self, key: &str, value: &serde_json::Value) -> AppResult<()> {
        self.conn.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            rusqlite::params![key, value.to_string(), now_millis()],
        )?;
        Ok(())
    }

    /// Convenience for plain-string values; exercised by tests, used by
    /// future typed settings accessors.
    #[allow(dead_code)]
    pub fn delete(&self, key: &str) -> AppResult<()> {
        self.conn
            .execute("DELETE FROM settings WHERE key = ?1", [key])?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_string(&self, key: &str) -> AppResult<Option<String>> {
        Ok(self.get(key)?.and_then(|v| v.as_str().map(str::to_owned)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;

    fn repo(db: &mut Database) -> SettingsRepo<'_> {
        SettingsRepo::new(db.conn())
    }

    #[test]
    fn set_get_roundtrip() {
        let mut db = Database::open_in_memory().unwrap();
        let r = repo(&mut db);
        assert_eq!(r.get("theme").unwrap(), None);

        r.set("theme", &serde_json::json!("dark")).unwrap();
        assert_eq!(r.get("theme").unwrap(), Some(serde_json::json!("dark")));

        r.set("theme", &serde_json::json!("light")).unwrap();
        assert_eq!(r.get_string("theme").unwrap(), Some("light".into()));
    }

    #[test]
    fn corrupt_json_is_treated_as_missing() {
        let mut db = Database::open_in_memory().unwrap();
        db.conn()
            .execute(
                "INSERT INTO settings (key, value, updated_at) VALUES ('k', 'not-json{', 0)",
                [],
            )
            .unwrap();
        assert_eq!(repo(&mut db).get("k").unwrap(), None);
    }

    #[test]
    fn delete_removes_key() {
        let mut db = Database::open_in_memory().unwrap();
        let r = repo(&mut db);
        r.set("k", &serde_json::json!(1)).unwrap();
        r.delete("k").unwrap();
        assert_eq!(r.get("k").unwrap(), None);
    }
}
