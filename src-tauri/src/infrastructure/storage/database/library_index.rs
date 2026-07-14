use std::sync::Arc;

use async_trait::async_trait;
use rusqlite::params;

use crate::application::library_index::{
    LibraryIndexItem, LibraryIndexKind, LibraryIndexPage, LibraryIndexQuery, LibraryIndexRepository,
};
use crate::Result;

use super::Database;

pub struct SqliteLibraryIndexRepository {
    database: Arc<Database>,
}

impl SqliteLibraryIndexRepository {
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

#[async_trait]
impl LibraryIndexRepository for SqliteLibraryIndexRepository {
    async fn query_history(&self, query: &LibraryIndexQuery) -> Result<LibraryIndexPage> {
        let search = query.search.trim();
        let pattern = format!("%{search}%");
        self.database.with_connection(|connection| {
            let total = connection.query_row(
                "SELECT COUNT(*)
                 FROM history_records r
                 LEFT JOIN translation_history t ON t.history_id = r.id
                 LEFT JOIN ocr_history o ON o.history_id = r.id
                 WHERE ?1 = ''
                    OR (r.kind = 'translation' AND (t.source_text LIKE ?2 OR t.results LIKE ?2))
                    OR (r.kind = 'ocr' AND o.recognized_text LIKE ?2)",
                params![search, pattern],
                |row| row.get::<_, i64>(0),
            )? as usize;
            let mut statement = connection.prepare(
                "WITH filtered AS (
                    SELECT r.id, r.kind, r.timestamp,
                           ROW_NUMBER() OVER (
                               PARTITION BY r.kind ORDER BY r.timestamp DESC, r.id DESC
                           ) - 1 AS source_offset,
                           CASE r.kind WHEN 'translation' THEN 0 ELSE 1 END AS source_order
                    FROM history_records r
                    LEFT JOIN translation_history t ON t.history_id = r.id
                    LEFT JOIN ocr_history o ON o.history_id = r.id
                    WHERE ?1 = ''
                       OR (r.kind = 'translation' AND (t.source_text LIKE ?2 OR t.results LIKE ?2))
                       OR (r.kind = 'ocr' AND o.recognized_text LIKE ?2)
                 )
                 SELECT id, kind, source_offset
                 FROM filtered
                 ORDER BY timestamp DESC, source_order, id DESC
                 LIMIT ?3 OFFSET ?4",
            )?;
            let items = statement
                .query_map(
                    params![search, pattern, query.limit as i64, query.offset as i64],
                    |row| {
                        let kind = match row.get::<_, String>(1)?.as_str() {
                            "translation" => LibraryIndexKind::Translation,
                            _ => LibraryIndexKind::Ocr,
                        };
                        Ok(LibraryIndexItem {
                            id: row.get(0)?,
                            kind,
                            source_offset: row.get::<_, i64>(2)? as usize,
                        })
                    },
                )?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(LibraryIndexPage { items, total })
        })
    }

