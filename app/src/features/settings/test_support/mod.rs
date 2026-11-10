//! Independent retained AppKit references for native regression tests.
//! These pages are absent from production builds; production uses the shared UI.
#![allow(dead_code, unused_imports)]
mod feature;
mod feature_page;
mod general;
mod general_page;
mod hotkeys;
pub(crate) mod native_settings_host;

use super::macos::{notices, SharedFeaturePage};
use super::presenter::general_controller::{
    GeneralAction, GeneralController, GeneralProjection, GeneralServices, UpdatePhase,
};
pub(crate) use feature::{FeatureField, FeatureKind, FeatureProjection, FeatureView};
pub(crate) use feature_page::FeaturePage;
pub(crate) use general::GeneralView;
pub(crate) use general_page::GeneralPage;
pub(crate) use hotkeys::{HotkeyView, HotkeyViewServices};
