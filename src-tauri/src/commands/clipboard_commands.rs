#[tauri::command]
pub fn copy_text_to_clipboard(text: String) -> Result<(), String> {
    crate::infrastructure::system::clipboard_text::copy_text(text)
}
