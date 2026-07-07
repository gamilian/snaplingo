pub mod capture;
pub mod config;
pub mod events;
pub mod hotkey;
pub mod ocr;
pub mod selection;
pub mod translation;

pub use capture::{CaptureConfig, CaptureMode, CaptureRegion, ImageFormat};
pub use config::{
    GeneralSettings, ScreenshotSettings, SettingsSnapshot, TranslationSettings,
};
pub use events::DomainEvent;
pub use hotkey::HotkeyAction;
pub use ocr::{OcrRequest, OcrResult};
pub use selection::{
    FrontmostApp, MethodAvailability, SelectedTextSnapshot, SelectionAttempt,
    SelectionAttemptStatus, SelectionContext, SelectionMethodKind, SelectionSource,
};
pub use translation::{TranslationRequest, TranslationResult};
