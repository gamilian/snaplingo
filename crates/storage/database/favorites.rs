use std::collections::BTreeSet;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::application::favorites::{
    FavoriteContent, FavoriteKind, FavoritePage, FavoriteQuery, FavoriteRecord, FavoriteRepository,
};
use crate::{AppError, Result};

use super::Database;

pub struct SqliteFavoriteRepository {
    database: Arc<Database>,
}

impl SqliteFavoriteRepository {
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

#[async_trait]
impl FavoriteRepository for SqliteFavoriteRepository {
    async fn find_by_fingerprint(&self, fingerprint: &str) -> Result<Option<FavoriteRecord>> {
        self.database.with_connection(|connection| {
            connection
                .query_row(
                    "SELECT id, kind, source_history_id, created_at, content_json, note
                     FROM favorites WHERE fingerprint = ?1",
                    [fingerprint],
                    |row| favorite_from_row(connection, row),
                )
                .optional()
                .map_err(snaplingo_core::AppError::database)
        })
    }

    async fn insert(
        &self,
        fingerprint: &str,
        source_history_id: Option<i64>,
        content: &FavoriteContent,
        created_at: DateTime<Utc>,
    ) -> Result<FavoriteRecord> {
        let kind = kind_value(content.kind());
        let content_json = serde_json::to_string(content)?;
        self.database.with_transaction(|transaction| {
            transaction
                .execute(
                    "INSERT INTO favorites
                 (kind, source_history_id, created_at, fingerprint, content_json, note)
                 VALUES (?1, ?2, ?3, ?4, ?5, NULL)",
                    params![
                        kind,
                        source_history_id,
                        created_at.to_rfc3339(),
                        fingerprint,
                        content_json
                    ],
                )
                .map_err(snaplingo_core::AppError::database)?;
            let id = transaction.last_insert_rowid();
            Ok(FavoriteRecord {
                id,
                created_at,
                source_history_id,
                content: content.clone(),
                note: None,
                tags: Vec::new(),
                thumbnail_data_url: None,
            })
        })
    }

