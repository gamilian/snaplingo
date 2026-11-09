mod coordinates;
mod editor;
mod editor_geometry;
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
mod selection;
mod session;
mod source;
pub use scrolling::{ScrollingCaptureGeometry, ScrollingCaptureHost, ScrollingWindowOptions};

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
pub use editor::{AnnotationTool, CaptureEditor};
pub use editor_geometry::AnnotationHandle;
pub use editor_toolbar::{CaptureEditorControlAction, CaptureEditorStyleChange};
pub use editor_toolbar::{CaptureEditorStylePresentation, CaptureEditorToolbarPresentation};
pub use image_composer::CaptureImageComposer;
pub use image_composer::{AnnotationFont, AnnotationFontProvider};
pub use image_composer::{ImageAnnotation, PngPlacement};
pub use interface_scale::interface_scale;
pub use ocr_completion::{CaptureOcrStatus, CaptureOcrTarget};
pub use ocr_delivery::{CaptureOcrDelivery, CaptureOcrEffects};
pub use output::{configured_capture_save_dir, CaptureOutputHost, CaptureOutputSystemPaths};
pub use output::{CaptureOutput, ClipboardCaptureOutput};
pub use render::CaptureSessionOutput;
pub use runtime::CaptureSessionRuntime;
pub use runtime_host::CaptureCursorMover;
pub use runtime_host::{CapturePinOutput, CaptureSessionRuntimeHost};
pub use selection::{
    contains, merge_selection_history, remember_selection, restore_selection, CaptureSelectionMode,
    Selection,
};
pub use session::CaptureSession;
pub use session::CaptureSessions;
pub use source::CaptureSessionSource;
