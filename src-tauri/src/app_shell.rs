use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};

use crate::app_actions::{dispatch_app_action, AppAction, CaptureLaunchMode};
use crate::application::hotkeys::display_hotkey_to_accelerator;
use crate::domain::hotkey_config::{hotkey_category, HotkeySettingsSnapshot, DEFAULT_HOTKEYS};
use crate::startup_shortcuts::hotkey_action_binding;

struct MenuBar(Menu<tauri::Wry>);

const TRAY_ID: &str = "snaplingo";
const SCREENSHOT_ID: &str = "screenshot";
const TRANSLATE_SELECTION_ID: &str = "translate-selection";
const SCREENSHOT_TRANSLATE_ID: &str = "screenshot-translate";
const SCREENSHOT_OCR_ID: &str = "screenshot-ocr";
const FILE_OCR_ID: &str = "file-ocr";
const SHOW_TRANSLATION_ID: &str = "show-translation";
const HISTORY_ID: &str = "history";
const SETTINGS_ID: &str = "settings";
const ABOUT_ID: &str = "about";
const QUIT_ID: &str = "quit";

pub(crate) fn set_capture_ocr_status(
    app: &tauri::AppHandle,
    status: Option<crate::application::capture::CaptureOcrStatus>,
) {
    use crate::application::capture::CaptureOcrStatus;
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let (title, tooltip) = match status {
        Some(CaptureOcrStatus::Recognizing) => ("OCR…", "SnapLingo — Recognizing text…"),
        Some(CaptureOcrStatus::Copied) => ("✓", "SnapLingo — Text copied"),
        Some(CaptureOcrStatus::Failed) => ("!", "SnapLingo — OCR failed"),
        None => ("", "SnapLingo"),
    };
    if let Err(error) = tray
        .set_title(Some(title))
        .and_then(|_| tray.set_tooltip(Some(tooltip)))
    {
        log::warn!("Failed to update capture OCR status: {error}");
    }
}

pub(crate) fn menu_action_for_id(id: &str) -> Option<AppAction> {
    match id {
        SCREENSHOT_ID => Some(AppAction::OpenCapture(CaptureLaunchMode::Screenshot)),
        TRANSLATE_SELECTION_ID => Some(AppAction::TranslateSelection),
        SCREENSHOT_TRANSLATE_ID => Some(AppAction::OpenCapture(
            CaptureLaunchMode::ScreenshotTranslate,
        )),
        SCREENSHOT_OCR_ID => Some(AppAction::OpenCapture(CaptureLaunchMode::ScreenshotOcr)),
        FILE_OCR_ID => Some(AppAction::RunFileOcr),
        SHOW_TRANSLATION_ID => Some(AppAction::OpenTranslationWindow),
        HISTORY_ID => Some(AppAction::OpenHistory),
        SETTINGS_ID => Some(AppAction::OpenSettings),
        ABOUT_ID => Some(AppAction::OpenAbout),
        QUIT_ID => Some(AppAction::Quit),
        _ => None,
    }
}

