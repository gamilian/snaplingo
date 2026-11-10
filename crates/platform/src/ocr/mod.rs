mod image_file;
pub use image_file::FilesystemImageReader;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(any(
    target_os = "linux",
    all(target_os = "macos", feature = "tesseract-ocr")
))]
mod tesseract;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(any(target_os = "windows", test))]
mod windows_image;

#[cfg(target_os = "macos")]
pub use macos::MacOSVisionOcrEngine;
#[cfg(target_os = "windows")]
pub use windows::WindowsOcrEngine;

#[cfg(any(
    target_os = "linux",
    all(target_os = "macos", feature = "tesseract-ocr")
))]
pub fn get_tesseract_engine() -> std::sync::Arc<dyn snaplingo_core::providers::ocr::TesseractEngine>
{
    std::sync::Arc::new(tesseract::SystemTesseractEngine::new())
}
