mod configuration;
mod coordinator;
mod file_input;
pub mod impls;
mod paddle_engine;
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
pub use file_input::{FileOcr, ImageFileReader};
pub use paddle_engine::PaddleOcrEngine;
pub use paddle_engine::{OcrAccelerationStatus, OcrExecutionDevice};
pub use system_engine::SystemOcrEngine;
#[cfg(any(
    target_os = "linux",
    all(target_os = "macos", feature = "tesseract-ocr")
))]
pub use tesseract_engine::TesseractEngine;
pub use trait_def::OcrProvider;
