use std::path::PathBuf;

use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

pub(super) async fn select_capture_save_path(app: &AppHandle) -> crate::Result<Option<PathBuf>> {
    let state = app.state::<crate::AppState>();
    let preferences = state.settings.configuration.snapshot()?.screenshot;
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
    let default_path = state.capture.output.dialog_capture_save_path(
        Some(directory),
        fallback,
        format,
        &preferences.naming_rule,
        &preferences.custom_file_name,
    );
    let mut dialog = app.dialog().file().set_title("保存截图");
    if let Some(directory) = default_path.parent() {
        dialog = dialog.set_directory(directory);
    }
    if let Some(name) = default_path.file_name() {
        dialog = dialog.set_file_name(name.to_string_lossy());
    }
    let mut filters = [
        ("PNG（无损）", &["png"][..]),
        ("JPEG", &["jpg", "jpeg"][..]),
        ("WebP（无损）", &["webp"][..]),
    ];
    filters.sort_by_key(|(_, extensions)| !extensions.contains(&format));
    for (name, extensions) in filters {
        dialog = dialog.add_filter(name, extensions);
    }
    let (sender, receiver) = tokio::sync::oneshot::channel();
    dialog.save_file(move |path| {
        let _ = sender.send(path);
    });
    let Some(path) = receiver.await.map_err(|error| error.to_string())? else {
        return Ok(None);
    };
    let mut path = path.into_path().map_err(|error| error.to_string())?;
    if path.extension().is_none() {
        path.set_extension(format);
    }
    Ok(Some(path))
}
