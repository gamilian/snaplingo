//! External service protocols and optional local OCR inference.
//! Product policy and consumed OCR/translation contracts belong to Core;
//! the application supplies configuration, model resources and cache paths.
pub mod http;
pub mod llm;
pub mod ocr;
pub mod translation;

pub use snaplingo_core::{AppError, Result};
