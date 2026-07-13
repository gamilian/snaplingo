use std::collections::BTreeSet;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Row};

use crate::application::history::{
    HistoryEntry, HistoryRepository, OcrHistoryEntry, TranslationHistoryEntry,
};
use crate::domain::ocr::{OcrRequest, OcrResult};
use crate::domain::translation::{TranslationRequest, TranslationResult};
use crate::{AppError, Result};

use super::Database;

/// SQLite repository for translation and OCR history plus its user metadata.
pub struct SqliteHistoryRepository {
    database: Arc<Database>,
}

impl SqliteHistoryRepository {
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }

    #[cfg(test)]
    pub fn new_in_memory() -> Result<Self> {
        Ok(Self::new(Arc::new(Database::in_memory()?)))
    }

    pub async fn insert_translation(
        &self,
        request: &TranslationRequest,
        results: &[TranslationResult],
        providers_used: &[String],
        timestamp: DateTime<Utc>,
        duration_ms: u64,
    ) -> Result<()> {
        self.database.with_transaction(|transaction| {
            transaction.execute(
                "INSERT INTO history_records (kind, timestamp) VALUES ('translation', ?1)",
                [timestamp.to_rfc3339()],
            )?;
            let history_id = transaction.last_insert_rowid();
            transaction.execute(
                "INSERT INTO translation_history
                 (history_id, source_text, source_lang, target_lang, providers_used, results, duration_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    history_id,
                    request.text,
                    request.source_lang,
                    request.target_lang,
                    serde_json::to_string(providers_used)?,
                    serde_json::to_string(results)?,
                    duration_ms as i64,
                ],
            )?;
            Ok(())
        })
    }

    pub async fn insert_ocr(
        &self,
        request: &OcrRequest,
        result: &OcrResult,
        provider_used: &str,
        timestamp: DateTime<Utc>,
        duration_ms: u64,
    ) -> Result<()> {
        let image_hash = format!("{:x}", md5::compute(&request.image_data));

        self.database.with_transaction(|transaction| {
            transaction.execute(
                "INSERT INTO history_records (kind, timestamp) VALUES ('ocr', ?1)",
                [timestamp.to_rfc3339()],
            )?;
            let history_id = transaction.last_insert_rowid();
            transaction.execute(
                "INSERT INTO ocr_history
                 (history_id, image_hash, language, provider_used, recognized_text, confidence, duration_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    history_id,
                    image_hash,
                    request.language.as_ref(),
                    provider_used,
                    result.text,
                    result.confidence,
                    duration_ms as i64,
                ],
            )?;
            Ok(())
        })
    }

    pub async fn query_translations(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<TranslationHistoryEntry>> {
        self.database.with_connection(|connection| {
            let mut statement = connection.prepare(
                "SELECT r.id, r.timestamp, r.favorite, r.note,
                        t.source_text, t.source_lang, t.target_lang, t.providers_used, t.results, t.duration_ms
                 FROM history_records r
                 JOIN translation_history t ON t.history_id = r.id
                 WHERE r.kind = 'translation'
                 ORDER BY r.timestamp DESC
                 LIMIT ?1 OFFSET ?2",
            )?;
            let rows = statement.query_map(params![limit as i64, offset as i64], |row| {
                translation_entry_from_row(connection, row)
            })?;
            rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
        })
    }

    pub async fn query_ocr(&self, limit: usize, offset: usize) -> Result<Vec<OcrHistoryEntry>> {
        self.database.with_connection(|connection| {
            let mut statement = connection.prepare(
                "SELECT r.id, r.timestamp, r.favorite, r.note,
                        o.image_hash, o.language, o.provider_used, o.recognized_text, o.confidence, o.duration_ms
                 FROM history_records r
                 JOIN ocr_history o ON o.history_id = r.id
                 WHERE r.kind = 'ocr'
                 ORDER BY r.timestamp DESC
                 LIMIT ?1 OFFSET ?2",
            )?;
            let rows = statement.query_map(params![limit as i64, offset as i64], |row| {
                ocr_entry_from_row(connection, row)
            })?;
            rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
        })
    }

    pub async fn search(&self, query: &str) -> Result<Vec<HistoryEntry>> {
        let pattern = format!("%{}%", query);
        self.database.with_connection(|connection| {
            let mut entries = Vec::new();
            let mut translation_statement = connection.prepare(
                "SELECT r.id, r.timestamp, r.favorite, r.note,
                        t.source_text, t.source_lang, t.target_lang, t.providers_used, t.results, t.duration_ms
                 FROM history_records r
                 JOIN translation_history t ON t.history_id = r.id
                 WHERE r.kind = 'translation' AND t.source_text LIKE ?1
                 ORDER BY r.timestamp DESC
                 LIMIT 50",
            )?;
            let translation_rows = translation_statement.query_map([&pattern], |row| {
                translation_entry_from_row(connection, row).map(HistoryEntry::Translation)
            })?;
            entries.extend(translation_rows.collect::<std::result::Result<Vec<_>, _>>()?);

            let mut ocr_statement = connection.prepare(
                "SELECT r.id, r.timestamp, r.favorite, r.note,
                        o.image_hash, o.language, o.provider_used, o.recognized_text, o.confidence, o.duration_ms
                 FROM history_records r
                 JOIN ocr_history o ON o.history_id = r.id
                 WHERE r.kind = 'ocr' AND o.recognized_text LIKE ?1
                 ORDER BY r.timestamp DESC
                 LIMIT 50",
            )?;
            let ocr_rows = ocr_statement.query_map([&pattern], |row| {
                ocr_entry_from_row(connection, row).map(HistoryEntry::Ocr)
            })?;
            entries.extend(ocr_rows.collect::<std::result::Result<Vec<_>, _>>()?);

            Ok(entries)
        })
    }

    pub async fn delete(&self, id: i64) -> Result<()> {
        self.database.with_transaction(|transaction| {
            let deleted = transaction.execute("DELETE FROM history_records WHERE id = ?1", [id])?;
            ensure_history_exists(deleted, id)
        })
    }

    pub async fn set_favorite(&self, id: i64, favorite: bool) -> Result<()> {
        self.database.with_transaction(|transaction| {
            let updated = transaction.execute(
                "UPDATE history_records SET favorite = ?1 WHERE id = ?2",
                params![i64::from(favorite), id],
            )?;
            ensure_history_exists(updated, id)
        })
    }

    pub async fn update_note(&self, id: i64, note: Option<String>) -> Result<()> {
        let note = note.and_then(|value| (!value.trim().is_empty()).then_some(value));
        self.database.with_transaction(|transaction| {
            let updated = transaction.execute(
                "UPDATE history_records SET note = ?1 WHERE id = ?2",
                params![note, id],
            )?;
            ensure_history_exists(updated, id)
        })
    }

    pub async fn replace_tags(&self, id: i64, tags: Vec<String>) -> Result<()> {
        let tags = normalize_tags(tags);
        self.database.with_transaction(|transaction| {
            ensure_history_exists(
                transaction.execute("UPDATE history_records SET id = id WHERE id = ?1", [id])?,
                id,
            )?;
            transaction.execute("DELETE FROM history_tags WHERE history_id = ?1", [id])?;

            for tag in tags {
                transaction.execute("INSERT OR IGNORE INTO tags (name) VALUES (?1)", [&tag])?;
                let tag_id: i64 = transaction.query_row(
                    "SELECT id FROM tags WHERE name = ?1",
                    [&tag],
                    |row| row.get(0),
                )?;
                transaction.execute(
                    "INSERT INTO history_tags (history_id, tag_id) VALUES (?1, ?2)",
                    params![id, tag_id],
                )?;
            }

            transaction.execute(
                "DELETE FROM tags
                 WHERE NOT EXISTS (SELECT 1 FROM history_tags WHERE history_tags.tag_id = tags.id)",
                [],
            )?;
            Ok(())
        })
    }

    pub async fn clear_all(&self) -> Result<()> {
        self.database.with_transaction(|transaction| {
            transaction.execute("DELETE FROM history_records", [])?;
            transaction.execute("DELETE FROM tags", [])?;
            Ok(())
        })
    }
}

