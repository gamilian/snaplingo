//! AppKit presentation/input adapters for the shared Rust Settings canvas.
mod dialogs;
pub(super) mod notices;
mod page;
mod text;
mod view;
pub(crate) use page::SharedFeaturePage;
pub(crate) use view::frame_image;
#[cfg(feature = "test-support")]
pub(crate) use view::SharedFeatureView;

#[cfg(feature = "test-support")]
pub(crate) use crate::features::settings::presenter::general_controller::UpdatePhase;
#[cfg(feature = "test-support")]
pub(crate) use notices::test_workflow as test_notices_workflow;

#[cfg(feature = "native-app")]
pub(crate) mod capacity;
#[cfg(feature = "native-app")]
mod host;
#[cfg(feature = "native-app")]
pub(crate) mod permissions;
#[cfg(feature = "native-app")]
pub(crate) use host::NativeSettingsWindow;
