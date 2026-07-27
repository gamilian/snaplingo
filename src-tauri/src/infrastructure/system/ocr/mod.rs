#[cfg(target_os = "macos")]
mod macos;
mod tesseract;
#[cfg(target_os = "windows")]
mod windows;

use std::sync::Arc;

#[cfg(all(test, target_os = "macos"))]
pub(crate) use macos::vision_languages_for_request;
#[cfg(target_os = "macos")]
pub(crate) use macos::MacOSVisionOcrEngine;
#[cfg(target_os = "windows")]
pub(crate) use windows::WindowsOcrEngine;

pub(crate) fn get_tesseract_engine() -> Arc<dyn crate::application::providers::ocr::TesseractEngine>
{
    Arc::new(tesseract::SystemTesseractEngine::new())
}
