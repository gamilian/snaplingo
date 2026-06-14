pub mod translation;
pub mod ocr;
pub mod capture;
pub mod config;
pub mod events;
pub mod hotkey;

pub use translation::{TranslationRequest, TranslationResult};
pub use ocr::{OcrRequest, OcrResult};
pub use capture::{CaptureMode, CaptureConfig, CaptureRegion, ImageFormat};
pub use config::AppConfig;
pub use events::DomainEvent;
pub use hotkey::HotkeyAction;
