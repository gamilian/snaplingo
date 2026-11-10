use rusqlite::{Connection, Transaction};

use crate::{AppError, Result};

const CURRENT_SCHEMA_VERSION: i32 = 6;

pub(super) fn migrate(connection: &mut Connection) -> Result<()> {
    let mut version: i32 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;

    if version > CURRENT_SCHEMA_VERSION {
        return Err(AppError::Other(format!(
            "Database schema v{} is newer than this version of SnapLingo supports (v{})",
            version, CURRENT_SCHEMA_VERSION
        )));
    }

    while version < CURRENT_SCHEMA_VERSION {
        let next_version = version + 1;
        let transaction = connection.transaction()?;
        match next_version {
            1 => migrate_to_v1(&transaction)?,
            2 => migrate_to_v2(&transaction)?,
            3 => migrate_to_v3(&transaction)?,
            4 => migrate_to_v4(&transaction)?,
            5 => migrate_to_v5(&transaction)?,
            6 => migrate_to_v6(&transaction)?,
            _ => unreachable!("missing migration for version {}", next_version),
        }
        transaction.pragma_update(None, "user_version", next_version)?;
        transaction.commit()?;
        version = next_version;
    }

    Ok(())
}

fn migrate_to_v5(transaction: &Transaction<'_>) -> Result<()> {
    transaction.execute_batch(
        "CREATE TABLE provider_credentials (
            storage_key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at INTEGER NOT NULL
        );",
    )?;
    Ok(())
}

fn migrate_to_v6(transaction: &Transaction<'_>) -> Result<()> {
    transaction.execute_batch(
        "INSERT INTO provider_credentials (storage_key, value, updated_at)
         SELECT
            substr(storage_key, 1, length(storage_key) - length(':api_key'))
                || ':credential:api_key',
            value,
            updated_at
         FROM provider_credentials
         WHERE storage_key GLOB 'provider:*:api_key'
           AND storage_key NOT GLOB 'provider:*:credential:*'
         ON CONFLICT(storage_key) DO UPDATE SET
            value = excluded.value,
            updated_at = excluded.updated_at
         WHERE excluded.updated_at >= provider_credentials.updated_at;

         DELETE FROM provider_credentials
         WHERE storage_key GLOB 'provider:*:api_key'
           AND storage_key NOT GLOB 'provider:*:credential:*';

         DROP TABLE IF EXISTS credential_store_metadata;",
    )?;
    Ok(())
}

fn migrate_to_v4(transaction: &Transaction<'_>) -> Result<()> {
    transaction.execute_batch(
        "CREATE TABLE app_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL,
            level TEXT NOT NULL CHECK(level IN ('DEBUG', 'INFO', 'WARN', 'ERROR')),
            target TEXT NOT NULL,
            message TEXT NOT NULL
        );

        CREATE INDEX idx_app_logs_timestamp ON app_logs(timestamp DESC);",
    )?;
    Ok(())
}

fn migrate_to_v3(transaction: &Transaction<'_>) -> Result<()> {
    transaction.execute_batch(
        "CREATE TABLE favorites (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            kind TEXT NOT NULL CHECK(kind IN ('translation', 'ocr')),
            source_history_id INTEGER,
            created_at TEXT NOT NULL,
            fingerprint TEXT NOT NULL UNIQUE,
            content_json TEXT NOT NULL CHECK(json_valid(content_json)),
            note TEXT
        );

        CREATE TABLE favorite_tags (
            favorite_id INTEGER NOT NULL REFERENCES favorites(id) ON DELETE CASCADE,
            tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            PRIMARY KEY(favorite_id, tag_id)
        );

        CREATE INDEX idx_favorites_kind_created_at
            ON favorites(kind, created_at DESC);
        CREATE INDEX idx_favorite_tags_tag_id
            ON favorite_tags(tag_id);",
    )?;
    Ok(())
}

fn migrate_to_v2(transaction: &Transaction<'_>) -> Result<()> {
    transaction.execute_batch(
        "ALTER TABLE ocr_history ADD COLUMN source_asset_path TEXT;
        ALTER TABLE ocr_history ADD COLUMN thumbnail_asset_path TEXT;

        CREATE TABLE screenshot_favorites (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            created_at TEXT NOT NULL,
            asset_path TEXT NOT NULL UNIQUE,
            thumbnail_path TEXT NOT NULL UNIQUE,
            width INTEGER NOT NULL,
            height INTEGER NOT NULL,
            note TEXT
        );

        CREATE TABLE screenshot_favorite_tags (
            favorite_id INTEGER NOT NULL REFERENCES screenshot_favorites(id) ON DELETE CASCADE,
            tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            PRIMARY KEY(favorite_id, tag_id)
        );

        CREATE INDEX idx_screenshot_favorites_created_at
            ON screenshot_favorites(created_at DESC);
        CREATE INDEX idx_screenshot_favorite_tags_tag_id
            ON screenshot_favorite_tags(tag_id);",
    )?;
    Ok(())
}

