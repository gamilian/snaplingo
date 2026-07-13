#[cfg(test)]
mod tests {
    use crate::application::history::{
        EventSubscriber, History, HistoryChangeNotifier, HistoryRepository,
    };
    use crate::domain::events::DomainEvent;
    use crate::domain::ocr::{OcrRequest, OcrResult};
    use crate::domain::translation::{TranslationRequest, TranslationResult};
    use crate::infrastructure::storage::SqliteHistoryRepository;
    use chrono::Utc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct CountingNotifier(AtomicUsize);

    impl HistoryChangeNotifier for CountingNotifier {
        fn history_changed(&self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn create_temp_db() -> Arc<dyn HistoryRepository> {
        Arc::new(SqliteHistoryRepository::new_in_memory().unwrap())
    }

    #[tokio::test]
    async fn test_history_handles_translation_completed_event() {
        // Arrange
        let db = create_temp_db();
        let history = History::new(db.clone());

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
        history.handle(&event).await;

        // Assert - history should be recorded
        let history = db.query_translations(10, 0).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].source_text, "Hello world");
        assert_eq!(history[0].target_lang, "es");
    }

    #[tokio::test]
    async fn successful_event_recording_notifies_runtime_observers() {
        let db = create_temp_db();
        let notifier = Arc::new(CountingNotifier(AtomicUsize::new(0)));
        let history = History::with_change_notifier(db, notifier.clone());
        let event = DomainEvent::TranslationCompleted {
            request: TranslationRequest {
                text: "Hello".to_string(),
                source_lang: "en".to_string(),
                target_lang: "zh-CN".to_string(),
            },
            results: vec![],
            providers_used: vec![],
            timestamp: Utc::now(),
            duration_ms: 5,
        };

        history.handle(&event).await;

        assert_eq!(notifier.0.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_history_handles_ocr_completed_event() {
        // Arrange
        let db = create_temp_db();
        let history = History::new(db.clone());

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
        history.handle(&event).await;

        // Assert - history should be recorded
        let history = db.query_ocr(10, 0).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].recognized_text, "Recognized text");
        assert_eq!(history[0].provider_used, "tesseract");
    }

    #[tokio::test]
    async fn test_history_query_apis() {
        // Arrange
        let db = create_temp_db();
        let history = History::new(db.clone());

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

        history.handle(&translation_event).await;

        // Act - Query via History APIs
        let translations = history.get_translation_history(10, 0).await.unwrap();
        let search_results = history.search_history("Test").await.unwrap();

        // Assert
        assert_eq!(translations.len(), 1);
        assert_eq!(translations[0].source_text, "Test");

        assert_eq!(search_results.len(), 1);
    }

    #[tokio::test]
    async fn test_history_delete_apis() {
        // Arrange
        let db = create_temp_db();
        let history = History::new(db.clone());

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

        history.handle(&event).await;

        let entries = history.get_translation_history(10, 0).await.unwrap();
        assert_eq!(entries.len(), 1);
        let id = entries[0].id;

        // Act - Delete
        history.delete_history(id).await.unwrap();

        // Assert
        let entries = history.get_translation_history(10, 0).await.unwrap();
        assert_eq!(entries.len(), 0);
    }

    #[tokio::test]
    async fn successful_history_mutations_notify_runtime_observers() {
        let db = create_temp_db();
        let notifier = Arc::new(CountingNotifier(AtomicUsize::new(0)));
        let history = History::with_change_notifier(db, notifier.clone());
        let event = DomainEvent::TranslationCompleted {
            request: TranslationRequest {
                text: "Mutable".to_string(),
                source_lang: "en".to_string(),
                target_lang: "zh-CN".to_string(),
            },
            results: vec![],
            providers_used: vec![],
            timestamp: Utc::now(),
            duration_ms: 1,
        };
        history.handle(&event).await;
        let id = history.get_translation_history(1, 0).await.unwrap()[0].id;
        notifier.0.store(0, Ordering::SeqCst);

        history.set_history_favorite(id, true).await.unwrap();
        history
            .update_history_note(id, Some("note".to_string()))
            .await
            .unwrap();
        history
            .replace_history_tags(id, vec!["tag".to_string()])
            .await
            .unwrap();
        history.delete_history(id).await.unwrap();
        history.clear_all_history().await.unwrap();

        assert_eq!(notifier.0.load(Ordering::SeqCst), 5);
    }
}
