use crate::domain::events::DomainEvent;
use async_trait::async_trait;

#[async_trait]
pub trait EventSubscriber: Send + Sync {
    async fn handle(&self, event: &DomainEvent);

    fn name(&self) -> &str {
        "unnamed_subscriber"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::events::DomainEvent;
    use crate::domain::translation::{TranslationRequest, TranslationResult};
    use chrono::Utc;

    struct FakeSubscriber;

    #[async_trait]
    impl EventSubscriber for FakeSubscriber {
        async fn handle(&self, _event: &DomainEvent) {}

        fn name(&self) -> &str {
            "fake"
        }
    }

    #[tokio::test]
    async fn fake_event_subscriber_can_handle_domain_events() {
        let subscriber = FakeSubscriber;
        let event = DomainEvent::TranslationCompleted {
            request: TranslationRequest {
                text: "hello".to_string(),
                source_lang: "en".to_string(),
                target_lang: "fr".to_string(),
            },
            results: vec![TranslationResult {
                provider_id: "fake".to_string(),
                translated_text: "bonjour".to_string(),
                detected_language: None,
                confidence: None,
            }],
            providers_used: vec!["fake".to_string()],
            timestamp: Utc::now(),
            duration_ms: 1,
        };

        subscriber.handle(&event).await;

        assert_eq!(subscriber.name(), "fake");
    }
}
