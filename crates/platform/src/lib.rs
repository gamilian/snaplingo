//! Native system capabilities implementing the portable Core contracts.
//! Application composition chooses adapters; this crate has no application shell,
//! window-host, UI-model or database dependencies.
pub mod annotation_fonts;
pub mod capture_cursor;
pub mod capture_output;
pub mod clipboard_text;
pub mod color_dialog;
pub mod hotkeys;
pub mod ocr;
pub mod paths;
pub mod required_permissions;
pub mod screenshot;
pub mod screenshot_favorites;
pub mod selection;
pub mod single_instance;
pub mod start_on_boot;
pub mod tts;

pub use snaplingo_core::{AppError, Result};
