mod capture_commands;
mod capture_session_commands;
mod history_commands;
mod ocr_commands;
mod pinned_image_commands;
mod provider_commands;
mod screenshot_window_commands;
mod translation_commands;

pub use capture_commands::*;
pub use capture_session_commands::*;
pub use history_commands::*;
pub use ocr_commands::*;
pub use pinned_image_commands::*;
pub use provider_commands::*;
pub use screenshot_window_commands::*;
pub use translation_commands::*;

use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

#[cfg(target_os = "macos")]
use objc2_app_kit::{
    NSScreenSaverWindowLevel, NSWindow, NSWindowAnimationBehavior, NSWindowCollectionBehavior,
    NSWindowStyleMask,
};
use serde::Serialize;
use tauri::{Emitter, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::{
    business_windows::{CAPTURE_RESULT_WINDOW_LABEL, CAPTURE_WINDOW_LABEL},
    settings_window,
};

static CAPTURE_RESULT_WINDOW_PAYLOAD: LazyLock<Mutex<Option<CaptureResultWindowPayload>>> =
    LazyLock::new(|| Mutex::new(None));

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum CaptureResultWindowMode {
    Translation,
    Ocr,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureResultWindowPayload {
    mode: CaptureResultWindowMode,
    text: String,
    auto_translate: bool,
    start_file_ocr: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ResultWindowEntrypoint {
    ManualTranslation,
    AutoTranslation,
    Ocr,
    ShowOcr,
    FileOcr,
}

#[tauri::command]
pub fn open_result_window(text: String, app: tauri::AppHandle) -> Result<(), String> {
    open_capture_result_window(
        result_window_payload_for_entrypoint(ResultWindowEntrypoint::ManualTranslation, text),
        app,
    )
}

#[tauri::command]
pub fn open_ocr_result_window(text: String, app: tauri::AppHandle) -> Result<(), String> {
    open_capture_result_window(
        result_window_payload_for_entrypoint(ResultWindowEntrypoint::Ocr, text),
        app,
    )
}

#[tauri::command]
pub fn open_capture_ocr_result_window(text: String, app: tauri::AppHandle) -> Result<(), String> {
    open_capture_result_window(capture_ocr_result_payload(text), app)
}

#[tauri::command]
pub fn open_capture_translation_result_window(
    text: String,
    app: tauri::AppHandle,
) -> Result<(), String> {
    open_capture_result_window(capture_translation_result_payload(text), app)
}

#[tauri::command]
pub fn take_capture_result_window_payload() -> Result<Option<CaptureResultWindowPayload>, String> {
    let mut payload = CAPTURE_RESULT_WINDOW_PAYLOAD
        .lock()
        .map_err(|_| "Capture result window payload lock poisoned".to_string())?;

    Ok(payload.take())
}

fn open_capture_result_window(
    payload: CaptureResultWindowPayload,
    app: tauri::AppHandle,
) -> Result<(), String> {
    {
        let mut pending_payload = CAPTURE_RESULT_WINDOW_PAYLOAD
            .lock()
            .map_err(|_| "Capture result window payload lock poisoned".to_string())?;
        *pending_payload = Some(payload);
    }

    let window = match app.get_webview_window(CAPTURE_RESULT_WINDOW_LABEL) {
        Some(window) => window,
        None => WebviewWindowBuilder::new(
            &app,
            CAPTURE_RESULT_WINDOW_LABEL,
            WebviewUrl::App(capture_result_window_url()),
        )
        .title("SnapLingo Result")
        .inner_size(780.0, 560.0)
        .position(120.0, 120.0)
        .decorations(false)
        .always_on_top(true)
        .visible_on_all_workspaces(true)
        .transparent(true)
        .visible(false)
        .skip_taskbar(true)
        .focused(false)
        .shadow(true)
        .build()
        .map_err(|e| e.to_string())?,
    };

    reveal_capture_result_window(&window)?;
    window
        .emit("capture-result-payload-ready", ())
        .map_err(|e| e.to_string())
}

fn capture_result_window_url() -> PathBuf {
    PathBuf::from("index.html?window=capture-result")
}

fn capture_translation_result_payload(text: String) -> CaptureResultWindowPayload {
    translation_result_payload(text, true)
}

fn capture_ocr_result_payload(text: String) -> CaptureResultWindowPayload {
    ocr_result_payload(text, false)
}

fn translation_result_payload(text: String, auto_translate: bool) -> CaptureResultWindowPayload {
    CaptureResultWindowPayload {
        mode: CaptureResultWindowMode::Translation,
        text,
        auto_translate,
        start_file_ocr: false,
    }
}

fn ocr_result_payload(text: String, start_file_ocr: bool) -> CaptureResultWindowPayload {
    CaptureResultWindowPayload {
        mode: CaptureResultWindowMode::Ocr,
        text,
        auto_translate: false,
        start_file_ocr,
    }
}

fn result_window_payload_for_entrypoint(
    entrypoint: ResultWindowEntrypoint,
    text: String,
) -> CaptureResultWindowPayload {
    match entrypoint {
        ResultWindowEntrypoint::ManualTranslation => translation_result_payload(text, false),
        ResultWindowEntrypoint::AutoTranslation => translation_result_payload(text, true),
        ResultWindowEntrypoint::Ocr | ResultWindowEntrypoint::ShowOcr => {
            ocr_result_payload(text, false)
        }
        ResultWindowEntrypoint::FileOcr => ocr_result_payload(String::new(), true),
    }
}

fn reveal_capture_result_window(window: &WebviewWindow) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        configure_capture_result_window_for_current_space(window)?;
        window.show().map_err(|e| e.to_string())?;
        let ns_window = window.ns_window().map_err(|e| e.to_string())?;
        if ns_window.is_null() {
            return Err("Capture result window has no native NSWindow".to_string());
        }

        let ns_window: &NSWindow = unsafe { &*ns_window.cast() };
        ns_window.orderFrontRegardless();
        window.set_focus().map_err(|e| e.to_string())?;
    }

    #[cfg(not(target_os = "macos"))]
    {
        window.show().map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn configure_capture_result_window_for_current_space(window: &WebviewWindow) -> Result<(), String> {
    let ns_window = window.ns_window().map_err(|e| e.to_string())?;
    if ns_window.is_null() {
        return Err("Capture result window has no native NSWindow".to_string());
    }

    let ns_window: &NSWindow = unsafe { &*ns_window.cast() };
    ns_window.setStyleMask(ns_window.styleMask() | NSWindowStyleMask::Borderless);
    ns_window.setCollectionBehavior(
        ns_window.collectionBehavior()
            | NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::Stationary
            | NSWindowCollectionBehavior::Transient
            | NSWindowCollectionBehavior::IgnoresCycle,
    );
    ns_window.setLevel(NSScreenSaverWindowLevel);
    ns_window.setCanHide(false);
    ns_window.setHidesOnDeactivate(false);
    if capture_result_window_disables_window_animation() {
        ns_window.setAnimationBehavior(NSWindowAnimationBehavior::None);
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn capture_result_window_disables_window_animation() -> bool {
    true
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
    if let Some(window) = app.get_webview_window(settings_window::SETTINGS_WINDOW_LABEL) {
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.emit("screenshot-error", message);
    }
}

pub fn emit_capture_screenshot_error(app: tauri::AppHandle, message: String) {
    if let Some(window) = app.get_webview_window(CAPTURE_WINDOW_LABEL) {
        let _ = window.emit("screenshot-error", message);
        return;
    }

    if should_focus_main_for_capture_screenshot_error() {
        emit_screenshot_error(app, message);
    } else {
        log::error!("Capture screenshot error: {}", message);
    }
}

fn should_focus_main_for_capture_screenshot_error() -> bool {
    false
}

#[tauri::command]
pub fn open_translation_result_window(text: String, app: tauri::AppHandle) -> Result<(), String> {
    open_capture_result_window(
        result_window_payload_for_entrypoint(ResultWindowEntrypoint::AutoTranslation, text),
        app,
    )
}

pub fn show_translation_window(app: tauri::AppHandle) -> Result<(), String> {
    open_result_window(String::new(), app)
}

pub fn show_ocr_window(app: tauri::AppHandle) -> Result<(), String> {
    open_capture_result_window(
        result_window_payload_for_entrypoint(ResultWindowEntrypoint::ShowOcr, String::new()),
        app,
    )
}

pub fn start_file_ocr(app: tauri::AppHandle) -> Result<(), String> {
    open_capture_result_window(
        result_window_payload_for_entrypoint(ResultWindowEntrypoint::FileOcr, String::new()),
        app,
    )
}

pub async fn open_selection_translation_window_for_state(
    app: tauri::AppHandle,
    state: &crate::AppState,
) -> Result<(), String> {
    let snapshot = state
        .selected_text_acquirer
        .acquire()
        .await
        .map_err(|e| e.to_string())?;
    open_translation_result_window(snapshot.text, app)
}

#[tauri::command]
pub async fn open_selection_translation_window(
    app: tauri::AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    open_selection_translation_window_for_state(app, state.inner()).await
}

#[tauri::command]
pub fn configure_hotkey(
    category: String,
    action: String,
    hotkey: String,
    app: tauri::AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<Option<String>, String> {
    let accelerator = crate::startup_shortcuts::configure_hotkey(&app, &category, &action, &hotkey)
        .map_err(|e| e.to_string())?;
    crate::startup_shortcuts::save_hotkey_config(&state.config_file, &category, &action, &hotkey)
        .map_err(|e| e.to_string())?;
    Ok(accelerator)
}

#[tauri::command]
pub fn configure_translation_hotkey(
    action: String,
    hotkey: String,
    app: tauri::AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<Option<String>, String> {
    let accelerator =
        crate::startup_shortcuts::configure_translation_shortcut(&app, &action, &hotkey)
            .map_err(|e| e.to_string())?;
    crate::startup_shortcuts::save_hotkey_config(
        &state.config_file,
        "translation",
        &action,
        &hotkey,
    )
    .map_err(|e| e.to_string())?;
    Ok(accelerator)
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
    use super::*;

    #[test]
    fn capture_result_window_uses_dedicated_route() {
        assert_eq!(
            capture_result_window_url().to_string_lossy(),
            "index.html?window=capture-result"
        );
    }

    #[test]
    fn capture_translation_payload_requests_auto_translation() {
        let payload = capture_translation_result_payload("hello".to_string());

        assert_eq!(payload.mode, CaptureResultWindowMode::Translation);
        assert_eq!(payload.text, "hello");
        assert!(payload.auto_translate);
    }

    #[test]
    fn capture_ocr_payload_does_not_request_auto_translation() {
        let payload = capture_ocr_result_payload("hello".to_string());

        assert_eq!(payload.mode, CaptureResultWindowMode::Ocr);
        assert_eq!(payload.text, "hello");
        assert!(!payload.auto_translate);
    }

    #[test]
    fn result_entrypoints_use_dedicated_result_window_payloads() {
        assert_eq!(
            result_window_payload_for_entrypoint(
                ResultWindowEntrypoint::ManualTranslation,
                "hello".to_string(),
            ),
            CaptureResultWindowPayload {
                mode: CaptureResultWindowMode::Translation,
                text: "hello".to_string(),
                auto_translate: false,
                start_file_ocr: false,
            }
        );
        assert_eq!(
            result_window_payload_for_entrypoint(
                ResultWindowEntrypoint::AutoTranslation,
                "hello".to_string(),
            )
            .auto_translate,
            true
        );
        assert_eq!(
            result_window_payload_for_entrypoint(ResultWindowEntrypoint::ShowOcr, String::new()),
            CaptureResultWindowPayload {
                mode: CaptureResultWindowMode::Ocr,
                text: String::new(),
                auto_translate: false,
                start_file_ocr: false,
            }
        );
        assert_eq!(
            result_window_payload_for_entrypoint(ResultWindowEntrypoint::FileOcr, String::new()),
            CaptureResultWindowPayload {
                mode: CaptureResultWindowMode::Ocr,
                text: String::new(),
                auto_translate: false,
                start_file_ocr: true,
            }
        );
    }

    #[test]
    fn capture_screenshot_errors_do_not_focus_main_window() {
        assert!(!should_focus_main_for_capture_screenshot_error());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn capture_result_window_disables_appkit_window_animation() {
        assert!(capture_result_window_disables_window_animation());
    }
}
