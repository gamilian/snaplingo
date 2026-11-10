//! AppKit presentation/input adapters for the shared Rust Settings canvas.
#![allow(dead_code, unused_imports)]
use crate::features::settings::presenter::general_controller;
mod dialogs;
pub(super) mod notices;
mod page;
mod text;
mod view;
pub(crate) use page::SharedFeaturePage;
pub(crate) use view::frame_image;
pub(crate) use view::SharedFeatureView;

pub(crate) use general_controller::{
    GeneralAction, GeneralController, GeneralProjection, GeneralServices, UpdatePhase,
};
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
