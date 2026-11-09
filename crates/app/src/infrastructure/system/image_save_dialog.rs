use std::path::PathBuf;

#[cfg(target_os = "macos")]
pub(crate) async fn select_native_capture_save_path(
    settings: &crate::SettingsConfiguration,
    output: &crate::CaptureOutput,
) -> crate::Result<Option<PathBuf>> {
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
    let default_path = output.dialog_capture_save_path(
        Some(directory),
        fallback,
        format,
        &preferences.naming_rule,
        &preferences.custom_file_name,
    );
    super::dialogs::save_native_capture_file(default_path, format).await
}