pub(crate) fn setup_menu_bar(app: &tauri::App) -> Result<(), String> {
    let screenshot = menu_item(app, SCREENSHOT_ID, "Screenshot")?;
    let translate_selection = menu_item(app, TRANSLATE_SELECTION_ID, "Translate Selection")?;
    let screenshot_translate = menu_item(app, SCREENSHOT_TRANSLATE_ID, "Screenshot Translate")?;
    let screenshot_ocr = menu_item(app, SCREENSHOT_OCR_ID, "Screenshot OCR")?;
    let file_ocr = menu_item(app, FILE_OCR_ID, "Upload Image OCR")?;
    let show_translation = menu_item(app, SHOW_TRANSLATION_ID, "Show Translation Window")?;
    let history = menu_item(app, HISTORY_ID, "History")?;
    let settings = menu_item(app, SETTINGS_ID, "Settings")?;
    let about = menu_item(app, ABOUT_ID, "About SnapLingo")?;
    let quit = menu_item(app, QUIT_ID, "Quit SnapLingo")?;

    let menu = Menu::with_items(
        app,
        &[
            &screenshot,
            &translate_selection,
            &screenshot_translate,
            &screenshot_ocr,
            &file_ocr,
            &show_translation,
            &history,
            &settings,
            &about,
            &quit,
        ],
    )
    .map_err(|e| e.to_string())?;

    let hotkeys = app
        .state::<crate::AppState>()
        .settings
        .hotkeys
        .snapshot()
        .map_err(|e| e.to_string())?;
    update_menu_shortcuts(&menu, &hotkeys)?;

    let mut tray = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .tooltip("SnapLingo")
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| {
            if let Some(action) = menu_action_for_id(event.id().as_ref()) {
                log::info!("Dispatching menu action: {:?}", action);
                dispatch_app_action(app.clone(), action);
            }
        });

    // Menu bar uses a dedicated monochrome template (alpha-only S glyph); the colored app
    // icon would render as a solid blob under `icon_as_template`.
    tray = tray
        .icon(tauri::include_image!("icons/trayTemplate.png"))
        .icon_as_template(true);

    tray.build(app).map_err(|e| e.to_string())?;
    app.manage(MenuBar(menu));
    Ok(())
}

pub(crate) fn refresh_menu_bar_shortcuts(app: &tauri::AppHandle) -> Result<(), String> {
    let Some(menu) = app.try_state::<MenuBar>() else {
        return Ok(());
    };
    let hotkeys = app
        .state::<crate::AppState>()
        .settings
        .hotkeys
        .snapshot()
        .map_err(|e| e.to_string())?;
    update_menu_shortcuts(&menu.0, &hotkeys)
}

