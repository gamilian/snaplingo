pub mod translation;
pub mod ocr;
pub mod capture;
pub mod config;

pub use translation::{TranslationRequest, TranslationResult};
pub use ocr::{OcrRequest, OcrResult};
pub use capture::{CaptureMode, CaptureConfig, CaptureRegion, ImageFormat};
pub use config::AppConfig;
