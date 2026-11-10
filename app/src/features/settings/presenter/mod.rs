//! Portable Settings sessions, workflow projections and ordered mutations.
//! Composition supplies workflows and native effects; hosts own windows and input.
pub mod capacity;
pub mod capacity_usage;
pub mod confirmation;
pub mod general;
pub mod general_controller;
pub mod library;
pub mod permission_prompt;
pub mod preferences;
pub mod services;
pub mod window;
#[cfg(test)]
mod window_tests;

use snaplingo_core::hotkeys::display_hotkey_to_accelerator;
use snaplingo_core::hotkeys::model as hotkey_config;
use snaplingo_core::hotkeys::model::hotkey_category;
use snaplingo_core::hotkeys::model::HotkeySettingsSnapshot;
use snaplingo_core::hotkeys::model::DEFAULT_HOTKEYS;
use snaplingo_core::hotkeys::model::HOTKEY_UNSET;
use snaplingo_core::hotkeys::model::OCR_CATEGORY;
use snaplingo_core::hotkeys::model::SCREENSHOT_CATEGORY;
use snaplingo_core::hotkeys::model::TRANSLATION_CATEGORY;
use snaplingo_core::settings::model::SettingsSnapshot;
use snaplingo_core::settings::GeneralField as GeneralSetting;
use snaplingo_core::settings::GeneralValue;
use snaplingo_core::settings::OcrField as Field;
use snaplingo_core::settings::ScreenshotField as ScreenshotSetting;
use snaplingo_core::settings::TranslationField as TranslationSetting;
use snaplingo_ui::{
    GeneralField, Hotkey, OcrField, ScreenshotField, Snapshot, Theme, TranslationField, Value,
};
use std::collections::BTreeMap;

const OCR_ACTIONS: [&str; 3] = [
    hotkey_config::SCREENSHOT_OCR_ACTION,
    hotkey_config::SILENT_SCREENSHOT_OCR_ACTION,
    hotkey_config::FILE_OCR_ACTION,
];
const TRANSLATION_ACTIONS: [&str; 3] = [
    hotkey_config::SELECTION_TRANSLATE_ACTION,
    hotkey_config::SCREENSHOT_TRANSLATE_ACTION,
    hotkey_config::SHOW_TRANSLATION_WINDOW_ACTION,
];
const SCREENSHOT_ACTIONS: [&str; 5] = [
    hotkey_config::SCREENSHOT_ACTION,
    hotkey_config::SCREENSHOT_COPY_ACTION,
    hotkey_config::PIN_ACTION,
    hotkey_config::PIN_TOGGLE_ALL_ACTION,
    hotkey_config::PIN_SWITCH_GROUP_ACTION,
];

