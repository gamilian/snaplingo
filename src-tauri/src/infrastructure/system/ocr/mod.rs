#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(target_os = "windows"))]
mod tesseract;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(all(test, target_os = "macos"))]
pub(crate) use macos::vision_languages_for_request;
#[cfg(target_os = "macos")]
pub(crate) use macos::MacOSVisionOcrEngine;
#[cfg(target_os = "windows")]
pub(crate) use windows::WindowsOcrEngine;

#[cfg(not(target_os = "windows"))]
pub(crate) fn get_tesseract_engine(
) -> std::sync::Arc<dyn crate::application::providers::ocr::TesseractEngine> {
    std::sync::Arc::new(tesseract::SystemTesseractEngine::new())
}
