//! AppKit presentation/input adapters for the shared Rust Settings canvas.
#![allow(dead_code, unused_imports)]
#[cfg(test)]
mod feature_macos;
#[cfg(test)]
mod feature_page_macos;
use crate::infrastructure::settings_canvas::general_controller;
#[cfg(test)]
mod general_macos;
#[cfg(test)]
mod general_page_macos;
#[cfg(test)]
mod hotkeys_macos;
#[cfg(test)]
pub(crate) mod native_settings_host;
mod notices_macos;
mod shared_dialogs_macos;
use crate::infrastructure::settings_canvas::general as shared_general_macos;
mod shared_page_macos;
mod shared_text_macos;
mod shared_view_macos;
pub(crate) use shared_page_macos::SharedFeaturePage;
pub(crate) use shared_view_macos::frame_image;
pub(crate) use shared_view_macos::SharedFeatureView;

#[cfg(test)]
pub(crate) use feature_macos::{FeatureField, FeatureKind, FeatureProjection, FeatureView};
#[cfg(test)]
pub(crate) use feature_page_macos::FeaturePage;
pub(crate) use general_controller::{
    GeneralAction, GeneralController, GeneralProjection, GeneralServices, UpdatePhase,
};
#[cfg(test)]
pub(crate) use general_macos::GeneralView;
#[cfg(test)]
pub(crate) use general_page_macos::GeneralPage;
#[cfg(test)]
pub(crate) use hotkeys_macos::{HotkeyView, HotkeyViewServices};
#[cfg(test)]
pub(crate) use notices_macos::test_workflow as test_notices_workflow;