fn translation_entry_from_row(
    connection: &Connection,
    row: &Row<'_>,
) -> rusqlite::Result<TranslationHistoryEntry> {
    let id: i64 = row.get(0)?;
    let timestamp: String = row.get(1)?;
    let providers_used: String = row.get(7)?;
    let results: String = row.get(8)?;
    Ok(TranslationHistoryEntry {
        id,
        timestamp: parse_timestamp(&timestamp),
        favorite: row.get::<_, i64>(2)? != 0,
        note: row.get(3)?,
        tags: history_tags(connection, id)?,
        source_text: row.get(4)?,
        source_lang: row.get(5)?,
        target_lang: row.get(6)?,
        providers_used: serde_json::from_str(&providers_used).unwrap_or_default(),
        results: serde_json::from_str(&results).unwrap_or_default(),
        duration_ms: row.get::<_, i64>(9)? as u64,
    })
}

fn ocr_entry_from_row(connection: &Connection, row: &Row<'_>) -> rusqlite::Result<OcrHistoryEntry> {
    let id: i64 = row.get(0)?;
    let timestamp: String = row.get(1)?;
    Ok(OcrHistoryEntry {
        id,
        timestamp: parse_timestamp(&timestamp),
        favorite: row.get::<_, i64>(2)? != 0,
        note: row.get(3)?,
        tags: history_tags(connection, id)?,
        image_hash: row.get(4)?,
        language: row.get(5)?,
        provider_used: row.get(6)?,
        recognized_text: row.get(7)?,
        confidence: row.get(8)?,
        duration_ms: row.get::<_, i64>(9)? as u64,
    })
}

