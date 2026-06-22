mod capture_commands;
mod capture_session_commands;
mod history_commands;
mod ocr_commands;
mod pinned_image_commands;
mod provider_commands;
mod screenshot_window_commands;
mod translation_commands;
mod workflow_commands;

pub use capture_commands::*;
pub use capture_session_commands::*;
pub use history_commands::*;
pub use ocr_commands::*;
pub use pinned_image_commands::*;
pub use provider_commands::*;
pub use screenshot_window_commands::*;
pub use translation_commands::*;
pub use workflow_commands::*;

use serde::Serialize;
use std::future::Future;
use std::time::Duration;
use tauri::{Emitter, Manager, State};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TranslationInputPayload {
    text: String,
    auto_translate: bool,
}

#[tauri::command]
pub fn open_result_window(text: String, app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;

        // Emit event to frontend with text
        window
            .emit("input-translation", text)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn open_ocr_result_window(text: String, app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        window.emit("input-ocr", text).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn copy_text_to_clipboard(text: String) -> Result<(), String> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| format!("Failed to open clipboard: {}", e))?;
    clipboard
        .set_text(text)
        .map_err(|e| format!("Failed to write text to clipboard: {}", e))
}

pub fn emit_screenshot_error(app: tauri::AppHandle, message: String) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.emit("screenshot-error", message);
    }
}

#[tauri::command]
pub fn open_translation_result_window(text: String, app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;

        window
            .emit(
                "input-translation",
                TranslationInputPayload {
                    text,
                    auto_translate: true,
                },
            )
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn show_translation_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        window
            .emit("show-translation-window", ())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn show_ocr_window(app: tauri::AppHandle) -> Result<(), String> {
    emit_main_window_event(app, "show-ocr-window")
}

pub fn start_file_ocr(app: tauri::AppHandle) -> Result<(), String> {
    emit_main_window_event(app, "start-file-ocr")
}

fn emit_main_window_event(app: tauri::AppHandle, event: &str) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        window.emit(event, ()).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub async fn open_selection_translation_window(app: tauri::AppHandle) -> Result<(), String> {
    let text = copy_selected_text(&app).await?;
    open_translation_result_window(text, app)
}

async fn copy_selected_text(app: &tauri::AppHandle) -> Result<String, String> {
    copy_selected_text_with(
        || press_selection_copy_shortcut_on_main_thread(app),
        read_clipboard_text,
        Duration::from_millis(120),
    )
    .await
}

async fn copy_selected_text_with<Press, PressFuture, Read>(
    press_copy_shortcut: Press,
    read_clipboard_text: Read,
    delay: Duration,
) -> Result<String, String>
where
    Press: FnOnce() -> PressFuture,
    PressFuture: Future<Output = Result<(), String>>,
    Read: FnOnce() -> Result<String, String>,
{
    press_copy_shortcut().await?;
    tokio::time::sleep(delay).await;
    let text = read_clipboard_text()?;
    validate_selected_text(text)
}

async fn press_selection_copy_shortcut_on_main_thread(
    app: &tauri::AppHandle,
) -> Result<(), String> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    app.run_on_main_thread(move || {
        let _ = sender.send(press_selection_copy_shortcut());
    })
    .map_err(|e| format!("Failed to dispatch selection copy shortcut: {}", e))?;

    receiver
        .await
        .map_err(|e| format!("Failed to receive selection copy shortcut result: {}", e))?
}

fn press_selection_copy_shortcut() -> Result<(), String> {
    use enigo::{Enigo, Key, KeyboardControllable};

    ensure_selection_copy_permission()?;

    let mut enigo = Enigo::new();
    #[cfg(target_os = "macos")]
    let modifier = Key::Meta;
    #[cfg(not(target_os = "macos"))]
    let modifier = Key::Control;

    enigo.key_down(modifier);
    enigo.key_click(Key::Layout('c'));
    enigo.key_up(modifier);

    Ok(())
}

fn ensure_selection_copy_permission() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        if !macos_accessibility_permission_granted() {
            return Err(
                "划词翻译需要 macOS 辅助功能权限。请在 系统设置 > 隐私与安全性 > 辅助功能 中允许 SnapLingo，然后重启应用。"
                    .to_string(),
            );
        }
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn macos_accessibility_permission_granted() -> bool {
    use std::os::raw::c_uchar;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> c_uchar;
    }

    unsafe { AXIsProcessTrusted() != 0 }
}

fn read_clipboard_text() -> Result<String, String> {
    use arboard::Clipboard;

    let mut clipboard = Clipboard::new().map_err(|e| format!("Failed to open clipboard: {}", e))?;
    clipboard
        .get_text()
        .map_err(|e| format!("Failed to read selected text from clipboard: {}", e))
}

fn validate_selected_text(text: String) -> Result<String, String> {
    if text.trim().is_empty() {
        return Err("Selected text is empty".to_string());
    }

    Ok(text)
}

#[tauri::command]
pub fn configure_hotkey(
    category: String,
    action: String,
    hotkey: String,
    app: tauri::AppHandle,
) -> Result<Option<String>, String> {
    crate::startup_shortcuts::configure_hotkey(&app, &category, &action, &hotkey)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn configure_translation_hotkey(
    action: String,
    hotkey: String,
    app: tauri::AppHandle,
) -> Result<Option<String>, String> {
    crate::startup_shortcuts::configure_translation_shortcut(&app, &action, &hotkey)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn trigger_screenshot(
    app: tauri::AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    open_capture_window_for_mode(&app, &state, "screenshot").await
}

#[cfg(test)]
mod tests {
    use super::copy_selected_text_with;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[tokio::test]
    async fn selection_copy_reads_clipboard_after_copy_shortcut() {
        let steps = Arc::new(Mutex::new(Vec::new()));
        let copy_steps = steps.clone();
        let read_steps = steps.clone();

        let text = copy_selected_text_with(
            move || async move {
                copy_steps.lock().unwrap().push("copy");
                Ok(())
            },
            move || {
                read_steps.lock().unwrap().push("read");
                Ok("selected text".to_string())
            },
            Duration::ZERO,
        )
        .await
        .unwrap();

        assert_eq!(text, "selected text");
        assert_eq!(*steps.lock().unwrap(), vec!["copy", "read"]);
    }

    #[tokio::test]
    async fn selection_copy_does_not_read_clipboard_when_copy_shortcut_fails() {
        let err = copy_selected_text_with(
            || async { Err("copy failed".to_string()) },
            || panic!("clipboard should not be read after copy failure"),
            Duration::ZERO,
        )
        .await
        .unwrap_err();

        assert_eq!(err, "copy failed");
    }

    #[tokio::test]
    async fn selection_copy_rejects_empty_clipboard_text() {
        let err = copy_selected_text_with(
            || async { Ok(()) },
            || Ok("   ".to_string()),
            Duration::ZERO,
        )
        .await
        .unwrap_err();

        assert_eq!(err, "Selected text is empty");
    }
}
