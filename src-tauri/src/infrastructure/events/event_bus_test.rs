#[cfg(test)]
mod tests {
    use super::super::{EventBus, EventSubscriber};
    use crate::domain::events::DomainEvent;
    use crate::domain::translation::{TranslationRequest, TranslationResult};
    use crate::domain::ocr::{OcrRequest, OcrResult};
    use async_trait::async_trait;
    use chrono::Utc;
    use std::sync::Arc;
    use tokio::sync::Mutex as TokioMutex;

    // Mock subscriber for testing
    struct MockSubscriber {
        events_received: Arc<TokioMutex<Vec<DomainEvent>>>,
    }

    impl MockSubscriber {
        fn new() -> Self {
            Self {
                events_received: Arc::new(TokioMutex::new(Vec::new())),
            }
        }

        async fn event_count(&self) -> usize {
            self.events_received.lock().await.len()
        }

        async fn get_events(&self) -> Vec<DomainEvent> {
            self.events_received.lock().await.clone()
        }
    }

    #[async_trait]
    impl EventSubscriber for MockSubscriber {
        async fn handle(&self, event: &DomainEvent) {
            self.events_received.lock().await.push(event.clone());
        }

        fn name(&self) -> &str {
            "mock_subscriber"
        }
    }

    #[tokio::test]
    async fn test_event_bus_can_subscribe_and_publish() {
        // Arrange
        let bus = Arc::new(EventBus::new());
        let subscriber = Arc::new(MockSubscriber::new());

        // Act
        bus.subscribe(subscriber.clone()).await;

        let event = DomainEvent::TranslationCompleted {
            request: TranslationRequest {
                text: "Hello".to_string(),
                source_lang: "en".to_string(),
                target_lang: "es".to_string(),
            },
            results: vec![TranslationResult {
                provider_id: None,
                translated_text: "Hola".to_string(),
                detected_language: Some("en".to_string()),
                confidence: Some(1.0),
            }],
            providers_used: vec!["google".to_string()],
            timestamp: Utc::now(),
            duration_ms: 100,
        };

        bus.publish(event.clone());

        // Wait for async processing
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Assert
        assert_eq!(subscriber.event_count().await, 1);
        let received = subscriber.get_events().await;
        assert_eq!(received[0].event_type(), "translation_completed");
    }

    #[tokio::test]
    async fn test_multiple_subscribers_all_receive_event() {
        // Arrange
        let bus = Arc::new(EventBus::new());
        let subscriber1 = Arc::new(MockSubscriber::new());
        let subscriber2 = Arc::new(MockSubscriber::new());
        let subscriber3 = Arc::new(MockSubscriber::new());

        bus.subscribe(subscriber1.clone()).await;
        bus.subscribe(subscriber2.clone()).await;
        bus.subscribe(subscriber3.clone()).await;

        // Act
        let event = DomainEvent::OcrCompleted {
            request: OcrRequest {
                image_data: vec![1, 2, 3],
                language: None,
            },
            result: OcrResult {
                text: "Test".to_string(),
                confidence: Some(0.95),
            },
            provider_used: "tesseract".to_string(),
            timestamp: Utc::now(),
            duration_ms: 50,
        };

        bus.publish(event.clone());

        // Wait for async processing
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Assert - all subscribers received the event
        assert_eq!(subscriber1.event_count().await, 1);
        assert_eq!(subscriber2.event_count().await, 1);
        assert_eq!(subscriber3.event_count().await, 1);

        assert_eq!(subscriber1.get_events().await[0].event_type(), "ocr_completed");
        assert_eq!(subscriber2.get_events().await[0].event_type(), "ocr_completed");
        assert_eq!(subscriber3.get_events().await[0].event_type(), "ocr_completed");
    }
}
