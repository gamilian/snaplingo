use crate::domain::ocr::{OcrRequest, OcrResult};
use crate::domain::translation::{TranslationRequest, TranslationResult};
use crate::Result;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

/// Database for storing history records
pub struct HistoryDatabase {
    conn: Mutex<Connection>,
}

/// Translation history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationHistoryEntry {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub source_text: String,
    pub source_lang: String,
    pub target_lang: String,
    pub providers_used: Vec<String>,
    pub results: Vec<TranslationResult>,
    pub duration_ms: u64,
}

/// OCR history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrHistoryEntry {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub image_hash: String,
    pub language: Option<String>,
    pub provider_used: String,
    pub recognized_text: String,
    pub confidence: Option<f64>,
    pub duration_ms: u64,
}

/// Generic history entry (for search results)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum HistoryEntry {
    Translation(TranslationHistoryEntry),
    Ocr(OcrHistoryEntry),
}

impl HistoryDatabase {
    /// Current database schema version
    /// Increment this when making schema changes
    const SCHEMA_VERSION: i32 = 1;

    /// Creates a new HistoryDatabase at the given path
    pub fn new(path: PathBuf) -> Result<Self> {
        let conn = Connection::open(path)?;

        // Check schema version compatibility
        let db_version: i32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap_or(0);

        if db_version != 0 && db_version != Self::SCHEMA_VERSION {
            return Err(crate::AppError::Other(format!(
                "Database schema version mismatch. Expected v{}, found v{}. \
                 Please backup and delete ~/.snaplingo/history.db to recreate with new schema.",
                Self::SCHEMA_VERSION,
                db_version
            )));
        }

        // Create translation history table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS translation_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                source_text TEXT NOT NULL,
                source_lang TEXT NOT NULL,
                target_lang TEXT NOT NULL,
                providers_used TEXT NOT NULL,
                results TEXT NOT NULL,
                duration_ms INTEGER NOT NULL
            )",
            [],
        )?;

        // Create OCR history table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS ocr_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                image_hash TEXT NOT NULL,
                language TEXT,
                provider_used TEXT NOT NULL,
                recognized_text TEXT NOT NULL,
                confidence REAL,
                duration_ms INTEGER NOT NULL
            )",
            [],
        )?;

        // Create indexes for performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_translation_timestamp
             ON translation_history(timestamp DESC)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_ocr_timestamp
             ON ocr_history(timestamp DESC)",
            [],
        )?;

        // Set schema version
        conn.execute(
            &format!("PRAGMA user_version = {}", Self::SCHEMA_VERSION),
            [],
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Insert a translation history record
    pub async fn insert_translation(
        &self,
        request: &TranslationRequest,
        results: &[TranslationResult],
        providers_used: &[String],
        timestamp: DateTime<Utc>,
        duration_ms: u64,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO translation_history
             (timestamp, source_text, source_lang, target_lang, providers_used, results, duration_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                timestamp.to_rfc3339(),
                request.text,
                request.source_lang,
                request.target_lang,
                serde_json::to_string(providers_used)?,
                serde_json::to_string(results)?,
                duration_ms as i64,
            ],
        )?;

        Ok(())
    }

    /// Insert an OCR history record
    pub async fn insert_ocr(
        &self,
        request: &OcrRequest,
        result: &OcrResult,
        provider_used: &str,
        timestamp: DateTime<Utc>,
        duration_ms: u64,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Compute a simple hash of the image data
        let image_hash = format!("{:x}", md5::compute(&request.image_data));

        conn.execute(
            "INSERT INTO ocr_history
             (timestamp, image_hash, language, provider_used, recognized_text, confidence, duration_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                timestamp.to_rfc3339(),
                image_hash,
                request.language.as_ref(),
                provider_used,
                result.text,
                result.confidence,
                duration_ms as i64,
            ],
        )?;

        Ok(())
    }

    /// Query translation history with pagination
    pub async fn query_translations(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<TranslationHistoryEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, source_text, source_lang, target_lang, providers_used, results, duration_ms
             FROM translation_history
             ORDER BY timestamp DESC
             LIMIT ?1 OFFSET ?2"
        )?;

        let rows = stmt.query_map(params![limit as i64, offset as i64], |row| {
            let timestamp_str: String = row.get(1)?;
            let providers_json: String = row.get(5)?;
            let results_json: String = row.get(6)?;

            Ok(TranslationHistoryEntry {
                id: row.get(0)?,
                timestamp: DateTime::parse_from_rfc3339(&timestamp_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                source_text: row.get(2)?,
                source_lang: row.get(3)?,
                target_lang: row.get(4)?,
                providers_used: serde_json::from_str(&providers_json).unwrap_or_default(),
                results: serde_json::from_str(&results_json).unwrap_or_default(),
                duration_ms: row.get::<_, i64>(7)? as u64,
            })
        })?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(row?);
        }

        Ok(entries)
    }

    /// Query OCR history with pagination
    pub async fn query_ocr(&self, limit: usize, offset: usize) -> Result<Vec<OcrHistoryEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, image_hash, language, provider_used, recognized_text, confidence, duration_ms
             FROM ocr_history
             ORDER BY timestamp DESC
             LIMIT ?1 OFFSET ?2"
        )?;

        let rows = stmt.query_map(params![limit as i64, offset as i64], |row| {
            let timestamp_str: String = row.get(1)?;

            Ok(OcrHistoryEntry {
                id: row.get(0)?,
                timestamp: DateTime::parse_from_rfc3339(&timestamp_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                image_hash: row.get(2)?,
                language: row.get(3)?,
                provider_used: row.get(4)?,
                recognized_text: row.get(5)?,
                confidence: row.get(6)?,
                duration_ms: row.get::<_, i64>(7)? as u64,
            })
        })?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(row?);
        }

        Ok(entries)
    }

    /// Search history (simplified - searches in source_text and recognized_text)
    pub async fn search(&self, query: &str) -> Result<Vec<HistoryEntry>> {
        let conn = self.conn.lock().unwrap();
        let search_pattern = format!("%{}%", query);

        // Search translations
        let mut translation_stmt = conn.prepare(
            "SELECT id, timestamp, source_text, source_lang, target_lang, providers_used, results, duration_ms
             FROM translation_history
             WHERE source_text LIKE ?1
             ORDER BY timestamp DESC
             LIMIT 50"
        )?;

        let translation_rows = translation_stmt.query_map(params![&search_pattern], |row| {
            let timestamp_str: String = row.get(1)?;
            let providers_json: String = row.get(5)?;
            let results_json: String = row.get(6)?;

            Ok(HistoryEntry::Translation(TranslationHistoryEntry {
                id: row.get(0)?,
                timestamp: DateTime::parse_from_rfc3339(&timestamp_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                source_text: row.get(2)?,
                source_lang: row.get(3)?,
                target_lang: row.get(4)?,
                providers_used: serde_json::from_str(&providers_json).unwrap_or_default(),
                results: serde_json::from_str(&results_json).unwrap_or_default(),
                duration_ms: row.get::<_, i64>(7)? as u64,
            }))
        })?;

        let mut entries = Vec::new();
        for row in translation_rows {
            entries.push(row?);
        }

        // Search OCR
        let mut ocr_stmt = conn.prepare(
            "SELECT id, timestamp, image_hash, language, provider_used, recognized_text, confidence, duration_ms
             FROM ocr_history
             WHERE recognized_text LIKE ?1
             ORDER BY timestamp DESC
             LIMIT 50"
        )?;

        let ocr_rows = ocr_stmt.query_map(params![&search_pattern], |row| {
            let timestamp_str: String = row.get(1)?;

            Ok(HistoryEntry::Ocr(OcrHistoryEntry {
                id: row.get(0)?,
                timestamp: DateTime::parse_from_rfc3339(&timestamp_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                image_hash: row.get(2)?,
                language: row.get(3)?,
                provider_used: row.get(4)?,
                recognized_text: row.get(5)?,
                confidence: row.get(6)?,
                duration_ms: row.get::<_, i64>(7)? as u64,
            }))
        })?;

        for row in ocr_rows {
            entries.push(row?);
        }

        Ok(entries)
    }

    /// Delete a history entry by ID (searches both tables)
    pub async fn delete(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute("DELETE FROM translation_history WHERE id = ?1", params![id])?;
        conn.execute("DELETE FROM ocr_history WHERE id = ?1", params![id])?;

        Ok(())
    }

    /// Clear all history
    pub async fn clear_all(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute("DELETE FROM translation_history", [])?;
        conn.execute("DELETE FROM ocr_history", [])?;

        Ok(())
    }
}