    async fn query(&self, query: &FavoriteQuery) -> Result<FavoritePage> {
        let kind = query.kind.map(kind_value).unwrap_or("");
        let search = query.search.as_deref().unwrap_or("").trim();
        let pattern = format!("%{search}%");
        let tag = query.tag.as_deref().unwrap_or("").trim();
        let ids = query.ids.as_ref().map(serde_json::to_string).transpose()?;
        self.database.with_connection(|connection| {
            let total = connection
                .query_row(
                    "SELECT COUNT(*) FROM favorites f
                 WHERE (?1 = '' OR f.kind = ?1)
                   AND (?2 = '' OR f.content_json LIKE ?3 OR COALESCE(f.note, '') LIKE ?3)
                   AND (?4 = '' OR EXISTS (
                       SELECT 1 FROM favorite_tags ft
                       JOIN tags t ON t.id = ft.tag_id
                       WHERE ft.favorite_id = f.id AND t.name = ?4
                   ))
                   AND (?5 IS NULL OR f.id IN (SELECT value FROM json_each(?5)))",
                    params![kind, search, pattern, tag, ids],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(snaplingo_core::AppError::database)? as usize;
            let mut statement = connection
                .prepare(
                    "SELECT id, kind, source_history_id, created_at, content_json, note
                 FROM favorites f
                 WHERE (?1 = '' OR f.kind = ?1)
                   AND (?2 = '' OR f.content_json LIKE ?3 OR COALESCE(f.note, '') LIKE ?3)
                   AND (?4 = '' OR EXISTS (
                       SELECT 1 FROM favorite_tags ft
                       JOIN tags t ON t.id = ft.tag_id
                       WHERE ft.favorite_id = f.id AND t.name = ?4
                   ))
                   AND (?5 IS NULL OR f.id IN (SELECT value FROM json_each(?5)))
                 ORDER BY created_at DESC, id DESC LIMIT ?6 OFFSET ?7",
                )
                .map_err(snaplingo_core::AppError::database)?;
            let items = statement
                .query_map(
                    params![
                        kind,
                        search,
                        pattern,
                        tag,
                        ids,
                        query.limit as i64,
                        query.offset as i64
                    ],
                    |row| favorite_from_row(connection, row),
                )
                .map_err(snaplingo_core::AppError::database)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(snaplingo_core::AppError::database)?;
            Ok(FavoritePage { items, total })
        })
    }

    async fn matching_keys(&self, keys: &[String]) -> Result<Vec<String>> {
        let keys = serde_json::to_string(keys)?;
        self.database.with_connection(|connection| {
            let mut statement = connection.prepare(
                "SELECT requested.value FROM json_each(?1) requested
                 WHERE EXISTS (
                    SELECT 1 FROM favorites f
                    WHERE f.kind = json_extract(requested.value, '$[0]') AND (
                      (f.kind = 'translation'
                        AND json_extract(f.content_json, '$.snapshot.sourceText') IS json_extract(requested.value, '$[1]')
                        AND json_extract(f.content_json, '$.snapshot.sourceLang') IS json_extract(requested.value, '$[2]')
                        AND json_extract(f.content_json, '$.snapshot.targetLang') IS json_extract(requested.value, '$[3]')
                        AND json_extract(f.content_json, '$.snapshot.result.provider_id') IS json_extract(requested.value, '$[4]')
                        AND json_extract(f.content_json, '$.snapshot.result.translated_text') IS json_extract(requested.value, '$[5]'))
                      OR (f.kind = 'ocr'
                        AND json_extract(f.content_json, '$.snapshot.recognizedText') IS json_extract(requested.value, '$[1]')
                        AND json_extract(f.content_json, '$.snapshot.language') IS json_extract(requested.value, '$[2]')
                        AND json_extract(f.content_json, '$.snapshot.providerUsed') IS json_extract(requested.value, '$[3]'))
                    )
                 )"
            ).map_err(snaplingo_core::AppError::database)?;
            let matches = statement.query_map([keys], |row| row.get(0)).map_err(snaplingo_core::AppError::database)?.collect::<rusqlite::Result<Vec<String>>>().map_err(snaplingo_core::AppError::database)?;
            Ok(matches)
        })
    }

    async fn find(&self, id: i64) -> Result<Option<FavoriteRecord>> {
        self.database.with_connection(|connection| {
            connection
                .query_row(
                    "SELECT id, kind, source_history_id, created_at, content_json, note
                     FROM favorites WHERE id = ?1",
                    [id],
                    |row| favorite_from_row(connection, row),
                )
                .optional()
                .map_err(snaplingo_core::AppError::database)
        })
    }

    async fn update_metadata(
        &self,
        id: i64,
        note: Option<String>,
        tags: Vec<String>,
    ) -> Result<()> {
        let tags = normalize_tags(tags);
        self.database.with_transaction(|transaction| {
            let changed = transaction
                .execute(
                    "UPDATE favorites SET note = ?1 WHERE id = ?2",
                    params![note, id],
                )
                .map_err(snaplingo_core::AppError::database)?;
            if changed == 0 {
                return Err(AppError::Other(format!("Favorite {id} not found")));
            }
            transaction
                .execute("DELETE FROM favorite_tags WHERE favorite_id = ?1", [id])
                .map_err(snaplingo_core::AppError::database)?;
            for tag in tags {
                transaction
                    .execute("INSERT OR IGNORE INTO tags(name) VALUES (?1)", [&tag])
                    .map_err(snaplingo_core::AppError::database)?;
                let tag_id: i64 = transaction
                    .query_row("SELECT id FROM tags WHERE name = ?1", [&tag], |row| {
                        row.get(0)
                    })
                    .map_err(snaplingo_core::AppError::database)?;
                transaction
                    .execute(
                        "INSERT INTO favorite_tags(favorite_id, tag_id) VALUES (?1, ?2)",
                        params![id, tag_id],
                    )
                    .map_err(snaplingo_core::AppError::database)?;
            }
            cleanup_orphan_tags(transaction).map_err(snaplingo_core::AppError::database)?;
            Ok(())
        })
    }

    async fn delete(&self, id: i64) -> Result<()> {
        self.database.with_transaction(|transaction| {
            transaction
                .execute("DELETE FROM favorites WHERE id = ?1", [id])
                .map_err(snaplingo_core::AppError::database)?;
            cleanup_orphan_tags(transaction).map_err(snaplingo_core::AppError::database)?;
            Ok(())
        })
    }

    async fn list_tags(&self, kind: FavoriteKind) -> Result<Vec<String>> {
        self.database.with_connection(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT DISTINCT t.name FROM tags t
                 JOIN favorite_tags ft ON ft.tag_id = t.id
                 JOIN favorites f ON f.id = ft.favorite_id
                 WHERE f.kind = ?1 ORDER BY t.name COLLATE NOCASE",
                )
                .map_err(snaplingo_core::AppError::database)?;
            let tags = statement
                .query_map([kind_value(kind)], |row| row.get(0))
                .map_err(snaplingo_core::AppError::database)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(snaplingo_core::AppError::database)?;
            Ok(tags)
        })
    }
}

fn favorite_from_row(connection: &Connection, row: &Row<'_>) -> rusqlite::Result<FavoriteRecord> {
    let id: i64 = row.get(0)?;
    let created_at: String = row.get(3)?;
    let content_json: String = row.get(4)?;
    let content = serde_json::from_str(&content_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            content_json.len(),
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })?;
    Ok(FavoriteRecord {
        id,
        created_at: DateTime::parse_from_rfc3339(&created_at)
            .map(|value| value.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        source_history_id: row.get(2)?,
        content,
        note: row.get(5)?,
        tags: favorite_tags(connection, id)?,
        thumbnail_data_url: None,
    })
}

fn favorite_tags(connection: &Connection, favorite_id: i64) -> rusqlite::Result<Vec<String>> {
    let mut statement = connection.prepare(
        "SELECT t.name FROM tags t JOIN favorite_tags ft ON ft.tag_id = t.id
         WHERE ft.favorite_id = ?1 ORDER BY t.name COLLATE NOCASE",
    )?;
    let tags = statement
        .query_map([favorite_id], |row| row.get(0))?
        .collect();
    tags
}

fn cleanup_orphan_tags(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute(
        "DELETE FROM tags
         WHERE NOT EXISTS (SELECT 1 FROM history_tags ht WHERE ht.tag_id = tags.id)
           AND NOT EXISTS (SELECT 1 FROM screenshot_favorite_tags st WHERE st.tag_id = tags.id)
           AND NOT EXISTS (SELECT 1 FROM favorite_tags ft WHERE ft.tag_id = tags.id)",
        [],
    )?;
    Ok(())
}

fn normalize_tags(tags: Vec<String>) -> Vec<String> {
    tags.into_iter()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn kind_value(kind: FavoriteKind) -> &'static str {
    match kind {
        FavoriteKind::Translation => "translation",
        FavoriteKind::Ocr => "ocr",
    }
}
