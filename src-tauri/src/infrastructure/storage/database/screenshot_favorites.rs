use std::collections::BTreeSet;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Row};

use crate::application::screenshot_favorites::{
    NewScreenshotFavorite, ScreenshotFavoriteQuery, ScreenshotFavoriteRecord,
    ScreenshotFavoriteRepository,
};
use crate::{AppError, Result};

use super::Database;

pub struct SqliteScreenshotFavoriteRepository {
    database: Arc<Database>,
}

impl SqliteScreenshotFavoriteRepository {
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }

    fn read_record(connection: &Connection, row: &Row<'_>) -> Result<ScreenshotFavoriteRecord> {
        let id = row.get(0)?;
        let created_at = DateTime::parse_from_rfc3339(&row.get::<_, String>(1)?)
            .map_err(|error| AppError::Other(error.to_string()))?
            .with_timezone(&Utc);
        Ok(ScreenshotFavoriteRecord {
            id,
            content_kind: "screenshot".to_string(),
            created_at,
            asset_path: row.get(2)?,
            thumbnail_path: row.get(3)?,
            width: row.get::<_, i64>(4)? as u32,
            height: row.get::<_, i64>(5)? as u32,
            note: row.get(6)?,
            tags: read_tags(connection, id)?,
        })
    }
}

#[async_trait]
impl ScreenshotFavoriteRepository for SqliteScreenshotFavoriteRepository {
    async fn insert(&self, favorite: &NewScreenshotFavorite) -> Result<ScreenshotFavoriteRecord> {
        self.database.with_transaction(|transaction| {
            transaction.execute(
                "INSERT INTO screenshot_favorites
                 (created_at, asset_path, thumbnail_path, width, height)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    favorite.created_at.to_rfc3339(),
                    favorite.asset_path,
                    favorite.thumbnail_path,
                    favorite.width as i64,
                    favorite.height as i64,
                ],
            )?;
            Ok(ScreenshotFavoriteRecord {
                id: transaction.last_insert_rowid(),
                content_kind: "screenshot".to_string(),
                created_at: favorite.created_at,
                asset_path: favorite.asset_path.clone(),
                thumbnail_path: favorite.thumbnail_path.clone(),
                width: favorite.width,
                height: favorite.height,
                note: None,
                tags: vec![],
            })
        })
    }

    async fn query(
        &self,
        query: &ScreenshotFavoriteQuery,
    ) -> Result<(Vec<ScreenshotFavoriteRecord>, usize)> {
        self.database.with_connection(|connection| {
            let search = query
                .search
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| format!("%{}%", value.to_lowercase()));
            let where_clause = if search.is_some() {
                "WHERE lower(COALESCE(sf.note, '')) LIKE ?1
                   OR EXISTS (
                       SELECT 1 FROM screenshot_favorite_tags sft
                       JOIN tags t ON t.id = sft.tag_id
                       WHERE sft.favorite_id = sf.id AND lower(t.name) LIKE ?1
                   )"
            } else {
                ""
            };
            let total_sql = format!("SELECT COUNT(*) FROM screenshot_favorites sf {where_clause}");
            let total: i64 = if let Some(search) = &search {
                connection.query_row(&total_sql, [search], |row| row.get(0))?
            } else {
                connection.query_row(&total_sql, [], |row| row.get(0))?
            };
            let page_sql = format!(
                "SELECT sf.id, sf.created_at, sf.asset_path, sf.thumbnail_path,
                        sf.width, sf.height, sf.note
                 FROM screenshot_favorites sf
                 {where_clause}
                 ORDER BY sf.created_at DESC, sf.id DESC
                 LIMIT ?{} OFFSET ?{}",
                if search.is_some() { 2 } else { 1 },
                if search.is_some() { 3 } else { 2 }
            );
            let mut statement = connection.prepare(&page_sql)?;
            let mut rows = if let Some(search) = &search {
                statement.query(params![search, query.limit as i64, query.offset as i64])?
            } else {
                statement.query(params![query.limit as i64, query.offset as i64])?
            };
            let mut records = Vec::new();
            while let Some(row) = rows.next()? {
                records.push(Self::read_record(connection, row)?);
            }
            Ok((records, total as usize))
        })
    }

    async fn find(&self, id: i64) -> Result<Option<ScreenshotFavoriteRecord>> {
        self.database.with_connection(|connection| {
            let mut statement = connection.prepare(
                "SELECT id, created_at, asset_path, thumbnail_path, width, height, note
                 FROM screenshot_favorites WHERE id = ?1",
            )?;
            let mut rows = statement.query([id])?;
            match rows.next()? {
                Some(row) => Ok(Some(Self::read_record(connection, row)?)),
                None => Ok(None),
            }
        })
    }

    async fn update_metadata(
        &self,
        id: i64,
        note: Option<String>,
        tags: Vec<String>,
    ) -> Result<()> {
        let tags = tags
            .into_iter()
            .map(|tag| tag.trim().to_string())
            .filter(|tag| !tag.is_empty())
            .collect::<BTreeSet<_>>();
        self.database.with_transaction(|transaction| {
            transaction.execute(
                "UPDATE screenshot_favorites SET note = ?1 WHERE id = ?2",
                params![note.filter(|value| !value.trim().is_empty()), id],
            )?;
            transaction.execute(
                "DELETE FROM screenshot_favorite_tags WHERE favorite_id = ?1",
                [id],
            )?;
            for tag in tags {
                transaction.execute("INSERT OR IGNORE INTO tags (name) VALUES (?1)", [&tag])?;
                let tag_id: i64 = transaction.query_row(
                    "SELECT id FROM tags WHERE name = ?1",
                    [&tag],
                    |row| row.get(0),
                )?;
                transaction.execute(
                    "INSERT INTO screenshot_favorite_tags (favorite_id, tag_id) VALUES (?1, ?2)",
                    params![id, tag_id],
                )?;
            }
            remove_orphan_tags(transaction)?;
            Ok(())
        })
    }

    async fn delete(&self, id: i64) -> Result<()> {
        self.database.with_transaction(|transaction| {
            transaction.execute("DELETE FROM screenshot_favorites WHERE id = ?1", [id])?;
            remove_orphan_tags(transaction)?;
            Ok(())
        })
    }

    async fn count_all(&self) -> Result<usize> {
        self.database.with_connection(|connection| {
            connection
                .query_row(
                    "SELECT (SELECT COUNT(*) FROM favorites) +
                            (SELECT COUNT(*) FROM screenshot_favorites)",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .map(|count| count as usize)
                .map_err(Into::into)
        })
    }
}

