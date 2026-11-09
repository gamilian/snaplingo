//! AppKit presentation/input adapters for the shared Rust Settings canvas.
#![allow(dead_code, unused_imports)]
#[cfg(test)]
mod feature;
#[cfg(test)]
mod feature_page;
use snaplingo_settings::general_controller;
mod dialogs;
#[cfg(test)]
mod general;
#[cfg(test)]
mod general_page;
#[cfg(test)]
mod hotkeys;
#[cfg(test)]
pub(crate) mod native_settings_host;
mod notices;
mod page;
mod text;
mod view;
pub(crate) use page::SharedFeaturePage;
pub(crate) use view::frame_image;
pub(crate) use view::SharedFeatureView;

#[cfg(test)]
pub(crate) use feature::{FeatureField, FeatureKind, FeatureProjection, FeatureView};
#[cfg(test)]
pub(crate) use feature_page::FeaturePage;
#[cfg(test)]
pub(crate) use general::GeneralView;
pub(crate) use general_controller::{
    GeneralAction, GeneralController, GeneralProjection, GeneralServices, UpdatePhase,
};
#[cfg(test)]
pub(crate) use general_page::GeneralPage;
#[cfg(test)]
pub(crate) use hotkeys::{HotkeyView, HotkeyViewServices};
#[cfg(test)]
pub(crate) use notices::test_workflow as test_notices_workflow;

#[cfg(feature = "native-app")]
pub(crate) mod capacity;
#[cfg(feature = "native-app")]
mod host;
#[cfg(feature = "native-app")]
pub(crate) mod permissions;
#[cfg(feature = "native-app")]
pub(crate) use host::NativeSettingsWindow;