fn update_menu_shortcuts(
    menu: &Menu<tauri::Wry>,
    hotkeys: &HotkeySettingsSnapshot,
) -> Result<(), String> {
    for item in menu.items().map_err(|e| e.to_string())? {
        if let Some(item) = item.as_menuitem() {
            item.set_accelerator(menu_shortcut(item.id().as_ref(), hotkeys)?)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn menu_shortcut(id: &str, hotkeys: &HotkeySettingsSnapshot) -> Result<Option<String>, String> {
    let Some(action) = menu_action_for_id(id) else {
        return Ok(None);
    };
    let hotkey = DEFAULT_HOTKEYS.iter().find(|hotkey| {
        hotkey_action_binding(hotkey.category, hotkey.action)
            .is_some_and(|binding| binding.action == action)
    });
    let Some(value) =
        hotkey.and_then(|hotkey| hotkey_category(hotkeys, hotkey.category)?.get(hotkey.action))
    else {
        return Ok(None);
    };
    display_hotkey_to_accelerator(value).map_err(|e| e.to_string())
}

fn menu_item(app: &tauri::App, id: &str, text: &str) -> Result<MenuItem<tauri::Wry>, String> {
    MenuItem::with_id(app, id, text, true, None::<&str>).map_err(|e| e.to_string())
}

pub(crate) fn should_prevent_implicit_exit(exit_code: Option<i32>) -> bool {
    exit_code.is_none()
}

#[cfg(target_os = "macos")]
fn menu_bar_resting_activation_policy() -> tauri::ActivationPolicy {
    tauri::ActivationPolicy::Accessory
}

#[cfg(target_os = "macos")]
pub(crate) fn apply_resting_activation_policy(app: &tauri::AppHandle) -> Result<(), String> {
    app.set_activation_policy(menu_bar_resting_activation_policy())
        .map_err(|e| e.to_string())
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn apply_resting_activation_policy(_app: &tauri::AppHandle) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_shortcuts_match_the_configured_actions() {
        let hotkeys = crate::domain::hotkey_config::default_hotkey_snapshot();
        for (id, expected) in [
            (SCREENSHOT_ID, Some("F1")),
            (TRANSLATE_SELECTION_ID, Some("Alt+KeyD")),
            (SCREENSHOT_TRANSLATE_ID, Some("Alt+KeyS")),
            (SCREENSHOT_OCR_ID, Some("Shift+Alt+KeyS")),
            (SHOW_TRANSLATION_ID, Some("Alt+KeyA")),
            (FILE_OCR_ID, None),
            (HISTORY_ID, None),
            (SETTINGS_ID, None),
            (ABOUT_ID, None),
            (QUIT_ID, None),
        ] {
            assert_eq!(
                menu_shortcut(id, &hotkeys).unwrap().as_deref(),
                expected,
                "{id}"
            );
        }
    }

    #[test]
    fn menu_shortcuts_follow_custom_clear_and_reset_configuration() {
        use std::sync::Arc;

        use crate::application::hotkeys::HotkeyConfiguration;
        use crate::domain::hotkey_config::{FILE_OCR_ACTION, HOTKEY_UNSET, OCR_CATEGORY};
        use crate::infrastructure::storage::SqliteConfigStore;

        let configuration = HotkeyConfiguration::new(Arc::new(SqliteConfigStore::new_in_memory()));
        for (value, expected) in [
            ("⌃⇧F6", Some("Ctrl+Shift+F6")),
            (HOTKEY_UNSET, None),
            ("  ", None),
            ("⌥O", Some("Alt+KeyO")),
        ] {
            let hotkeys = configuration
                .update_hotkey(OCR_CATEGORY, FILE_OCR_ACTION, value)
                .unwrap();
            assert_eq!(
                menu_shortcut(FILE_OCR_ID, &hotkeys).unwrap().as_deref(),
                expected
            );
            assert_eq!(
                menu_shortcut(SCREENSHOT_OCR_ID, &hotkeys)
                    .unwrap()
                    .as_deref(),
                Some("Shift+Alt+KeyS")
            );
        }
        let hotkeys = configuration.reset_category(OCR_CATEGORY).unwrap();
        assert_eq!(menu_shortcut(FILE_OCR_ID, &hotkeys).unwrap(), None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn menu_bar_shell_uses_accessory_activation_policy() {
        assert!(matches!(
            menu_bar_resting_activation_policy(),
            tauri::ActivationPolicy::Accessory
        ));
    }

    #[test]
    fn menu_bar_shell_stays_alive_when_last_window_is_destroyed() {
        assert!(should_prevent_implicit_exit(None));
    }

    #[test]
    fn menu_bar_shell_allows_explicit_quit() {
        assert!(!should_prevent_implicit_exit(Some(0)));
    }

    #[test]
    fn maps_known_menu_item_ids_to_actions() {
        assert_eq!(
            menu_action_for_id("screenshot"),
            Some(AppAction::OpenCapture(CaptureLaunchMode::Screenshot))
        );
        assert_eq!(
            menu_action_for_id("translate-selection"),
            Some(AppAction::TranslateSelection)
        );
        assert_eq!(
            menu_action_for_id("screenshot-translate"),
            Some(AppAction::OpenCapture(
                CaptureLaunchMode::ScreenshotTranslate
            ))
        );
        assert_eq!(
            menu_action_for_id("screenshot-ocr"),
            Some(AppAction::OpenCapture(CaptureLaunchMode::ScreenshotOcr))
        );
        assert_eq!(menu_action_for_id("file-ocr"), Some(AppAction::RunFileOcr));
        assert_eq!(
            menu_action_for_id("show-translation"),
            Some(AppAction::OpenTranslationWindow)
        );
        assert_eq!(menu_action_for_id("history"), Some(AppAction::OpenHistory));
        assert_eq!(
            menu_action_for_id("settings"),
            Some(AppAction::OpenSettings)
        );
        assert_eq!(menu_action_for_id("about"), Some(AppAction::OpenAbout));
        assert_eq!(menu_action_for_id("quit"), Some(AppAction::Quit));
    }

    #[test]
    fn rejects_unknown_menu_item_ids() {
        assert_eq!(menu_action_for_id("unknown"), None);
    }
}
