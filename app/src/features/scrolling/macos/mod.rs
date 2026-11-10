mod controls;
mod host;
mod overlays;
use super::geometry as scrolling_geometry;
use crate::integrations::system::native_theme;
use controls as scrolling_controls_macos;
pub(super) use controls::{set_working, update};
pub(crate) use host::*;
use overlays as scrolling_overlays_macos;
pub(super) async fn select_save_path(
    settings: &snaplingo_core::settings::SettingsConfiguration,
    output: &snaplingo_core::capture::CaptureOutput,
) -> crate::Result<Option<std::path::PathBuf>> {
    crate::integrations::system::image_save_dialog::select_native_capture_save_path(
        settings, output,
    )
    .await
}
