use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct GeneralSettings {
    pub language: String,
    pub theme: String,
    pub start_on_boot: bool,
    pub proxy_mode: String,
    pub proxy_url: String,
    pub request_timeout_ms: u32,
    pub retry_count: u8,
    pub log_level: String,
    pub log_retention_days: u16,
    pub performance_monitoring: bool,
    pub experimental_gpu_acceleration: bool,
    pub system_tts_voice: String,
    pub system_tts_rate: u16,
    pub settings_window_x: Option<i32>,
    pub settings_window_y: Option<i32>,
    pub settings_window_width: Option<u32>,
    pub settings_window_height: Option<u32>,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            language: "zh-CN".to_string(),
            theme: "system".to_string(),
            start_on_boot: false,
            proxy_mode: "system".to_string(),
            proxy_url: String::new(),
            request_timeout_ms: 10_000,
            retry_count: 1,
            log_level: "info".to_string(),
            log_retention_days: 7,
            performance_monitoring: false,
            experimental_gpu_acceleration: false,
            system_tts_voice: String::new(),
            system_tts_rate: 180,
            settings_window_x: None,
            settings_window_y: None,
            settings_window_width: None,
            settings_window_height: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ScreenshotSettings {
    pub save_path: String,
    pub format: String,
    pub quality: u8,
    pub naming_rule: String,
    pub custom_file_name: String,
    pub auto_copy: bool,
    pub default_stroke_width: u8,
    pub default_font_size: u8,
    pub remember_last_tool: bool,
    pub show_selection_size: bool,
    pub show_magnifier: bool,
    pub magnifier_zoom: u8,
    pub pin_opacity: u8,
    pub pin_shadow: bool,
    pub annotation_colors: Vec<[u8; 4]>,
    pub selection_border_width: u8,
    pub selection_border_color: [u8; 4],
    pub selection_mask_color: [u8; 4],
}

impl Default for ScreenshotSettings {
    fn default() -> Self {
        Self {
            save_path: String::new(),
            format: "png".to_string(),
            quality: 90,
            naming_rule: "timestamp".to_string(),
            custom_file_name: "SnapLingo".to_string(),
            auto_copy: false,
            default_stroke_width: 2,
            default_font_size: 24,
            remember_last_tool: true,
            show_selection_size: true,
            show_magnifier: false,
            magnifier_zoom: 12,
            pin_opacity: 100,
            pin_shadow: true,
            annotation_colors: vec![
                [255, 77, 79, 255],
                [40, 167, 69, 255],
                [24, 144, 255, 255],
                [250, 219, 20, 255],
                [255, 255, 255, 255],
                [0, 0, 0, 255],
            ],
            selection_border_width: 2,
            selection_border_color: [91, 127, 255, 242],
            selection_mask_color: [0, 0, 0, 46],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct TranslationSettings {
    pub default_source_lang: String,
    pub default_target_lang: String,
    pub auto_translate: bool,
    pub auto_copy: bool,
    pub preserve_line_breaks: bool,
    pub incremental_translation: bool,
    pub window_always_on_top: bool,
    pub hide_on_blur: bool,
    pub selection_window_position: String,
    pub input_window_position: String,
    pub selection_input_state: String,
    pub screenshot_input_state: String,
    pub max_window_height_ratio: u8,
    pub window_width: u16,
    pub selection_text_mode: String,
}

impl Default for TranslationSettings {
    fn default() -> Self {
        Self {
            default_source_lang: "auto".to_string(),
            default_target_lang: "zh-CN".to_string(),
            auto_translate: true,
            auto_copy: false,
            preserve_line_breaks: true,
            incremental_translation: false,
            window_always_on_top: true,
            hide_on_blur: true,
            selection_window_position: "below-cursor".to_string(),
            input_window_position: "center".to_string(),
            selection_input_state: "last".to_string(),
            screenshot_input_state: "last".to_string(),
            max_window_height_ratio: 70,
            window_width: 660,
            selection_text_mode: "smart".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct OcrSettings {
    pub recognition_language: String,
    pub preserve_formatting: bool,
    pub remove_chinese_spaces: bool,
    pub show_confidence: bool,
    pub window_position: String,
    pub hide_silent_status: bool,
}

impl Default for OcrSettings {
    fn default() -> Self {
        Self {
            recognition_language: "auto".to_string(),
            preserve_formatting: true,
            remove_chinese_spaces: true,
            show_confidence: false,
            window_position: "cursor".to_string(),
            hide_silent_status: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct HistorySettings {
    pub auto_cleanup_enabled: bool,
    pub retention_days: u32,
    pub maximum_records: u32,
    pub maximum_favorites: u32,
}

impl Default for HistorySettings {
    fn default() -> Self {
        Self {
            auto_cleanup_enabled: false,
            retention_days: 30,
            maximum_records: 5000,
            maximum_favorites: 1000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct SettingsSnapshot {
    pub general: GeneralSettings,
    pub screenshot: ScreenshotSettings,
    pub translation: TranslationSettings,
    pub ocr: OcrSettings,
    pub history: HistorySettings,
}

#[cfg(test)]
mod tests {
    use super::SettingsSnapshot;

    #[test]
    fn legacy_settings_documents_receive_defaults_for_new_fields() {
        let snapshot: SettingsSnapshot = serde_json::from_value(serde_json::json!({
            "screenshot": { "quality": 80 },
            "translation": {},
            "ocr": {}
        }))
        .unwrap();

        assert_eq!(snapshot.screenshot.quality, 80);
        assert_eq!(snapshot.screenshot.selection_border_width, 2);
        assert_eq!(snapshot.screenshot.magnifier_zoom, 12);
        assert_eq!(
            snapshot.screenshot.selection_border_color,
            [91, 127, 255, 242]
        );
        assert_eq!(snapshot.screenshot.selection_mask_color, [0, 0, 0, 46]);
        assert_eq!(
            snapshot.translation.selection_window_position,
            "below-cursor"
        );
        assert_eq!(snapshot.translation.window_width, 660);
        assert_eq!(snapshot.ocr.window_position, "cursor");
        assert!(!snapshot.ocr.hide_silent_status);
    }
}
