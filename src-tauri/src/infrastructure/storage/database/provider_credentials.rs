use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::application::providers::{CredentialSnapshot, ProviderCredentialStore};
use crate::{AppError, Result};

use super::Database;

pub struct SqliteCredentialStore {
    database: Arc<Database>,
}

impl SqliteCredentialStore {
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }

    fn load_optional(&self, storage_key: &str) -> Result<Option<String>> {
        self.database
            .with_connection(|connection| load_optional(connection, storage_key))
    }
}

impl ProviderCredentialStore for SqliteCredentialStore {
    fn save_provider_credential(&self, provider_id: &str, api_key: &str) -> Result<()> {
        let credentials = HashMap::from([("api_key".to_string(), api_key.to_string())]);
        self.save_provider_credentials(provider_id, &credentials)
    }

    fn load_provider_credential(&self, provider_id: &str) -> Result<String> {
        let storage_key = credential_storage_key(provider_id, "api_key");
        self.load_optional(&storage_key)?
            .ok_or_else(|| AppError::CredentialNotFound(storage_key))
    }

    fn delete_provider_credential(&self, provider_id: &str) -> Result<()> {
        let storage_key = credential_storage_key(provider_id, "api_key");
        let deleted = self.database.with_transaction(|transaction| {
            Ok(transaction.execute(
                "DELETE FROM provider_credentials WHERE storage_key = ?1",
                [&storage_key],
            )?)
        })?;
        if deleted == 0 {
            return Err(AppError::CredentialNotFound(storage_key));
        }
        Ok(())
    }

    fn save_provider_credentials(
        &self,
        provider_id: &str,
        credentials: &HashMap<String, String>,
    ) -> Result<()> {
        let updated_at = Utc::now().timestamp_millis();
        self.database.with_transaction(|transaction| {
            for (field_name, value) in credentials {
                upsert(
                    transaction,
                    &credential_storage_key(provider_id, field_name),
                    value,
                    updated_at,
                )?;
            }
            Ok(())
        })
    }

    fn snapshot_provider_credentials(
        &self,
        provider_id: &str,
        field_names: &[String],
    ) -> Result<CredentialSnapshot> {
        self.database.with_connection(|connection| {
            let api_key =
                load_optional(connection, &credential_storage_key(provider_id, "api_key"))?;
            let mut snapshot = CredentialSnapshot {
                api_key: Some(api_key),
                structured: HashMap::new(),
            };
            for field_name in field_names {
                snapshot.structured.insert(
                    field_name.clone(),
                    load_optional(connection, &credential_storage_key(provider_id, field_name))?,
                );
            }
            Ok(snapshot)
        })
    }

    fn restore_provider_credentials(
        &self,
        provider_id: &str,
        snapshot: &CredentialSnapshot,
    ) -> Result<()> {
        let mut fields = snapshot.structured.clone();
        if let Some(api_key) = &snapshot.api_key {
            fields
                .entry("api_key".to_string())
                .or_insert_with(|| api_key.clone());
        }
        let updated_at = Utc::now().timestamp_millis();

        self.database.with_transaction(|transaction| {
            for (field_name, value) in &fields {
                let storage_key = credential_storage_key(provider_id, field_name);
                match value {
                    Some(value) => upsert(transaction, &storage_key, value, updated_at)?,
                    None => {
                        transaction.execute(
                            "DELETE FROM provider_credentials WHERE storage_key = ?1",
                            [&storage_key],
                        )?;
                    }
                }
            }
            Ok(())
        })
    }

    fn load_provider_credentials(
        &self,
        provider_id: &str,
        field_names: &[String],
    ) -> Result<HashMap<String, String>> {
        self.database.with_connection(|connection| {
            let mut credentials = HashMap::new();
            for field_name in field_names {
                if let Some(value) =
                    load_optional(connection, &credential_storage_key(provider_id, field_name))?
                {
                    credentials.insert(field_name.clone(), value);
                }
            }
            if credentials.is_empty() {
                return Err(AppError::CredentialNotFound(provider_id.to_string()));
            }
            Ok(credentials)
        })
    }

