mod baidu_ocr;
#[cfg(target_os = "macos")]
mod system_ocr;
mod tesseract;

pub use baidu_ocr::BaiduOcrProvider;
#[cfg(target_os = "macos")]
pub use system_ocr::SystemOcrProvider;
pub use tesseract::TesseractProvider;