fn history_tags(connection: &Connection, history_id: i64) -> rusqlite::Result<Vec<String>> {
    let mut statement = connection.prepare(
        "SELECT t.name
         FROM tags t
         JOIN history_tags ht ON ht.tag_id = t.id
         WHERE ht.history_id = ?1
         ORDER BY t.name COLLATE NOCASE",
    )?;
    let tags = statement
        .query_map([history_id], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(tags)
}

fn ensure_history_exists(changed_rows: usize, id: i64) -> Result<()> {
    if changed_rows == 0 {
        return Err(AppError::Other(format!(
            "History record '{}' was not found",
            id
        )));
    }
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

fn parse_timestamp(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

#[async_trait]
impl HistoryRepository for SqliteHistoryRepository {
    async fn insert_translation(
        &self,
        request: &TranslationRequest,
        results: &[TranslationResult],
        providers_used: &[String],
        timestamp: DateTime<Utc>,
        duration_ms: u64,
    ) -> Result<()> {
        SqliteHistoryRepository::insert_translation(
            self,
            request,
            results,
            providers_used,
            timestamp,
            duration_ms,
        )
        .await
    }

    async fn insert_ocr(
        &self,
        request: &OcrRequest,
        result: &OcrResult,
        provider_used: &str,
        timestamp: DateTime<Utc>,
        duration_ms: u64,
    ) -> Result<()> {
        SqliteHistoryRepository::insert_ocr(
            self,
            request,
            result,
            provider_used,
            timestamp,
            duration_ms,
        )
        .await
    }

    async fn query_translations(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<TranslationHistoryEntry>> {
        SqliteHistoryRepository::query_translations(self, limit, offset).await
    }

    async fn query_ocr(&self, limit: usize, offset: usize) -> Result<Vec<OcrHistoryEntry>> {
        SqliteHistoryRepository::query_ocr(self, limit, offset).await
    }

    async fn search(&self, query: &str) -> Result<Vec<HistoryEntry>> {
        SqliteHistoryRepository::search(self, query).await
    }

    async fn delete(&self, id: i64) -> Result<()> {
        SqliteHistoryRepository::delete(self, id).await
    }

    async fn set_favorite(&self, id: i64, favorite: bool) -> Result<()> {
        SqliteHistoryRepository::set_favorite(self, id, favorite).await
    }

    async fn update_note(&self, id: i64, note: Option<String>) -> Result<()> {
        SqliteHistoryRepository::update_note(self, id, note).await
    }

    async fn replace_tags(&self, id: i64, tags: Vec<String>) -> Result<()> {
        SqliteHistoryRepository::replace_tags(self, id, tags).await
    }

    async fn clear_all(&self) -> Result<()> {
        SqliteHistoryRepository::clear_all(self).await
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::SqliteHistoryRepository;
    use crate::domain::translation::{TranslationRequest, TranslationResult};

    async fn insert_translation(repository: &SqliteHistoryRepository) -> i64 {
        repository
            .insert_translation(
                &TranslationRequest {
                    text: "Hello".to_string(),
                    source_lang: "en".to_string(),
                    target_lang: "fr".to_string(),
                },
                &[TranslationResult {
                    provider_id: "demo".to_string(),
                    translated_text: "Bonjour".to_string(),
                    detected_language: None,
                    confidence: None,
                }],
                &["demo".to_string()],
                Utc::now(),
                10,
            )
            .await
            .unwrap();
        repository.query_translations(1, 0).await.unwrap()[0].id
    }

    #[tokio::test]
    async fn persists_history_metadata_with_the_record() {
        let repository = SqliteHistoryRepository::new_in_memory().unwrap();
        let id = insert_translation(&repository).await;

        repository.set_favorite(id, true).await.unwrap();
        repository
            .update_note(id, Some("keep this".to_string()))
            .await
            .unwrap();
        repository
            .replace_tags(
                id,
                vec!["work".to_string(), "work".to_string(), "rust".to_string()],
            )
            .await
            .unwrap();

        let entry = repository
            .query_translations(1, 0)
            .await
            .unwrap()
            .pop()
            .unwrap();
        assert!(entry.favorite);
        assert_eq!(entry.note.as_deref(), Some("keep this"));
        assert_eq!(entry.tags, vec!["rust", "work"]);
    }
}