    fn delete_provider_credentials(&self, provider_id: &str, field_names: &[String]) -> Result<()> {
        self.database.with_transaction(|transaction| {
            for field_name in field_names {
                transaction.execute(
                    "DELETE FROM provider_credentials WHERE storage_key = ?1",
                    [credential_storage_key(provider_id, field_name)],
                )?;
            }
            Ok(())
        })
    }
}

fn credential_storage_key(provider_id: &str, field_name: &str) -> String {
    format!("provider:{provider_id}:credential:{field_name}")
}

fn load_optional(connection: &Connection, storage_key: &str) -> Result<Option<String>> {
    Ok(connection
        .query_row(
            "SELECT value FROM provider_credentials WHERE storage_key = ?1",
            [storage_key],
            |row| row.get::<_, String>(0),
        )
        .optional()?)
}

fn upsert(
    transaction: &Transaction<'_>,
    storage_key: &str,
    value: &str,
    updated_at: i64,
) -> Result<()> {
    transaction.execute(
        "INSERT INTO provider_credentials (storage_key, value, updated_at)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(storage_key) DO UPDATE SET
            value = excluded.value,
            updated_at = excluded.updated_at",
        params![storage_key, value, updated_at],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_and_structured_api_key_methods_share_one_sqlite_value() {
        let database = Arc::new(Database::in_memory().unwrap());
        let store = SqliteCredentialStore::new(database);

        store.save_provider_credential("custom", "secret").unwrap();

        assert_eq!(store.load_provider_credential("custom").unwrap(), "secret");
        assert_eq!(
            store
                .load_provider_credentials("custom", &["api_key".to_string()])
                .unwrap()
                .get("api_key"),
            Some(&"secret".to_string())
        );
    }

    #[test]
    fn multi_field_save_rolls_back_as_one_sql_transaction() {
        let database = Arc::new(Database::in_memory().unwrap());
        database
            .with_connection(|connection| {
                connection.execute_batch(
                    "CREATE TRIGGER reject_second_credential
                     BEFORE INSERT ON provider_credentials
                     WHEN NEW.storage_key LIKE '%:field2'
                     BEGIN
                       SELECT RAISE(ABORT, 'rejected field2');
                     END;",
                )?;
                Ok(())
            })
            .unwrap();
        let store = SqliteCredentialStore::new(database);
        let credentials = HashMap::from([
            ("field1".to_string(), "first".to_string()),
            ("field2".to_string(), "second".to_string()),
        ]);

        assert!(store
            .save_provider_credentials("custom", &credentials)
            .is_err());
        assert!(store
            .load_provider_credentials("custom", &["field1".to_string()])
            .is_err());
    }

    #[test]
    fn snapshot_restore_handles_present_and_absent_fields_atomically() {
        let database = Arc::new(Database::in_memory().unwrap());
        let store = SqliteCredentialStore::new(database);
        store
            .save_provider_credentials(
                "custom",
                &HashMap::from([("field1".to_string(), "original".to_string())]),
            )
            .unwrap();
        let snapshot = store
            .snapshot_provider_credentials("custom", &["field1".to_string(), "field2".to_string()])
            .unwrap();
        store
            .save_provider_credentials(
                "custom",
                &HashMap::from([
                    ("field1".to_string(), "changed".to_string()),
                    ("field2".to_string(), "added".to_string()),
                ]),
            )
            .unwrap();

        store
            .restore_provider_credentials("custom", &snapshot)
            .unwrap();

        assert_eq!(
            store
                .load_provider_credentials("custom", &["field1".to_string()])
                .unwrap()
                .get("field1"),
            Some(&"original".to_string())
        );
        assert!(store
            .load_provider_credentials("custom", &["field2".to_string()])
            .is_err());
    }
}
