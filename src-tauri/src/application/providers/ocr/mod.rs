mod configuration;
mod coordinator;
pub mod impls;
mod system_engine;
mod tesseract_engine;
mod trait_def;

#[cfg(test)]
mod coordinator_test;

pub use configuration::OcrProviderConfiguration;
pub use coordinator::OcrCoordinator;
pub(crate) use system_engine::SystemOcrEngine;
pub(crate) use tesseract_engine::TesseractEngine;
pub use trait_def::OcrProvider;
