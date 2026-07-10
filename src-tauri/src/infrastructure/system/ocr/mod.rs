#[cfg(target_os = "macos")]
mod backend;
#[cfg(target_os = "macos")]
mod macos;
mod tesseract;

use std::sync::Arc;

#[cfg(target_os = "macos")]
pub use backend::SystemOcrEngine;
#[cfg(all(test, target_os = "macos"))]
pub(crate) use macos::vision_languages_for_request;

#[cfg(target_os = "macos")]
pub fn get_system_ocr_engine() -> Arc<dyn SystemOcrEngine> {
    Arc::new(macos::MacOSVisionOcrEngine::new())
}

pub(crate) fn get_tesseract_engine() -> Arc<dyn crate::application::providers::ocr::TesseractEngine>
{
    Arc::new(tesseract::SystemTesseractEngine::new())
}
