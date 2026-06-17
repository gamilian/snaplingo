pub mod capture_service;
pub mod capture_output_service;
pub mod capture_session_service;
pub mod history_service;
pub mod hotkey_service;
pub mod image_composition_service;
pub mod pinned_image_service;
pub mod workflow_service;

#[cfg(test)]
mod capture_session_service_test;
#[cfg(test)]
mod capture_output_service_test;
#[cfg(test)]
mod history_service_test;
#[cfg(test)]
mod image_composition_service_test;
#[cfg(test)]
mod pinned_image_service_test;

pub use capture_service::CaptureService;
pub use capture_output_service::CaptureOutputService;
pub use capture_session_service::CaptureSessionService;
pub use history_service::HistoryService;
pub use hotkey_service::HotkeyService;
pub use image_composition_service::ImageCompositionService;
pub use pinned_image_service::{
    PinnedImageGroupMembership, PinnedImageGroupRemoval, PinnedImageGroupSwitch,
    PinnedImageService,
};
pub use workflow_service::{WorkflowOutcome, WorkflowService};
