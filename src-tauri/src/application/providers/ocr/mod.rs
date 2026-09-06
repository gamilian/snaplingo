mod configuration;
mod coordinator;
pub mod impls;
mod system_engine;
#[cfg(any(
    target_os = "linux",
    all(target_os = "macos", feature = "tesseract-ocr")
))]
mod tesseract_engine;
mod trait_def;

#[cfg(test)]
mod coordinator_test;

pub use configuration::OcrProviderConfiguration;
pub use coordinator::OcrCoordinator;
pub(crate) use system_engine::SystemOcrEngine;
#[cfg(any(
    target_os = "linux",
    all(target_os = "macos", feature = "tesseract-ocr")
))]
pub(crate) use tesseract_engine::TesseractEngine;
pub use trait_def::OcrProvider;
