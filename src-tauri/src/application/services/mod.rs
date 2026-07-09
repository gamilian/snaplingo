pub mod capture_output_service;
pub mod capture_service;
mod capture_session_render;
pub mod capture_session_runtime;
pub mod capture_session_service;
pub mod history_service;
pub mod image_composition_service;
pub mod pinned_image_service;
pub mod selected_text_acquirer;

#[cfg(test)]
mod capture_output_service_test;
#[cfg(test)]
mod capture_session_service_test;
#[cfg(test)]
mod history_service_test;
#[cfg(test)]
mod image_composition_service_test;
#[cfg(test)]
mod pinned_image_service_test;

pub use capture_output_service::{CaptureOutputService, ClipboardCaptureOutput};
pub use capture_service::CaptureService;
pub use capture_session_render::CaptureSessionOutput;
pub use capture_session_runtime::CaptureSessionRuntime;
pub use capture_session_service::CaptureSessionService;
pub use history_service::HistoryService;
pub use image_composition_service::ImageCompositionService;
pub use pinned_image_service::{
    PinnedImageGroupMembership, PinnedImageGroupRemoval, PinnedImageGroupSwitch,
    PinnedImageOpenRequest, PinnedImageService,
};
pub use selected_text_acquirer::{SelectedTextAcquirer, SelectionScheme};
