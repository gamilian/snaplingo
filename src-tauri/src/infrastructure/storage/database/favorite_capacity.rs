use std::sync::Arc;

use async_trait::async_trait;

use crate::application::favorite_capacity::FavoriteCapacityRepository;
use crate::Result;

use super::Database;

pub struct SqliteFavoriteCapacityRepository {
    database: Arc<Database>,
}

impl SqliteFavoriteCapacityRepository {
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

#[async_trait]
impl FavoriteCapacityRepository for SqliteFavoriteCapacityRepository {
    async fn current_count(&self) -> Result<usize> {
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::application::favorite_capacity::FavoriteCapacityRepository;

    use super::{Database, SqliteFavoriteCapacityRepository};

    #[tokio::test]
    async fn counts_regular_and_screenshot_favorites_through_one_adapter() {
        let database = Arc::new(Database::in_memory().unwrap());
        database
            .with_connection(|connection| {
                connection.execute(
                    "INSERT INTO favorites
                     (kind, created_at, fingerprint, content_json)
                     VALUES ('translation', '2026-07-15T00:00:00Z', 'one', '{}')",
                    [],
                )?;
                connection.execute(
                    "INSERT INTO screenshot_favorites
                     (created_at, asset_path, thumbnail_path, width, height)
                     VALUES ('2026-07-15T00:00:00Z', 'one.png', 'one-thumb.png', 1, 1)",
                    [],
                )?;
                Ok(())
            })
            .unwrap();
        let repository = SqliteFavoriteCapacityRepository::new(database);

        assert_eq!(repository.current_count().await.unwrap(), 2);
    }
}