fn migrate_to_v1(transaction: &Transaction<'_>) -> Result<()> {
    transaction.execute_batch(
        "CREATE TABLE settings (
            namespace TEXT PRIMARY KEY,
            payload_version INTEGER NOT NULL,
            payload_json TEXT NOT NULL CHECK(json_valid(payload_json)),
            revision INTEGER NOT NULL DEFAULT 1,
            updated_at INTEGER NOT NULL
        );

        CREATE TABLE history_records (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            kind TEXT NOT NULL CHECK(kind IN ('translation', 'ocr')),
            timestamp TEXT NOT NULL,
            favorite INTEGER NOT NULL DEFAULT 0 CHECK(favorite IN (0, 1)),
            note TEXT
        );

        CREATE TABLE translation_history (
            history_id INTEGER PRIMARY KEY REFERENCES history_records(id) ON DELETE CASCADE,
            source_text TEXT NOT NULL,
            source_lang TEXT NOT NULL,
            target_lang TEXT NOT NULL,
            providers_used TEXT NOT NULL,
            results TEXT NOT NULL,
            duration_ms INTEGER NOT NULL
        );

        CREATE TABLE ocr_history (
            history_id INTEGER PRIMARY KEY REFERENCES history_records(id) ON DELETE CASCADE,
            image_hash TEXT NOT NULL,
            language TEXT,
            provider_used TEXT NOT NULL,
            recognized_text TEXT NOT NULL,
            confidence REAL,
            duration_ms INTEGER NOT NULL
        );

        CREATE TABLE tags (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE
        );

        CREATE TABLE history_tags (
            history_id INTEGER NOT NULL REFERENCES history_records(id) ON DELETE CASCADE,
            tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            PRIMARY KEY(history_id, tag_id)
        );

        CREATE INDEX idx_history_records_timestamp ON history_records(timestamp DESC);
        CREATE INDEX idx_history_records_favorite ON history_records(favorite, timestamp DESC);
        CREATE INDEX idx_history_tags_tag_id ON history_tags(tag_id);",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use rusqlite::{Connection, OptionalExtension};

    #[test]
    fn failed_migration_rolls_back_every_table_created_in_that_version() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .execute("CREATE TABLE ocr_history (id INTEGER PRIMARY KEY)", [])
            .unwrap();

        assert!(super::migrate(&mut connection).is_err());

        let settings_table = connection
            .query_row(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'settings'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .unwrap();
        let version: i32 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();

        assert_eq!(settings_table, None);
        assert_eq!(version, 0);
    }

    #[test]
    fn v6_normalizes_legacy_api_keys_and_removes_unused_metadata() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE provider_credentials (
                    storage_key TEXT PRIMARY KEY,
                    value TEXT NOT NULL,
                    updated_at INTEGER NOT NULL
                );
                CREATE TABLE credential_store_metadata (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );
                INSERT INTO provider_credentials VALUES (
                    'provider:custom:api_key', 'secret', 42
                );
                INSERT INTO provider_credentials VALUES (
                    'provider:newer:api_key', 'stale', 40
                );
                INSERT INTO provider_credentials VALUES (
                    'provider:newer:credential:api_key', 'fresh', 43
                );
                PRAGMA user_version = 5;",
            )
            .unwrap();

        super::migrate(&mut connection).unwrap();

        let credential: String = connection
            .query_row(
                "SELECT value FROM provider_credentials WHERE storage_key = ?1",
                ["provider:custom:credential:api_key"],
                |row| row.get(0),
            )
            .unwrap();
        let old_key_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM provider_credentials WHERE storage_key = ?1",
                ["provider:custom:api_key"],
                |row| row.get(0),
            )
            .unwrap();
        let metadata_table: Option<String> = connection
            .query_row(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'credential_store_metadata'",
                [],
                |row| row.get(0),
            )
            .optional()
            .unwrap();

        assert_eq!(credential, "secret");
        let newer_credential: String = connection
            .query_row(
                "SELECT value FROM provider_credentials WHERE storage_key = ?1",
                ["provider:newer:credential:api_key"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(newer_credential, "fresh");
        assert_eq!(old_key_count, 0);
        assert_eq!(metadata_table, None);
    }
}