fn read_tags(connection: &Connection, favorite_id: i64) -> Result<Vec<String>> {
    let mut statement = connection.prepare(
        "SELECT t.name FROM tags t
         JOIN screenshot_favorite_tags sft ON sft.tag_id = t.id
         WHERE sft.favorite_id = ?1 ORDER BY t.name",
    )?;
    let tags = statement
        .query_map([favorite_id], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<String>>>()?;
    Ok(tags)
}

fn remove_orphan_tags(connection: &Connection) -> Result<()> {
    connection.execute(
        "DELETE FROM tags
         WHERE NOT EXISTS (SELECT 1 FROM history_tags ht WHERE ht.tag_id = tags.id)
           AND NOT EXISTS (
               SELECT 1 FROM screenshot_favorite_tags sft WHERE sft.tag_id = tags.id
           )
           AND NOT EXISTS (SELECT 1 FROM favorite_tags ft WHERE ft.tag_id = tags.id)",
        [],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::storage::SqliteHistoryRepository;

    #[tokio::test]
    async fn persists_queries_and_normalizes_metadata() {
        let repository =
            SqliteScreenshotFavoriteRepository::new(Arc::new(Database::in_memory().unwrap()));
        let record = repository
            .insert(&NewScreenshotFavorite {
                created_at: Utc::now(),
                asset_path: "screenshots/one.png".into(),
                thumbnail_path: "thumbnails/one.png".into(),
                width: 100,
                height: 50,
            })
            .await
            .unwrap();
        repository
            .update_metadata(
                record.id,
                Some("project alpha".into()),
                vec![" work ".into(), "work".into()],
            )
            .await
            .unwrap();

        let (records, total) = repository
            .query(&ScreenshotFavoriteQuery {
                search: Some("WORK".into()),
                limit: 20,
                offset: 0,
            })
            .await
            .unwrap();
        assert_eq!(total, 1);
        assert_eq!(records[0].tags, vec!["work"]);
        assert_eq!(records[0].note.as_deref(), Some("project alpha"));
    }

    #[tokio::test]
    async fn clearing_history_preserves_screenshot_favorite_tags() {
        let database = Arc::new(Database::in_memory().unwrap());
        let repository = SqliteScreenshotFavoriteRepository::new(database.clone());
        let history = SqliteHistoryRepository::new(database);
        let record = repository
            .insert(&NewScreenshotFavorite {
                created_at: Utc::now(),
                asset_path: "screenshots/one.png".into(),
                thumbnail_path: "thumbnails/one.png".into(),
                width: 100,
                height: 50,
            })
            .await
            .unwrap();
        repository
            .update_metadata(record.id, None, vec!["work".into()])
            .await
            .unwrap();

        history.clear_all().await.unwrap();

        assert_eq!(
            repository.find(record.id).await.unwrap().unwrap().tags,
            vec!["work"]
        );
    }
}
