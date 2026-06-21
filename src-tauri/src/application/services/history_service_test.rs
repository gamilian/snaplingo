#[cfg(test)]
mod tests {
    use super::super::HistoryService;
    use crate::domain::events::DomainEvent;
    use crate::domain::ocr::{OcrRequest, OcrResult};
    use crate::domain::translation::{TranslationRequest, TranslationResult};
    use crate::infrastructure::events::EventSubscriber;
    use crate::infrastructure::storage::HistoryDatabase;
    use chrono::Utc;
    use std::sync::Arc;
    use tempfile::NamedTempFile;

    fn create_temp_db() -> Arc<HistoryDatabase> {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();
        std::mem::forget(temp_file);
        Arc::new(HistoryDatabase::new(path).unwrap())
    }

    #[tokio::test]
    async fn test_history_service_handles_translation_completed_event() {
        // Arrange
        let db = create_temp_db();
        let service = HistoryService::new(db.clone());

        let event = DomainEvent::TranslationCompleted {
            request: TranslationRequest {
                text: "Hello world".to_string(),
                source_lang: "en".to_string(),
                target_lang: "es".to_string(),
            },
            results: vec![TranslationResult {
                provider_id: "google".to_string(),
                translated_text: "Hola mundo".to_string(),
                detected_language: Some("en".to_string()),
                confidence: Some(0.95),
            }],
            providers_used: vec!["google".to_string()],
            timestamp: Utc::now(),
            duration_ms: 150,
        };

        // Act
        service.handle(&event).await;

        // Assert - history should be recorded
        let history = db.query_translations(10, 0).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].source_text, "Hello world");
        assert_eq!(history[0].target_lang, "es");
    }

    #[tokio::test]
    async fn test_history_service_handles_ocr_completed_event() {
        // Arrange
        let db = create_temp_db();
        let service = HistoryService::new(db.clone());

        let event = DomainEvent::OcrCompleted {
            request: OcrRequest {
                image_data: vec![1, 2, 3, 4, 5],
                language: None,
            },
            result: OcrResult {
                text: "Recognized text".to_string(),
                confidence: Some(0.92),
            },
            provider_used: "tesseract".to_string(),
            timestamp: Utc::now(),
            duration_ms: 200,
        };

        // Act
        service.handle(&event).await;

        // Assert - history should be recorded
        let history = db.query_ocr(10, 0).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].recognized_text, "Recognized text");
        assert_eq!(history[0].provider_used, "tesseract");
    }

    #[tokio::test]
    async fn test_history_service_query_apis() {
        // Arrange
        let db = create_temp_db();
        let service = HistoryService::new(db.clone());

        // Insert some test data via events
        let translation_event = DomainEvent::TranslationCompleted {
            request: TranslationRequest {
                text: "Test".to_string(),
                source_lang: "en".to_string(),
                target_lang: "fr".to_string(),
            },
            results: vec![TranslationResult {
                provider_id: "deepl".to_string(),
                translated_text: "Tester".to_string(),
                detected_language: Some("en".to_string()),
                confidence: Some(0.98),
            }],
            providers_used: vec!["deepl".to_string()],
            timestamp: Utc::now(),
            duration_ms: 100,
        };

        service.handle(&translation_event).await;

        // Act - Query via HistoryService APIs
        let translations = service.get_translation_history(10, 0).await.unwrap();
        let search_results = service.search_history("Test").await.unwrap();

        // Assert
        assert_eq!(translations.len(), 1);
        assert_eq!(translations[0].source_text, "Test");

        assert_eq!(search_results.len(), 1);
    }

    #[tokio::test]
    async fn test_history_service_delete_apis() {
        // Arrange
        let db = create_temp_db();
        let service = HistoryService::new(db.clone());

        // Insert data
        let event = DomainEvent::TranslationCompleted {
            request: TranslationRequest {
                text: "Delete me".to_string(),
                source_lang: "en".to_string(),
                target_lang: "es".to_string(),
            },
            results: vec![TranslationResult {
                provider_id: "google".to_string(),
                translated_text: "Bórrame".to_string(),
                detected_language: None,
                confidence: None,
            }],
            providers_used: vec!["google".to_string()],
            timestamp: Utc::now(),
            duration_ms: 50,
        };

        service.handle(&event).await;

        let history = service.get_translation_history(10, 0).await.unwrap();
        assert_eq!(history.len(), 1);
        let id = history[0].id;

        // Act - Delete
        service.delete_history(id).await.unwrap();

        // Assert
        let history = service.get_translation_history(10, 0).await.unwrap();
        assert_eq!(history.len(), 0);
    }
}
