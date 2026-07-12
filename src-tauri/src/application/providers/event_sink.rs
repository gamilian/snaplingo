use crate::domain::events::DomainEvent;

pub trait ProviderEventSink: Send + Sync {
    fn publish(&self, event: DomainEvent);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::translation::{TranslationRequest, TranslationResult};
    use chrono::Utc;
    use std::sync::Mutex;

    struct FakeProviderEventSink {
        published: Mutex<Vec<String>>,
    }

    impl ProviderEventSink for FakeProviderEventSink {
        fn publish(&self, event: DomainEvent) {
            self.published
                .lock()
                .unwrap()
                .push(event.event_type().to_string());
        }
    }

    #[test]
    fn fake_provider_event_sink_records_published_event() {
        let sink = FakeProviderEventSink {
            published: Mutex::new(Vec::new()),
        };

        sink.publish(DomainEvent::TranslationCompleted {
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
        });

        assert_eq!(
            sink.published.lock().unwrap().as_slice(),
            ["translation_completed"]
        );
    }
}
