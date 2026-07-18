use std::sync::Arc;

use chrono::Utc;
use rusqlite::{params, OptionalExtension};

use super::KeychainBackend;
use crate::infrastructure::storage::Database;
use crate::{AppError, Result};

pub(super) struct SqliteCredentialBackend {
    database: Arc<Database>,
}

impl SqliteCredentialBackend {
    pub(super) fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

impl KeychainBackend for SqliteCredentialBackend {
    fn save(&self, key: &str, value: &str) -> Result<()> {
        let updated_at = Utc::now().timestamp_millis();
        self.database.with_transaction(|transaction| {
            transaction.execute(
                "INSERT INTO provider_credentials (storage_key, value, updated_at)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(storage_key) DO UPDATE SET
                    value = excluded.value,
                    updated_at = excluded.updated_at",
                params![key, value, updated_at],
            )?;
            Ok(())
        })
    }

    fn load(&self, key: &str) -> Result<String> {
        self.database.with_connection(|connection| {
            connection
                .query_row(
                    "SELECT value FROM provider_credentials WHERE storage_key = ?1",
                    [key],
                    |row| row.get::<_, String>(0),
                )
                .optional()?
                .ok_or_else(|| AppError::CredentialNotFound(key.to_string()))
        })
    }

    fn delete(&self, key: &str) -> Result<()> {
        let deleted = self.database.with_transaction(|transaction| {
            Ok(transaction.execute(
                "DELETE FROM provider_credentials WHERE storage_key = ?1",
                [key],
            )?)
        })?;

        if deleted == 0 {
            return Err(AppError::CredentialNotFound(key.to_string()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_and_deletes_credentials_without_a_platform_backend() {
        let backend = SqliteCredentialBackend::new(Arc::new(Database::in_memory().unwrap()));

        backend.save("provider:test:api_key", "secret").unwrap();
        assert_eq!(backend.load("provider:test:api_key").unwrap(), "secret");

        backend.delete("provider:test:api_key").unwrap();
        assert!(matches!(
            backend.load("provider:test:api_key"),
            Err(AppError::CredentialNotFound(_))
        ));
    }
}
