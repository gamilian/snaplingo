mod host;
mod view;
use super::geometry as scrolling_geometry;
use crate::integrations::system::layered_windows;
pub(crate) use host::*;
use view as scrolling_view_windows;
pub(super) use view::{set_working, update};
pub(super) async fn select_save_path(
    settings: &snaplingo_core::settings::SettingsConfiguration,
    output: &snaplingo_core::capture::CaptureOutput,
) -> crate::Result<Option<std::path::PathBuf>> {
    let preferences = settings.snapshot()?.screenshot;
    let format = preferences
        .last_save_format
        .as_deref()
        .unwrap_or(&preferences.format);
    let directory = preferences
        .last_save_directory
        .as_deref()
        .unwrap_or(&preferences.save_path);
    let fallback = preferences
        .last_save_directory
        .as_ref()
        .map(|_| preferences.save_path.as_str());
    let path = output.dialog_capture_save_path(
        Some(directory),
        fallback,
        format,
        &preferences.naming_rule,
        &preferences.custom_file_name,
    );
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let directory = path.parent().map(|p| p.to_path_buf());
    let future = crate::integrations::ui_thread::run("save scrolling capture", move || {
        let mut dialog = rfd::AsyncFileDialog::new()
            .set_title("保存滚动截图")
            .set_file_name(name)
            .add_filter("图片", &["png", "jpg", "webp"]);
        if let Some(directory) = directory {
            dialog = dialog.set_directory(directory);
        }
        Ok(dialog.save_file())
    })
    .await?;
    Ok(future.await.map(|path| path.path().to_path_buf()))
}
