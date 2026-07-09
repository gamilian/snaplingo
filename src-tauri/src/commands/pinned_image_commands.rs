use std::path::Path;

use tauri::{AppHandle, State};

use crate::application::services::PinnedImageOpenRequest;
use crate::domain::capture::PinnedImageView;
use crate::infrastructure::system::pinned_window::{
    apply_pinned_group_window_switch, close_pinned_group_windows, close_pinned_image_window,
    hide_moved_pinned_image_window, hide_pinned_group_windows, open_pinned_image_window,
    show_or_open_pinned_image_window, toggle_pinned_image_windows_visibility,
};

#[tauri::command]
pub fn pin_clipboard_image(
    app: AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    pin_clipboard_image_for_state(&app, state.inner())
}

pub fn pin_clipboard_image_for_state(
    app: &AppHandle,
    state: &crate::AppState,
) -> Result<(), String> {
    let request = state
        .capture
        .pinned_images
        .pin_clipboard_capture_output(
            &state.capture.image_composition,
            &state.capture.output,
        )
        .map_err(|e| e.to_string())?;

    open_pinned_image_for_request(app, request)
}

fn open_pinned_image_for_request(
    app: &AppHandle,
    request: PinnedImageOpenRequest,
) -> Result<(), String> {
    match request {
        PinnedImageOpenRequest::Reopen(image) => show_or_open_pinned_image_window(app, &image),
        PinnedImageOpenRequest::Open(image) => open_pinned_image_window(app, &image),
    }
}

#[tauri::command]
pub fn close_pinned_image(
    image_id: String,
    app: AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    close_pinned_image_for_state(&image_id, &app, state.inner())
}

pub fn close_pinned_image_for_state(
    image_id: &str,
    app: &AppHandle,
    state: &crate::AppState,
) -> Result<(), String> {
    state
        .capture
        .pinned_images
        .close_pinned_image(image_id)
        .map_err(|e| e.to_string())?;

    close_pinned_image_window(app, image_id)
}

#[tauri::command]
pub fn get_pinned_image(
    image_id: String,
    state: State<'_, crate::AppState>,
) -> Result<PinnedImageView, String> {
    state
        .inner()
        .capture
        .pinned_images
        .get_pinned_image(&image_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_pinned_image(
    image_id: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .inner()
        .capture
        .pinned_images
        .remove_pinned_image(&image_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn copy_pinned_image(
    image_id: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .inner()
        .capture
        .pinned_images
        .copy_pinned_png_to_clipboard(&state.inner().capture.output, &image_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn replace_pinned_image_from_clipboard(
    image_id: String,
    state: State<'_, crate::AppState>,
) -> Result<PinnedImageView, String> {
    state
        .inner()
        .capture
        .pinned_images
        .replace_clipboard_capture_output_view(
            &state.inner().capture.image_composition,
            &state.inner().capture.output,
            &image_id,
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_pinned_image(
    image_id: String,
    path: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .inner()
        .capture
        .pinned_images
        .save_pinned_png_to_path(
            &state.inner().capture.output,
            &image_id,
            Path::new(&path),
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn toggle_pinned_images_visibility(app: AppHandle) -> Result<Option<bool>, String> {
    toggle_pinned_image_windows_visibility(&app)
}

#[tauri::command]
pub fn switch_pinned_image_group(
    app: AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<Option<u32>, String> {
    switch_pinned_image_group_for_state(&app, state.inner())
}

pub fn switch_pinned_image_group_for_state(
    app: &AppHandle,
    state: &crate::AppState,
) -> Result<Option<u32>, String> {
    let Some(group_switch) = state.capture.pinned_images.switch_to_next_group() else {
        return Ok(None);
    };

    apply_pinned_group_window_switch(
        app,
        &group_switch.hide_image_ids,
        &group_switch.show_image_ids,
    )?;

    Ok(Some(group_switch.next_group))
}

#[tauri::command]
pub fn move_pinned_image_to_next_group(
    image_id: String,
    app: AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<u32, String> {
    let next_group = state
        .inner()
        .capture
        .pinned_images
        .move_pinned_image_to_next_group(&image_id)
        .map_err(|e| e.to_string())?;
    hide_moved_pinned_image_window(&app, &image_id)?;

    Ok(next_group)
}

#[tauri::command]
pub fn hide_pinned_image_group(
    image_id: String,
    app: AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<Vec<String>, String> {
    let membership = state
        .inner()
        .capture
        .pinned_images
        .pinned_image_group_containing(&image_id)
        .map_err(|e| e.to_string())?;

    hide_pinned_group_windows(&app, &membership.image_ids)?;

    Ok(membership.image_ids)
}

#[tauri::command]
pub fn destroy_pinned_image_group(
    image_id: String,
    app: AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<Vec<String>, String> {
    let removal = state
        .inner()
        .capture
        .pinned_images
        .remove_pinned_image_group_containing(&image_id)
        .map_err(|e| e.to_string())?;

    close_pinned_group_windows(&app, &removal.removed_image_ids)?;

    Ok(removal.removed_image_ids)
}
