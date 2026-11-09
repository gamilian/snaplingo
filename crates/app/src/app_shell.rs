use std::cell::RefCell;
use std::sync::{Arc, LazyLock, Mutex, Once};

use tray_icon::{
    menu::{accelerator::Accelerator, Menu, MenuEvent, MenuItem},
    Icon, TrayIcon, TrayIconBuilder,
};

use crate::app_actions::{AppAction, CaptureLaunchMode};
use crate::application::hotkeys::display_hotkey_to_accelerator;
use crate::domain::hotkey_config::{hotkey_category, HotkeySettingsSnapshot, DEFAULT_HOTKEYS};
use crate::startup_shortcuts::hotkey_action_binding;

struct MenuBar {
    menu: Menu,
    tray: TrayIcon,
}

type MenuDispatch = Arc<dyn Fn(AppAction) + Send + Sync>;
static MENU_DISPATCH: LazyLock<Mutex<Option<MenuDispatch>>> = LazyLock::new(|| Mutex::new(None));
static MENU_HANDLER: Once = Once::new();
thread_local! {
    // Native menu/tray values own non-Send UI objects and are created/dropped on the event-loop thread.
    static MENU_BAR: RefCell<Option<MenuBar>> = const { RefCell::new(None) };
}

/// Install before constructing menus; muda accepts only its first global handler.
pub(crate) fn initialize_menu_events() {
    MENU_HANDLER.call_once(|| {
        MenuEvent::set_event_handler(Some(|event: MenuEvent| {
            if let Some(action) = menu_action_for_id(event.id().as_ref()) {
                let dispatch = MENU_DISPATCH.lock().unwrap().clone();
                if let Some(dispatch) = dispatch {
                    dispatch(action);
                }
            }
        }));
    });
}

pub(crate) fn shutdown_menu_bar() {
    *MENU_DISPATCH.lock().unwrap() = None;
    MENU_BAR.with_borrow_mut(Option::take);
}

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

#[cfg(any(
    target_os = "macos",
    all(target_os = "windows", feature = "native-app")
))]
pub(crate) fn set_capture_ocr_status_native(
    status: Option<crate::application::capture::CaptureOcrStatus>,
) {
    crate::infrastructure::ui_thread::dispatch(capture_ocr_status_update(status));
}

