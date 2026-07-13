mod config_store;
mod history;
mod migrations;

pub use config_store::SqliteConfigStore;
pub use history::SqliteHistoryRepository;

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{Connection, Transaction};

use crate::Result;

/// Shared SQLite connection for SnapLingo's non-secret persistent state.
pub struct Database {
    connection: Mutex<Connection>,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut connection = Connection::open(path)?;
        configure_connection(&connection)?;
        migrations::migrate(&mut connection)?;

        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    #[cfg(test)]
    pub fn in_memory() -> Result<Self> {
        let mut connection = Connection::open_in_memory()?;
        configure_connection(&connection)?;
        migrations::migrate(&mut connection)?;

        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn with_connection<T>(
        &self,
        operation: impl FnOnce(&Connection) -> Result<T>,
    ) -> Result<T> {
        let connection = self.connection.lock().unwrap();
        operation(&connection)
    }

    pub fn with_transaction<T>(
        &self,
        operation: impl FnOnce(&Transaction<'_>) -> Result<T>,
    ) -> Result<T> {
        let mut connection = self.connection.lock().unwrap();
        let transaction = connection.transaction()?;
        let value = operation(&transaction)?;
        transaction.commit()?;
        Ok(value)
    }
}

fn configure_connection(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA busy_timeout = 5000;",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rusqlite::Connection;
    use tempfile::tempdir;

    use super::Database;

    #[test]
    fn initializes_and_reopens_the_current_schema() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("snaplingo.db");

        let database = Database::open(&path).unwrap();
        let version = database
            .with_connection(|connection| {
                Ok(connection.query_row("PRAGMA user_version", [], |row| row.get::<_, i32>(0))?)
            })
            .unwrap();
        assert_eq!(version, 1);

        drop(database);
        Database::open(&path).unwrap();
        assert!(fs::metadata(path).is_ok());
    }

    #[test]
    fn rejects_a_database_from_a_newer_application_version() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("future.db");
        let connection = Connection::open(&path).unwrap();
        connection.execute("PRAGMA user_version = 2", []).unwrap();
        drop(connection);

        let error = Database::open(&path).err().expect("newer schema must fail");
        assert!(error.to_string().contains("newer than this version"));
    }
}
