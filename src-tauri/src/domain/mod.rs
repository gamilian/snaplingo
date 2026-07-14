pub mod capture;
pub mod config;
pub mod events;
pub mod hotkey_config;
pub mod ocr;
pub mod selection;
pub mod translation;

pub use capture::{CaptureConfig, CaptureMode, CaptureRegion, ImageFormat};
pub use config::{
    GeneralSettings, HistorySettings, ScreenshotSettings, SettingsSnapshot, TranslationSettings,
};
pub use events::DomainEvent;
pub use hotkey_config::HotkeySettingsSnapshot;
pub use ocr::{OcrRequest, OcrResult};
pub use selection::{
    FrontmostApp, MethodAvailability, SelectedTextSnapshot, SelectionAttempt,
    SelectionAttemptStatus, SelectionContext, SelectionMethodKind, SelectionSource,
};
pub use translation::{TranslationRequest, TranslationResult};