pub(crate) fn capture_ocr_status_update(
    status: Option<crate::application::capture::CaptureOcrStatus>,
) -> impl FnOnce() + Send {
    use crate::application::capture::CaptureOcrStatus;
    let (title, tooltip) = match status {
        Some(CaptureOcrStatus::Recognizing) => {
            ("OCR…", "SnapLingo — Recognizing text…".to_string())
        }
        Some(CaptureOcrStatus::Copied { language }) => (
            "✓",
            match language {
                Some(language) => format!("SnapLingo — Text copied · OCR language: {language}"),
                None => "SnapLingo — Text copied".to_string(),
            },
        ),
        Some(CaptureOcrStatus::Failed) => ("!", "SnapLingo — OCR failed".to_string()),
        None => ("", "SnapLingo".to_string()),
    };
    move || {
        MENU_BAR.with_borrow(|bar| {
            if let Some(bar) = bar {
                bar.tray.set_title(Some(title));
                if let Err(error) = bar.tray.set_tooltip(Some(tooltip.as_str())) {
                    log::warn!("Failed to update capture OCR status: {error}");
                }
            }
        });
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

fn create_menu_bar(hotkeys: &HotkeySettingsSnapshot) -> Result<MenuBar, String> {
    let screenshot = menu_item(SCREENSHOT_ID, "Screenshot");
    let translate_selection = menu_item(TRANSLATE_SELECTION_ID, "Translate Selection");
    let screenshot_translate = menu_item(SCREENSHOT_TRANSLATE_ID, "Screenshot Translate");
    let screenshot_ocr = menu_item(SCREENSHOT_OCR_ID, "Screenshot OCR");
    let file_ocr = menu_item(FILE_OCR_ID, "Upload Image OCR");
    let show_translation = menu_item(SHOW_TRANSLATION_ID, "Show Translation Window");
    let history = menu_item(HISTORY_ID, "History");
    let settings = menu_item(SETTINGS_ID, "Settings");
    let about = menu_item(ABOUT_ID, "About SnapLingo");
    let quit = menu_item(QUIT_ID, "Quit SnapLingo");

    let menu = Menu::with_items(&[
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
    ])
    .map_err(|e| e.to_string())?;

    update_menu_shortcuts(&menu, hotkeys)?;

    let pixels = image::load_from_memory(include_bytes!("../icons/trayTemplate.png"))
        .map_err(|error| error.to_string())?
        .to_rgba8();
    let (width, height) = pixels.dimensions();
    let icon =
        Icon::from_rgba(pixels.into_raw(), width, height).map_err(|error| error.to_string())?;
    let tray = TrayIconBuilder::new()
        .with_id(TRAY_ID)
        .with_menu(Box::new(menu.clone()))
        .with_tooltip("SnapLingo")
        .with_menu_on_left_click(true)
        .with_icon(icon)
        .with_icon_as_template(true)
        .build()
        .map_err(|error| error.to_string())?;
    Ok(MenuBar { menu, tray })
}

pub(crate) fn setup_menu_bar(
    hotkeys: &HotkeySettingsSnapshot,
    dispatch: impl Fn(AppAction) + Send + Sync + 'static,
) -> Result<(), String> {
    let bar = create_menu_bar(hotkeys)?;
    *MENU_DISPATCH.lock().unwrap() = Some(Arc::new(dispatch));
    MENU_BAR.with_borrow_mut(|current| *current = Some(bar));
    Ok(())
}

#[cfg(all(test, target_os = "macos"))]
pub(crate) fn test_show_translation_menu_action() {
    use tray_icon::menu::ContextMenu;
    MENU_BAR.with_borrow(|bar| {
        let bar = bar.as_ref().unwrap();
        let menu = unsafe { &*(bar.menu.ns_menu() as *const objc2_app_kit::NSMenu) };
        menu.performActionForItemAtIndex(5);
    });
}

#[cfg(all(test, target_os = "macos"))]
pub(crate) fn test_native_menu_bar() {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSEventModifierFlags, NSMenu};
    use tray_icon::menu::ContextMenu;

    let mtm = MainThreadMarker::new().unwrap();
    let hotkeys = crate::domain::hotkey_config::default_hotkey_snapshot();
    let bar = create_menu_bar(&hotkeys).unwrap();
    let menu = unsafe { &*(bar.menu.ns_menu() as *const NSMenu) };
    assert_eq!(menu.numberOfItems(), 10);
    assert_eq!(
        menu.itemAtIndex(0).unwrap().title().to_string(),
        "Screenshot"
    );
    let translation = menu.itemAtIndex(1).unwrap();
    assert_eq!(translation.keyEquivalent().to_string(), "d");
    assert!(translation
        .keyEquivalentModifierMask()
        .contains(NSEventModifierFlags::Option));

    let mut changed = hotkeys;
    changed
        .screenshot
        .insert("screenshot".into(), "⌃⇧F6".into());
    update_menu_shortcuts(&bar.menu, &changed).unwrap();
    let screenshot = menu.itemAtIndex(0).unwrap();
    assert_eq!(screenshot.keyEquivalent().to_string(), "\u{f709}");
    assert!(screenshot
        .keyEquivalentModifierMask()
        .contains(NSEventModifierFlags::Control | NSEventModifierFlags::Shift));

    let received = Arc::new(Mutex::new(Vec::new()));
    let output = received.clone();
    *MENU_DISPATCH.lock().unwrap() =
        Some(Arc::new(move |action| output.lock().unwrap().push(action)));
    menu.performActionForItemAtIndex(6);
    assert_eq!(*received.lock().unwrap(), vec![AppAction::OpenHistory]);
    let status = bar.tray.ns_status_item().unwrap();
    bar.tray.set_title(Some("OCR…"));
    bar.tray
        .set_tooltip(Some("SnapLingo — Recognizing text…"))
        .unwrap();
    assert_eq!(status.button(mtm).unwrap().title().to_string(), "OCR…");
    assert!(status
        .button(mtm)
        .unwrap()
        .toolTip()
        .unwrap()
        .to_string()
        .contains("Recognizing text"));
    MENU_BAR.with_borrow_mut(|current| *current = Some(bar));
    shutdown_menu_bar();
    assert!(MENU_BAR.with_borrow(Option::is_none));
    assert!(MENU_DISPATCH.lock().unwrap().is_none());
    assert!(NSApplication::sharedApplication(mtm)
        .setActivationPolicy(menu_bar_resting_activation_policy()));
    assert_eq!(
        NSApplication::sharedApplication(mtm).activationPolicy(),
        menu_bar_resting_activation_policy()
    );
}

/// Call on the platform UI thread.
pub(crate) fn refresh_menu_bar_shortcuts(hotkeys: &HotkeySettingsSnapshot) -> Result<(), String> {
    MENU_BAR.with_borrow(|bar| {
        if let Some(bar) = bar {
            update_menu_shortcuts(&bar.menu, hotkeys)
        } else {
            Ok(())
        }
    })
}

#[cfg(target_os = "macos")]
pub(crate) fn refresh_menu_bar_shortcuts_native(hotkeys: HotkeySettingsSnapshot) {
    crate::infrastructure::ui_thread::dispatch(move || {
        if let Err(error) = refresh_menu_bar_shortcuts(&hotkeys) {
            log::warn!("Failed to update native menu shortcuts: {error}");
        }
    });
}

fn update_menu_shortcuts(menu: &Menu, hotkeys: &HotkeySettingsSnapshot) -> Result<(), String> {
    for item in menu.items() {
        if let Some(item) = item.as_menuitem() {
            let accelerator = menu_shortcut(item.id().as_ref(), hotkeys)?
                .map(|accelerator| {
                    accelerator
                        .parse::<Accelerator>()
                        .map_err(|error| error.to_string())
                })
                .transpose()?;
            item.set_accelerator(accelerator)
                .map_err(|error| error.to_string())?;
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

fn menu_item(id: &str, text: &str) -> MenuItem {
    MenuItem::with_id(id, text, true, None)
}

pub(crate) fn should_prevent_implicit_exit(exit_code: Option<i32>) -> bool {
    exit_code.is_none()
}

#[cfg(target_os = "macos")]
fn menu_bar_resting_activation_policy() -> objc2_app_kit::NSApplicationActivationPolicy {
    objc2_app_kit::NSApplicationActivationPolicy::Accessory
}

#[cfg(target_os = "macos")]
pub(crate) fn apply_resting_activation_policy() -> Result<(), String> {
    let update = || {
        let mtm =
            objc2::MainThreadMarker::new().expect("activation policy runs on the main thread");
        let app = objc2_app_kit::NSApplication::sharedApplication(mtm);
        if !app.setActivationPolicy(menu_bar_resting_activation_policy()) {
            log::warn!("Failed to apply native accessory activation policy");
        }
    };
    if objc2::MainThreadMarker::new().is_some() {
        update();
        Ok(())
    } else {
        crate::infrastructure::ui_thread::dispatch(update);
        Ok(())
    }
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn apply_resting_activation_policy() -> Result<(), String> {
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
        let hotkeys =
            snaplingo_core::test_support::reset_hotkey_category(&configuration, OCR_CATEGORY)
                .unwrap();
        assert_eq!(menu_shortcut(FILE_OCR_ID, &hotkeys).unwrap(), None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn menu_bar_shell_uses_accessory_activation_policy() {
        assert!(matches!(
            menu_bar_resting_activation_policy(),
            objc2_app_kit::NSApplicationActivationPolicy::Accessory
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
