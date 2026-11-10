mod coordinates;
mod image_composer;
mod image_effects;
mod interface_scale;
mod ocr_completion;
mod ocr_delivery;
mod output;
mod render;
mod runtime;
mod runtime_host;
mod scrolling;
mod scrolling_image;
mod session;
mod source;
pub(crate) use scrolling::{
    ScrollingCaptureGeometry, ScrollingCaptureHost, ScrollingWindowOptions,
};

#[cfg(test)]
mod image_composer_test;
#[cfg(test)]
mod image_effects_test;
#[cfg(test)]
mod output_test;
#[cfg(test)]
mod session_allocations_test;
#[cfg(test)]
mod session_test;

pub use coordinates::{CaptureCoordinatePolicy, CaptureWindowGeometry};
pub use image_composer::CaptureImageComposer;
pub(crate) use image_composer::{AnnotationFont, AnnotationFontProvider};
pub(crate) use image_composer::{ImageAnnotation, PngPlacement};
pub(crate) use ocr_completion::{CaptureOcrStatus, CaptureOcrTarget};
pub(crate) use ocr_delivery::{CaptureOcrDelivery, CaptureOcrEffects};
pub(crate) use output::{configured_capture_save_dir, CaptureOutputHost, CaptureOutputSystemPaths};
pub use output::{CaptureOutput, ClipboardCaptureOutput};
pub use render::CaptureSessionOutput;
pub use runtime::CaptureSessionRuntime;
pub use runtime_host::CaptureCursorMover;
pub(crate) use runtime_host::{CapturePinOutput, CaptureSessionRuntimeHost};
pub use session::CaptureSessions;
pub use source::CaptureSessionSource;