/// Project real durable values; callers such as the retained app can reuse this
/// conversion without opening a second database or constructing this adapter.
pub fn project_snapshot(
    saved: &SettingsSnapshot,
    hotkeys: &HotkeySettingsSnapshot,
    defaults: &HotkeySettingsSnapshot,
) -> Snapshot {
    let mut snapshot = Snapshot {
        theme: match saved.general.interface_theme.as_str() {
            "studio" => Theme::Studio,
            "paper" => Theme::Paper,
            _ => Theme::Native,
        },
        dark: saved.general.theme == "dark",
        ..Snapshot::default()
    };
    let source = &saved.ocr;
    let source_general = &saved.general;
    snapshot.general = snaplingo_ui::GeneralPreferences {
        interface_theme: source_general.interface_theme.clone(),
        language: source_general.language.clone(),
        appearance: source_general.theme.clone(),
        start_on_boot: source_general.start_on_boot,
        proxy_mode: source_general.proxy_mode.clone(),
        proxy_url: source_general.proxy_url.clone(),
        request_timeout_ms: source_general.request_timeout_ms,
        retry_count: source_general.retry_count.into(),
        log_level: source_general.log_level.clone(),
        log_retention_days: source_general.log_retention_days.into(),
        gpu_composition: source_general.experimental_gpu_acceleration,
        performance_monitoring: source_general.performance_monitoring,
    };
    let screenshot = &saved.screenshot;
    snapshot.screenshot = snaplingo_ui::ScreenshotPreferences {
        save_path: screenshot.save_path.clone(),
        format: screenshot.format.clone(),
        quality: screenshot.quality,
        naming_rule: screenshot.naming_rule.clone(),
        custom_file_name: screenshot.custom_file_name.clone(),
        last_save_directory: screenshot.last_save_directory.clone(),
        last_save_format: screenshot.last_save_format.clone(),
        auto_copy: screenshot.auto_copy,
        default_stroke_width: screenshot.default_stroke_width,
        default_font_size: screenshot.default_font_size,
        remember_last_tool: screenshot.remember_last_tool,
        show_selection_size: screenshot.show_selection_size,
        show_magnifier: screenshot.show_magnifier,
        magnifier_zoom: screenshot.magnifier_zoom,
        pin_opacity: screenshot.pin_opacity,
        pin_shadow: screenshot.pin_shadow,
        pin_border: screenshot.pin_border,
        pin_position: screenshot.pin_position.clone(),
        pin_border_shortcut: screenshot.pin_border_shortcut.clone(),
        longshot_max_height: screenshot.longshot_max_height,
        capture_ui_scale: screenshot.capture_ui_scale.clone(),
        annotation_tool_shortcuts: screenshot.annotation_tool_shortcuts.clone(),
        annotation_colors: screenshot.annotation_colors.clone(),
        selection_border_width: screenshot.selection_border_width,
        selection_border_color: screenshot.selection_border_color,
        selection_mask_color: screenshot.selection_mask_color,
    };
    snapshot.ocr = snaplingo_ui::OcrPreferences {
        gpu_acceleration: source.gpu_acceleration,
        recognition_language: source.recognition_language.clone(),
        preserve_formatting: source.preserve_formatting,
        remove_chinese_spaces: source.remove_chinese_spaces,
        show_confidence: source.show_confidence,
        window_position: source.window_position.clone(),
        hide_silent_status: source.hide_silent_status,
    };
    let source = &saved.translation;
    snapshot.translation = snaplingo_ui::TranslationPreferences {
        default_source_lang: source.default_source_lang.clone(),
        default_target_lang: source.default_target_lang.clone(),
        auto_translate: source.auto_translate,
        auto_copy: source.auto_copy,
        preserve_line_breaks: source.preserve_line_breaks,
        incremental_translation: source.incremental_translation,
        window_always_on_top: source.window_always_on_top,
        hide_on_blur: source.hide_on_blur,
        selection_window_position: source.selection_window_position.clone(),
        input_window_position: source.input_window_position.clone(),
        selection_input_state: source.selection_input_state.clone(),
        screenshot_input_state: source.screenshot_input_state.clone(),
        max_window_height_ratio: source.max_window_height_ratio,
        window_width: source.window_width,
        selection_text_mode: source.selection_text_mode.clone(),
    };
    let rows = |category, actions: [&str; 3]| {
        actions.map(|action| Hotkey {
            value: hotkey_category(hotkeys, category)
                .and_then(|keys| keys.get(action))
                .cloned()
                .unwrap_or(HOTKEY_UNSET.into()),
            default_value: hotkey_category(defaults, category)
                .and_then(|keys| keys.get(action))
                .cloned()
                .unwrap_or(HOTKEY_UNSET.into()),
        })
    };
    snapshot.hotkeys = rows(OCR_CATEGORY, OCR_ACTIONS);
    snapshot.translation_hotkeys = rows(TRANSLATION_CATEGORY, TRANSLATION_ACTIONS);
    snapshot.screenshot_hotkeys = SCREENSHOT_ACTIONS.map(|action| Hotkey {
        value: hotkey_category(hotkeys, SCREENSHOT_CATEGORY)
            .and_then(|keys| keys.get(action))
            .cloned()
            .unwrap_or(HOTKEY_UNSET.into()),
        default_value: hotkey_category(defaults, SCREENSHOT_CATEGORY)
            .and_then(|keys| keys.get(action))
            .cloned()
            .unwrap_or(HOTKEY_UNSET.into()),
    });
    snapshot.capacity.retention_days = saved.history.retention_days.into();
    snapshot.capacity.history_limit = saved.history.maximum_records;
    snapshot.capacity.favorites_limit = saved.history.maximum_favorites;
    snapshot
}

pub fn validate_hotkey_conflicts(snapshot: &HotkeySettingsSnapshot) -> Result<(), String> {
    let mut owners = BTreeMap::new();
    for binding in DEFAULT_HOTKEYS {
        let Some(shortcut) =
            hotkey_category(snapshot, binding.category).and_then(|keys| keys.get(binding.action))
        else {
            continue;
        };
        if let Some(accelerator) =
            display_hotkey_to_accelerator(shortcut).map_err(|error| error.to_string())?
        {
            let owner = format!("{}:{}", binding.category, binding.action);
            let mut keys = accelerator
                .split('+')
                .map(|part| match part {
                    "CmdOrCtrl" if cfg!(target_os = "macos") => "Meta",
                    "CmdOrCtrl" => "Ctrl",
                    other => other,
                })
                .collect::<Vec<_>>();
            keys.sort_unstable();
            keys.dedup();
            if let Some(previous) = owners.insert(keys.join("+"), owner.clone()) {
                return Err(format!("快捷键冲突：{previous} 与 {owner} 使用相同快捷键"));
            }
        }
    }
    Ok(())
}

pub fn general_value(value: Value) -> GeneralValue {
    match value {
        Value::Toggle(value) => GeneralValue::Toggle(value),
        Value::Text(value) => GeneralValue::Text(value),
        Value::Number(value) => GeneralValue::Number(value),
    }
}

