mod baidu_ocr;
mod system_ocr;
#[cfg(any(
    target_os = "linux",
    all(target_os = "macos", feature = "tesseract-ocr")
))]
mod tesseract;

pub use baidu_ocr::BaiduOcrProvider;
pub use system_ocr::SystemOcrProvider;
#[cfg(any(
    target_os = "linux",
    all(target_os = "macos", feature = "tesseract-ocr")
))]
pub use tesseract::TesseractProvider;
