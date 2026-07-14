use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};

use crate::app_actions::{dispatch_app_action, AppAction, CaptureLaunchMode};

const TRAY_ID: &str = "snaplingo";
const SCREENSHOT_ID: &str = "screenshot";
const TRANSLATE_SELECTION_ID: &str = "translate-selection";
const SCREENSHOT_TRANSLATE_ID: &str = "screenshot-translate";
const INPUT_TRANSLATION_ID: &str = "input-translation";
const SCREENSHOT_OCR_ID: &str = "screenshot-ocr";
const FILE_OCR_ID: &str = "file-ocr";
const SETTINGS_ID: &str = "settings";
const ABOUT_ID: &str = "about";
const QUIT_ID: &str = "quit";

pub(crate) fn menu_action_for_id(id: &str) -> Option<AppAction> {
    match id {
        SCREENSHOT_ID => Some(AppAction::OpenCapture(CaptureLaunchMode::Screenshot)),
        TRANSLATE_SELECTION_ID => Some(AppAction::TranslateSelection),
        SCREENSHOT_TRANSLATE_ID => Some(AppAction::OpenCapture(
            CaptureLaunchMode::ScreenshotTranslate,
        )),
        INPUT_TRANSLATION_ID => Some(AppAction::OpenInputTranslation),
        SCREENSHOT_OCR_ID => Some(AppAction::OpenCapture(CaptureLaunchMode::ScreenshotOcr)),
        FILE_OCR_ID => Some(AppAction::RunFileOcr),
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
    let input_translation = menu_item(app, INPUT_TRANSLATION_ID, "Input Translation")?;
    let screenshot_ocr = menu_item(app, SCREENSHOT_OCR_ID, "Screenshot OCR")?;
    let file_ocr = menu_item(app, FILE_OCR_ID, "Upload Image OCR")?;
    let settings = menu_item(app, SETTINGS_ID, "Settings")?;
    let about = menu_item(app, ABOUT_ID, "About SnapLingo")?;
    let quit = menu_item(app, QUIT_ID, "Quit SnapLingo")?;

    let menu = Menu::with_items(
        app,
        &[
            &screenshot,
            &translate_selection,
            &screenshot_translate,
            &input_translation,
            &screenshot_ocr,
            &file_ocr,
            &settings,
            &about,
            &quit,
        ],
    )
    .map_err(|e| e.to_string())?;

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

    if let Some(icon) = app.default_window_icon().cloned() {
        tray = tray.icon(icon).icon_as_template(true);
    }

    tray.build(app).map_err(|e| e.to_string())?;
    Ok(())
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
            menu_action_for_id("input-translation"),
            Some(AppAction::OpenInputTranslation)
        );
        assert_eq!(
            menu_action_for_id("screenshot-ocr"),
            Some(AppAction::OpenCapture(CaptureLaunchMode::ScreenshotOcr))
        );
        assert_eq!(menu_action_for_id("file-ocr"), Some(AppAction::RunFileOcr));
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
