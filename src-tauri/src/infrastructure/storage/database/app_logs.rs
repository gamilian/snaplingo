use std::sync::Arc;

use crate::application::settings::{AppLogEntry, AppLogRepository};
use chrono::{Duration, Utc};
use rusqlite::params;

use crate::domain::SettingsSnapshot;
use crate::Result;

use super::Database;

pub struct SqliteAppLogRepository {
    database: Arc<Database>,
}

impl SqliteAppLogRepository {
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }

    pub fn list(&self, limit: usize) -> Result<Vec<AppLogEntry>> {
        self.database.with_connection(|connection| {
            let mut statement = connection.prepare(
                "SELECT id, timestamp, level, target, message
                 FROM app_logs
                 ORDER BY id DESC
                 LIMIT ?1",
            )?;
            let rows = statement.query_map([limit.clamp(1, 1_000) as i64], |row| {
                Ok(AppLogEntry {
                    id: row.get(0)?,
                    timestamp: row.get(1)?,
                    level: row.get(2)?,
                    target: row.get(3)?,
                    message: row.get(4)?,
                })
            })?;
            Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
        })
    }

    pub fn clear(&self) -> Result<()> {
        self.database.with_connection(|connection| {
            connection.execute("DELETE FROM app_logs", [])?;
            Ok(())
        })
    }

    pub fn delete_expired(&self, retention_days: u16) -> Result<usize> {
        let cutoff = Utc::now() - Duration::days(i64::from(retention_days.clamp(1, 365)));
        self.database.with_connection(|connection| {
            Ok(connection.execute(
                "DELETE FROM app_logs WHERE timestamp < ?1",
                [cutoff.to_rfc3339()],
            )?)
        })
    }

    pub(crate) fn record(&self, level: &str, target: &str, message: &str) -> Result<()> {
        if !self.level_enabled(level) {
            return Ok(());
        }
        self.database.with_connection(|connection| {
            connection.execute(
                "INSERT INTO app_logs (timestamp, level, target, message)
                 VALUES (?1, ?2, ?3, ?4)",
                params![Utc::now().to_rfc3339(), level, target, message],
            )?;
            Ok(())
        })
    }

    fn level_enabled(&self, level: &str) -> bool {
        let configured_level = self.database.with_connection(|connection| {
            let payload = connection
                .query_row(
                    "SELECT payload_json FROM settings WHERE namespace = 'settings'",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .ok();
            Ok(payload
                .and_then(|value| serde_json::from_str::<SettingsSnapshot>(&value).ok())
                .unwrap_or_default()
                .general
                .log_level)
        });
        let configured = match configured_level.as_deref().unwrap_or("info") {
            "debug" => 0,
            "warn" => 2,
            "error" => 3,
            _ => 1,
        };
        let current = match level {
            "DEBUG" => 0,
            "WARN" => 2,
            "ERROR" => 3,
            _ => 1,
        };
        current >= configured
    }
}

impl AppLogRepository for SqliteAppLogRepository {
    fn list(&self, limit: usize) -> Result<Vec<AppLogEntry>> {
        SqliteAppLogRepository::list(self, limit)
    }
    fn clear(&self) -> Result<()> {
        SqliteAppLogRepository::clear(self)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::{Duration, Utc};
    use rusqlite::params;

    use super::{Database, SqliteAppLogRepository};

    #[test]
    fn writes_and_clears_structured_application_logs_in_sqlite() {
        let database = Arc::new(Database::in_memory().unwrap());
        let repository = SqliteAppLogRepository::new(database);
        repository
            .record("INFO", "capture", "selection ready")
            .unwrap();
        repository
            .record("DEBUG", "capture", "filtered at default level")
            .unwrap();
        let entries = repository.list(20).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, "INFO");
        assert_eq!(entries[0].target, "capture");
        assert_eq!(entries[0].message, "selection ready");

        repository.clear().unwrap();
        assert!(repository.list(20).unwrap().is_empty());
    }

    #[test]
    fn deletes_only_logs_older_than_the_retention_period() {
        let database = Arc::new(Database::in_memory().unwrap());
        database
            .with_connection(|connection| {
                connection.execute(
                    "INSERT INTO app_logs (timestamp, level, target, message)
                     VALUES (?1, 'INFO', 'test', 'expired'),
                            (?2, 'INFO', 'test', 'current')",
                    params![
                        (Utc::now() - Duration::days(8)).to_rfc3339(),
                        Utc::now().to_rfc3339()
                    ],
                )?;
                Ok(())
            })
            .unwrap();
        let repository = SqliteAppLogRepository::new(database);

        assert_eq!(repository.delete_expired(7).unwrap(), 1);
        let entries = repository.list(20).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].message, "current");
    }
}