pub fn general_field(field: GeneralField) -> GeneralSetting {
    match field {
        GeneralField::InterfaceTheme => GeneralSetting::InterfaceTheme,
        GeneralField::Language => GeneralSetting::Language,
        GeneralField::Appearance => GeneralSetting::Appearance,
        GeneralField::StartOnBoot => GeneralSetting::StartOnBoot,
        GeneralField::ProxyMode => GeneralSetting::ProxyMode,
        GeneralField::ProxyUrl => GeneralSetting::ProxyUrl,
        GeneralField::RequestTimeout => GeneralSetting::RequestTimeout,
        GeneralField::RetryCount => GeneralSetting::RetryCount,
        GeneralField::LogLevel => GeneralSetting::LogLevel,
        GeneralField::LogRetention => GeneralSetting::LogRetention,
        GeneralField::GpuComposition => GeneralSetting::GpuComposition,
        GeneralField::PerformanceMonitoring => GeneralSetting::PerformanceMonitoring,
    }
}

pub fn ocr_field(field: OcrField) -> Field {
    match field {
        OcrField::GpuAcceleration => Field::GpuAcceleration,
        OcrField::RecognitionLanguage => Field::RecognitionLanguage,
        OcrField::PreserveFormatting => Field::PreserveFormatting,
        OcrField::RemoveChineseSpaces => Field::RemoveChineseSpaces,
        OcrField::ShowConfidence => Field::ShowConfidence,
        OcrField::WindowPosition => Field::WindowPosition,
        OcrField::HideSilentStatus => Field::HideSilentStatus,
    }
}

pub fn screenshot_field(field: ScreenshotField) -> ScreenshotSetting {
    match field {
        ScreenshotField::SavePath => ScreenshotSetting::SavePath,
        ScreenshotField::Format => ScreenshotSetting::Format,
        ScreenshotField::Quality => ScreenshotSetting::Quality,
        ScreenshotField::NamingRule => ScreenshotSetting::NamingRule,
        ScreenshotField::CustomFileName => ScreenshotSetting::CustomFileName,
        ScreenshotField::AutoCopy => ScreenshotSetting::AutoCopy,
        ScreenshotField::LongshotMaxHeight => ScreenshotSetting::LongshotMaxHeight,
        ScreenshotField::SelectionBorderWidth => ScreenshotSetting::SelectionBorderWidth,
        ScreenshotField::SelectionBorderColor => ScreenshotSetting::SelectionBorderColor,
        ScreenshotField::SelectionMaskColor => ScreenshotSetting::SelectionMaskColor,
        ScreenshotField::MaskOpacity => ScreenshotSetting::MaskOpacity,
        ScreenshotField::RememberLastTool => ScreenshotSetting::RememberLastTool,
        ScreenshotField::ShowSelectionSize => ScreenshotSetting::ShowSelectionSize,
        ScreenshotField::ShowMagnifier => ScreenshotSetting::ShowMagnifier,
        ScreenshotField::MagnifierZoom => ScreenshotSetting::MagnifierZoom,
        ScreenshotField::PinOpacity => ScreenshotSetting::PinOpacity,
        ScreenshotField::PinShadow => ScreenshotSetting::PinShadow,
        ScreenshotField::PinPosition => ScreenshotSetting::PinPosition,
        ScreenshotField::PinBorder => ScreenshotSetting::PinBorder,
        ScreenshotField::CaptureUiScale => ScreenshotSetting::CaptureUiScale,
        ScreenshotField::AnnotationShortcut(tool) => ScreenshotSetting::AnnotationShortcut(tool),
        ScreenshotField::ResetAnnotationShortcuts => ScreenshotSetting::ResetAnnotationShortcuts,
        ScreenshotField::PinBorderShortcut => ScreenshotSetting::PinBorderShortcut,
    }
}

pub fn translation_field(field: TranslationField) -> TranslationSetting {
    match field {
        TranslationField::DefaultSourceLang => TranslationSetting::DefaultSourceLang,
        TranslationField::DefaultTargetLang => TranslationSetting::DefaultTargetLang,
        TranslationField::SelectionTextMode => TranslationSetting::SelectionTextMode,
        TranslationField::SelectionWindowPosition => TranslationSetting::SelectionWindowPosition,
        TranslationField::InputWindowPosition => TranslationSetting::InputWindowPosition,
        TranslationField::MaxWindowHeightRatio => TranslationSetting::MaxWindowHeightRatio,
        TranslationField::WindowWidth => TranslationSetting::WindowWidth,
        TranslationField::SelectionInputState => TranslationSetting::SelectionInputState,
        TranslationField::ScreenshotInputState => TranslationSetting::ScreenshotInputState,
        TranslationField::AutoTranslate => TranslationSetting::AutoTranslate,
        TranslationField::AutoCopy => TranslationSetting::AutoCopy,
        TranslationField::PreserveLineBreaks => TranslationSetting::PreserveLineBreaks,
        TranslationField::IncrementalTranslation => TranslationSetting::IncrementalTranslation,
        TranslationField::WindowAlwaysOnTop => TranslationSetting::WindowAlwaysOnTop,
        TranslationField::HideOnBlur => TranslationSetting::HideOnBlur,
    }
}
