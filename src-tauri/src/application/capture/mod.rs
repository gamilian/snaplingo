mod coordinates;
mod editor_handoff;
mod editor_toolbar;
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
#[cfg(any(target_os = "macos", test))]
mod selection;
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
pub use editor_handoff::CaptureEditorSelection;
#[cfg(target_os = "macos")]
pub(crate) use editor_toolbar::CaptureEditorStyleChange;
pub use editor_toolbar::{CaptureEditorStylePresentation, CaptureEditorToolbarPresentation};
pub use image_composer::CaptureImageComposer;
pub(crate) use image_composer::{AnnotationFont, AnnotationFontProvider};
pub(crate) use image_composer::{ImageAnnotation, PngPlacement};
#[cfg(target_os = "macos")]
pub(crate) use interface_scale::interface_scale;
pub(crate) use ocr_completion::{CaptureOcrStatus, CaptureOcrTarget};
pub(crate) use ocr_delivery::{CaptureOcrDelivery, CaptureOcrEffects};
pub(crate) use output::{configured_capture_save_dir, CaptureOutputHost, CaptureOutputSystemPaths};
pub use output::{CaptureOutput, ClipboardCaptureOutput};
pub use render::CaptureSessionOutput;
pub use runtime::CaptureSessionRuntime;
pub use runtime_host::CaptureCursorMover;
pub(crate) use runtime_host::{CapturePinOutput, CaptureSessionRuntimeHost};
#[cfg(target_os = "macos")]
pub(crate) use selection::{
    contains, merge_selection_history, remember_selection, restore_selection, CaptureSelectionMode,
    Selection,
};
#[cfg(target_os = "macos")]
pub(crate) use session::CaptureSession;
pub use session::CaptureSessions;
pub use source::CaptureSessionSource;
