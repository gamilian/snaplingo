use crate::domain::translation::{TranslationRequest, TranslationResult};
use crate::domain::ocr::{OcrRequest, OcrResult};
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

/// Domain events representing significant business actions.
///
/// These events are published by Coordinators when operations complete,
/// allowing decoupled components (like HistoryService) to react.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DomainEvent {
    /// A translation operation completed successfully
    TranslationCompleted {
        request: TranslationRequest,
        results: Vec<TranslationResult>,
        providers_used: Vec<String>,
        timestamp: DateTime<Utc>,
        duration_ms: u64,
    },
    /// An OCR operation completed successfully
    OcrCompleted {
        request: OcrRequest,
        result: OcrResult,
        provider_used: String,
        timestamp: DateTime<Utc>,
        duration_ms: u64,
    },
}

impl DomainEvent {
    /// Returns the timestamp when this event occurred
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            Self::TranslationCompleted { timestamp, .. } => *timestamp,
            Self::OcrCompleted { timestamp, .. } => *timestamp,
        }
    }

    /// Returns a string identifying the event type
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::TranslationCompleted { .. } => "translation_completed",
            Self::OcrCompleted { .. } => "ocr_completed",
        }
    }
}