    async fn query_favorites(&self, query: &LibraryIndexQuery) -> Result<LibraryIndexPage> {
        let search = query.search.trim();
        let pattern = format!("%{search}%");
        let lowercase_pattern = format!("%{}%", search.to_lowercase());
        self.database.with_connection(|connection| {
            let regular_total = connection.query_row(
                "SELECT COUNT(*) FROM favorites f
                 WHERE ?1 = '' OR f.content_json LIKE ?2 OR COALESCE(f.note, '') LIKE ?2",
                params![search, pattern],
                |row| row.get::<_, i64>(0),
            )?;
            let screenshot_total = connection.query_row(
                "SELECT COUNT(*) FROM screenshot_favorites sf
                 WHERE ?1 = ''
                    OR lower(COALESCE(sf.note, '')) LIKE ?2
                    OR EXISTS (
                        SELECT 1 FROM screenshot_favorite_tags sft
                        JOIN tags t ON t.id = sft.tag_id
                        WHERE sft.favorite_id = sf.id AND lower(t.name) LIKE ?2
                    )",
                params![search, lowercase_pattern],
                |row| row.get::<_, i64>(0),
            )?;
            let mut statement = connection.prepare(
                "WITH regular AS (
                    SELECT f.id, f.kind, f.created_at,
                           ROW_NUMBER() OVER (ORDER BY f.created_at DESC, f.id DESC) - 1
                               AS source_offset,
                           0 AS source_order
                    FROM favorites f
                    WHERE ?1 = '' OR f.content_json LIKE ?2 OR COALESCE(f.note, '') LIKE ?2
                 ),
                 screenshots AS (
                    SELECT sf.id, 'screenshot' AS kind, sf.created_at,
                           ROW_NUMBER() OVER (ORDER BY sf.created_at DESC, sf.id DESC) - 1
                               AS source_offset,
                           1 AS source_order
                    FROM screenshot_favorites sf
                    WHERE ?1 = ''
                       OR lower(COALESCE(sf.note, '')) LIKE ?3
                       OR EXISTS (
                           SELECT 1 FROM screenshot_favorite_tags sft
                           JOIN tags t ON t.id = sft.tag_id
                           WHERE sft.favorite_id = sf.id AND lower(t.name) LIKE ?3
                       )
                 ),
                 combined AS (
                    SELECT * FROM regular
                    UNION ALL
                    SELECT * FROM screenshots
                 )
                 SELECT id, kind, source_offset
                 FROM combined
                 ORDER BY created_at DESC, source_order, id DESC
                 LIMIT ?4 OFFSET ?5",
            )?;
            let items = statement
                .query_map(
                    params![
                        search,
                        pattern,
                        lowercase_pattern,
                        query.limit as i64,
                        query.offset as i64,
                    ],
                    |row| {
                        let kind = match row.get::<_, String>(1)?.as_str() {
                            "translation" => LibraryIndexKind::Translation,
                            "ocr" => LibraryIndexKind::Ocr,
                            _ => LibraryIndexKind::Screenshot,
                        };
                        Ok(LibraryIndexItem {
                            id: row.get(0)?,
                            kind,
                            source_offset: row.get::<_, i64>(2)? as usize,
                        })
                    },
                )?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(LibraryIndexPage {
                items,
                total: (regular_total + screenshot_total) as usize,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use rusqlite::params;

    use crate::application::library_index::{
        LibraryIndexKind, LibraryIndexQuery, LibraryIndexRepository,
    };

    use super::{Database, SqliteLibraryIndexRepository};

    #[tokio::test]
    async fn favorite_index_paginates_before_loading_content() {
        let database = Arc::new(Database::in_memory().unwrap());
        database
            .with_connection(|connection| {
                connection.execute(
                    "INSERT INTO favorites (kind, created_at, fingerprint, content_json)
                     VALUES ('translation', ?1, 'regular', '{}')",
                    ["2026-07-15T08:00:00Z"],
                )?;
                connection.execute(
                    "INSERT INTO screenshot_favorites
                     (created_at, asset_path, thumbnail_path, width, height)
                     VALUES (?1, 'one.png', 'one-thumb.png', 1, 1)",
                    ["2026-07-15T09:00:00Z"],
                )?;
                connection.execute(
                    "INSERT INTO screenshot_favorites
                     (created_at, asset_path, thumbnail_path, width, height)
                     VALUES (?1, 'two.png', 'two-thumb.png', 1, 1)",
                    ["2026-07-15T07:00:00Z"],
                )?;
                Ok(())
            })
            .unwrap();
        let repository = SqliteLibraryIndexRepository::new(database);

        let page = repository
            .query_favorites(&LibraryIndexQuery {
                search: String::new(),
                limit: 1,
                offset: 1,
            })
            .await
            .unwrap();

        assert_eq!(page.total, 3);
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].kind, LibraryIndexKind::Translation);
        assert_eq!(page.items[0].source_offset, 0);
    }

    #[tokio::test]
    async fn history_index_reports_offsets_within_each_source() {
        let database = Arc::new(Database::in_memory().unwrap());
        database
            .with_connection(|connection| {
                for (kind, timestamp) in [
                    ("translation", "2026-07-15T09:00:00Z"),
                    ("ocr", "2026-07-15T08:00:00Z"),
                    ("translation", "2026-07-15T07:00:00Z"),
                ] {
                    connection.execute(
                        "INSERT INTO history_records (kind, timestamp) VALUES (?1, ?2)",
                        params![kind, timestamp],
                    )?;
                    let id = connection.last_insert_rowid();
                    if kind == "translation" {
                        connection.execute(
                            "INSERT INTO translation_history
                             (history_id, source_text, source_lang, target_lang, providers_used, results, duration_ms)
                             VALUES (?1, 'text', 'en', 'zh-CN', '[]', '[]', 1)",
                            [id],
                        )?;
                    } else {
                        connection.execute(
                            "INSERT INTO ocr_history
                             (history_id, image_hash, provider_used, recognized_text, duration_ms)
                             VALUES (?1, 'hash', 'system', 'text', 1)",
                            [id],
                        )?;
                    }
                }
                Ok(())
            })
            .unwrap();
        let repository = SqliteLibraryIndexRepository::new(database);

        let page = repository
            .query_history(&LibraryIndexQuery {
                search: String::new(),
                limit: 2,
                offset: 1,
            })
            .await
            .unwrap();

        assert_eq!(page.total, 3);
        assert_eq!(page.items[0].kind, LibraryIndexKind::Ocr);
        assert_eq!(page.items[0].source_offset, 0);
        assert_eq!(page.items[1].kind, LibraryIndexKind::Translation);
        assert_eq!(page.items[1].source_offset, 1);
    }
}
