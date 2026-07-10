mod image_composer;
mod output;
mod render;
mod runtime;
mod session;
mod source;

#[cfg(test)]
mod image_composer_test;
#[cfg(test)]
mod output_test;
#[cfg(test)]
mod session_test;

pub use image_composer::CaptureImageComposer;
pub(crate) use image_composer::{ImageAnnotation, PngPlacement};
pub(crate) use output::configured_capture_save_dir;
pub use output::{CaptureOutput, ClipboardCaptureOutput};
pub use render::CaptureSessionOutput;
pub use runtime::CaptureSessionRuntime;
pub(crate) use runtime::TauriCaptureSessionRuntimeHost;
pub use session::CaptureSessions;
pub use source::CaptureSessionSource;
