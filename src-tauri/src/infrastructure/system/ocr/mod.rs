#[cfg(target_os = "macos")]
mod macos;
mod tesseract;

use std::sync::Arc;

#[cfg(all(test, target_os = "macos"))]
pub(crate) use macos::vision_languages_for_request;
#[cfg(target_os = "macos")]
pub(crate) use macos::MacOSVisionOcrEngine;

pub(crate) fn get_tesseract_engine() -> Arc<dyn crate::application::providers::ocr::TesseractEngine>
{
    Arc::new(tesseract::SystemTesseractEngine::new())
}
