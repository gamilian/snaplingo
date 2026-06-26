#[cfg(test)]
mod tests {
    use super::super::HistoryDatabase;
    use crate::domain::translation::{TranslationRequest, TranslationResult};
    use chrono::Utc;
    use tempfile::NamedTempFile;

    fn create_temp_db() -> HistoryDatabase {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();
        // Don't drop temp_file - we want the path to persist for the test
        std::mem::forget(temp_file);
        HistoryDatabase::new(path).unwrap()
    }

    #[tokio::test]
    async fn test_create_database_with_schema() {
        // Act
        let db = create_temp_db();

        // Assert - database should be created without errors
        // We'll verify schema by trying to insert data in the next test
        drop(db);
    }

    #[tokio::test]
    async fn test_insert_translation_history() {
        // Arrange
        let db = create_temp_db();
        let request = TranslationRequest {
            text: "Hello world".to_string(),
            source_lang: "en".to_string(),
            target_lang: "es".to_string(),
        };
        let results = vec![TranslationResult {
            provider_id: "google".to_string(),
            translated_text: "Hola mundo".to_string(),
            detected_language: Some("en".to_string()),
            confidence: Some(0.95),
        }];
        let providers_used = vec!["google".to_string()];
        let timestamp = Utc::now();

        // Act
        let result = db
            .insert_translation(&request, &results, &providers_used, timestamp, 150)
            .await;

        // Assert
        assert!(result.is_ok());
    }
}
