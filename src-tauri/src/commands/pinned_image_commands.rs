use std::path::Path;

use tauri::State;

use crate::domain::capture::PinnedImageView;

#[tauri::command]
pub async fn pin_clipboard_image(state: State<'_, crate::AppState>) -> Result<(), String> {
    pin_clipboard_image_for_state(state.inner()).await
}

pub async fn pin_clipboard_image_for_state(state: &crate::AppState) -> Result<(), String> {
    state
        .capture
        .pinned_images
        .pin_clipboard()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn close_pinned_image(
    image_id: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .capture
        .pinned_images
        .close(&image_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_pinned_image(
    image_id: String,
    state: State<'_, crate::AppState>,
) -> Result<PinnedImageView, String> {
    state
        .capture
        .pinned_images
        .get(&image_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn remove_pinned_image(
    image_id: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .capture
        .pinned_images
        .remove(&image_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn copy_pinned_image(
    image_id: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .capture
        .pinned_images
        .copy(&image_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn replace_pinned_image_from_clipboard(
    image_id: String,
    state: State<'_, crate::AppState>,
) -> Result<PinnedImageView, String> {
    state
        .capture
        .pinned_images
        .replace_from_clipboard(&image_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn save_pinned_image(
    image_id: String,
    path: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .capture
        .pinned_images
        .save(&image_id, Path::new(&path))
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn toggle_pinned_images_visibility(
    state: State<'_, crate::AppState>,
) -> Result<Option<bool>, String> {
    toggle_pinned_images_visibility_for_state(state.inner()).await
}

pub async fn toggle_pinned_images_visibility_for_state(
    state: &crate::AppState,
) -> Result<Option<bool>, String> {
    state
        .capture
        .pinned_images
        .toggle_visibility()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn switch_pinned_image_group(
    state: State<'_, crate::AppState>,
) -> Result<Option<u32>, String> {
    switch_pinned_image_group_for_state(state.inner()).await
}

pub async fn switch_pinned_image_group_for_state(
    state: &crate::AppState,
) -> Result<Option<u32>, String> {
    state
        .capture
        .pinned_images
        .switch_group()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn move_pinned_image_to_next_group(
    image_id: String,
    state: State<'_, crate::AppState>,
) -> Result<u32, String> {
    state
        .capture
        .pinned_images
        .move_to_next_group(&image_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn hide_pinned_image_group(
    image_id: String,
    state: State<'_, crate::AppState>,
) -> Result<Vec<String>, String> {
    state
        .capture
        .pinned_images
        .hide_group(&image_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn destroy_pinned_image_group(
    image_id: String,
    state: State<'_, crate::AppState>,
) -> Result<Vec<String>, String> {
    state
        .capture
        .pinned_images
        .destroy_group(&image_id)
        .await
        .map_err(|error| error.to_string())
}
