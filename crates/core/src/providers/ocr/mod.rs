mod configuration;
mod coordinator;
mod file_input;
pub mod model;
mod paddle;
mod paddle_engine;
mod system;
mod system_engine;
mod tesseract;
mod tesseract_engine;
mod trait_def;

#[cfg(test)]
mod coordinator_test;

pub use configuration::OcrProviderConfiguration;
pub use coordinator::OcrCoordinator;
pub use file_input::{FileOcr, ImageFileReader};
pub use paddle::PaddleOcrProvider;
pub use paddle_engine::PaddleOcrEngine;
pub use paddle_engine::{OcrAccelerationStatus, OcrExecutionDevice};
pub use system::SystemOcrProvider;
pub use system_engine::SystemOcrEngine;
pub use tesseract::TesseractProvider;
pub use tesseract_engine::TesseractEngine;
pub use trait_def::OcrProvider;
