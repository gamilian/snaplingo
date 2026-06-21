use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::domain::capture::PinnedImageView;

use super::backend::{
    destroyed_pinned_group_window_labels, hidden_pinned_group_window_visibility_changes,
    is_pinned_window_label, moved_pinned_image_window_visibility_change,
    next_pinned_windows_visible_state, pinned_group_window_visibility_changes, pinned_window_label,
    pinned_window_size, pinned_window_url,
};

pub fn close_pinned_image_window(app: &AppHandle, image_id: &str) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(&pinned_window_label(image_id)) {
        window.hide().map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub fn show_or_open_pinned_image_window(
    app: &AppHandle,
    image: &PinnedImageView,
) -> Result<(), String> {
    let label = pinned_window_label(&image.id);
    if let Some(window) = app.get_webview_window(&label) {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    open_pinned_image_window(app, image)
}

pub fn open_pinned_image_window(app: &AppHandle, image: &PinnedImageView) -> Result<(), String> {
    let label = pinned_window_label(&image.id);
    let (width, height) = pinned_window_size(image.width, image.height);

    WebviewWindowBuilder::new(app, &label, WebviewUrl::App(pinned_window_url(&image.id)))
        .title("SnapLingo Pin")
        .inner_size(width, height)
        .min_inner_size(80.0, 60.0)
        .decorations(false)
        .always_on_top(true)
        .visible_on_all_workspaces(true)
        .skip_taskbar(true)
        .focused(true)
        .shadow(true)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn toggle_pinned_image_windows_visibility(app: &AppHandle) -> Result<Option<bool>, String> {
    let pinned_windows: Vec<_> = app
        .webview_windows()
        .into_iter()
        .filter(|(label, _)| is_pinned_window_label(label))
        .map(|(_, window)| window)
        .collect();
    let visibility: Vec<bool> = pinned_windows
        .iter()
        .map(|window| window.is_visible().unwrap_or(false))
        .collect();
    let Some(next_visible) = next_pinned_windows_visible_state(&visibility) else {
        return Ok(None);
    };

    for window in pinned_windows {
        if next_visible {
            window.show().map_err(|e| e.to_string())?;
        } else {
            window.hide().map_err(|e| e.to_string())?;
        }
    }

    Ok(Some(next_visible))
}

pub fn apply_pinned_group_window_switch(
    app: &AppHandle,
    hide_image_ids: &[String],
    show_image_ids: &[String],
) -> Result<(), String> {
    for change in pinned_group_window_visibility_changes(hide_image_ids, show_image_ids) {
        let Some(window) = app.get_webview_window(&change.label) else {
            continue;
        };

        if change.visible {
            window.show().map_err(|e| e.to_string())?;
        } else {
            window.hide().map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

pub fn hide_moved_pinned_image_window(app: &AppHandle, image_id: &str) -> Result<(), String> {
    let change = moved_pinned_image_window_visibility_change(image_id);

    if let Some(window) = app.get_webview_window(&change.label) {
        window.hide().map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub fn hide_pinned_group_windows(app: &AppHandle, image_ids: &[String]) -> Result<(), String> {
    for change in hidden_pinned_group_window_visibility_changes(image_ids) {
        if let Some(window) = app.get_webview_window(&change.label) {
            window.hide().map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

pub fn close_pinned_group_windows(app: &AppHandle, image_ids: &[String]) -> Result<(), String> {
    for label in destroyed_pinned_group_window_labels(image_ids) {
        if let Some(window) = app.get_webview_window(&label) {
            window.close().map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}
