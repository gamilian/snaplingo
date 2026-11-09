use crate::Value;
use std::collections::HashMap;

pub const ANNOTATION_SHORTCUT_LABELS: &[(&str, &str)] = &[
    ("rectangle", "矩形"),
    ("ellipse", "椭圆"),
    ("arrow", "箭头"),
    ("line", "直线"),
    ("pen", "画笔"),
    ("highlight", "荧光笔"),
    ("region-highlight", "区域高亮"),
    ("blur", "模糊"),
    ("spotlight", "聚光灯"),
    ("mosaic", "马赛克"),
    ("text", "文字"),
    ("eraser", "橡皮擦"),
    ("scrolling", "长截图"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenshotField {
    SavePath,
    Format,
    Quality,
    NamingRule,
    CustomFileName,
    AutoCopy,
    LongshotMaxHeight,
    SelectionBorderWidth,
    SelectionBorderColor,
    SelectionMaskColor,
    MaskOpacity,
    RememberLastTool,
    ShowSelectionSize,
    ShowMagnifier,
    MagnifierZoom,
    PinOpacity,
    PinShadow,
    PinPosition,
    PinBorder,
    CaptureUiScale,
    AnnotationShortcut(&'static str),
    ResetAnnotationShortcuts,
    PinBorderShortcut,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenshotPreferences {
    pub save_path: String,
    pub format: String,
    pub quality: u8,
    pub naming_rule: String,
    pub custom_file_name: String,
    pub last_save_directory: Option<String>,
    pub last_save_format: Option<String>,
    pub auto_copy: bool,
    pub default_stroke_width: u8,
    pub default_font_size: u8,
    pub remember_last_tool: bool,
    pub show_selection_size: bool,
    pub show_magnifier: bool,
    pub magnifier_zoom: u8,
    pub pin_opacity: u8,
    pub pin_shadow: bool,
    pub pin_border: bool,
    pub pin_position: String,
    pub pin_border_shortcut: String,
    pub longshot_max_height: u32,
    pub capture_ui_scale: String,
    pub annotation_tool_shortcuts: HashMap<String, String>,
    pub annotation_colors: Vec<[u8; 4]>,
    pub selection_border_width: u8,
    pub selection_border_color: [u8; 4],
    pub selection_mask_color: [u8; 4],
}

impl Default for ScreenshotPreferences {
    fn default() -> Self {
        Self {
            save_path: String::new(),
            format: "png".to_string(),
            quality: 90,
            naming_rule: "timestamp".to_string(),
            custom_file_name: "SnapLingo".to_string(),
            last_save_directory: None,
            last_save_format: None,
            auto_copy: false,
            default_stroke_width: 2,
            default_font_size: 24,
            remember_last_tool: true,
            show_selection_size: true,
            show_magnifier: false,
            magnifier_zoom: 12,
            pin_opacity: 100,
            pin_shadow: true,
            pin_border: false,
            pin_position: "original".to_string(),
            pin_border_shortcut: "b".to_string(),
            longshot_max_height: 30_000,
            capture_ui_scale: "auto".to_string(),
            annotation_tool_shortcuts: default_annotation_tool_shortcuts(),
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

impl ScreenshotField {
    pub fn label(self) -> &'static str {
        match self {
            Self::SavePath => "默认保存路径",
            Self::Format => "图片格式",
            Self::Quality => "JPEG 图片质量",
            Self::NamingRule => "文件命名规则",
            Self::CustomFileName => "自定义文件名前缀",
            Self::AutoCopy => "截图后自动复制",
            Self::LongshotMaxHeight => "长截图最大高度",
            Self::SelectionBorderWidth => "边框宽度",
            Self::SelectionBorderColor => "边框颜色",
            Self::SelectionMaskColor => "遮罩颜色",
            Self::MaskOpacity => "遮罩透明度",
            Self::RememberLastTool => "记住上次标注工具",
            Self::ShowSelectionSize => "显示选区尺寸",
            Self::ShowMagnifier => "显示放大镜",
            Self::MagnifierZoom => "放大镜倍率",
            Self::PinOpacity => "贴图默认透明度",
            Self::PinShadow => "贴图显示阴影",
            Self::PinPosition => "贴图初始位置",
            Self::PinBorder => "显示贴图边框",
            Self::CaptureUiScale => "界面缩放",
            Self::PinBorderShortcut => "贴图边框快捷键",
            Self::ResetAnnotationShortcuts => "恢复标注快捷键默认值",
            Self::AnnotationShortcut(tool) => ANNOTATION_SHORTCUT_LABELS
                .iter()
                .find(|(id, _)| *id == tool)
                .map(|(_, label)| *label)
                .unwrap_or(tool),
        }
    }
    pub fn description(self) -> Option<&'static str> {
        match self {
            Self::Format => Some("快捷保存使用此格式；另存为记住上次目录和格式。"),
            Self::LongshotMaxHeight => {
                Some("单位：像素。到达上限后自动复制并保存 PNG；同时受总像素上限保护。")
            }
            Self::RememberLastTool => Some("本次运行内沿用上次工具；关闭后每次从矩形开始。"),
            Self::CaptureUiScale => Some("应用于设置、截图工具、长截图控件与光标。"),
            _ => None,
        }
    }
    pub fn choices(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Self::Format => &[
                ("png", "PNG（无损）"),
                ("jpg", "JPG"),
                ("webp", "WebP（无损）"),
            ],
            Self::NamingRule => &[
                ("timestamp", "日期与时间"),
                ("unix-ms", "Unix 毫秒时间戳"),
                ("date", "日期"),
                ("counter", "计数器"),
                ("custom", "自定义"),
            ],
            Self::PinPosition => &[("original", "截图原位置"), ("center", "桌面中央（级联）")],
            Self::CaptureUiScale => &[
                ("auto", "自动（跟随显示器）"),
                ("100", "100%"),
                ("125", "125%"),
                ("150", "150%"),
                ("175", "175%"),
                ("200", "200%"),
            ],
            _ => &[],
        }
    }
    pub fn range(self) -> Option<(u32, u32, u32, &'static str)> {
        match self {
            Self::Quality => Some((50, 100, 1, "%")),
            Self::SelectionBorderWidth => Some((1, 8, 1, "px")),
            Self::MaskOpacity => Some((0, 90, 1, "%")),
            Self::MagnifierZoom => Some((4, 20, 1, "×")),
            Self::PinOpacity => Some((5, 100, 1, "%")),
            _ => None,
        }
    }
    pub fn colors(self) -> &'static [[u8; 4]] {
        match self {
            Self::SelectionBorderColor => &[
                [91, 127, 255, 242],
                [255, 77, 79, 242],
                [40, 167, 69, 242],
                [250, 219, 20, 242],
                [255, 255, 255, 242],
                [0, 0, 0, 242],
            ],
            Self::SelectionMaskColor => &[
                [0, 0, 0, 46],
                [32, 36, 44, 72],
                [62, 70, 84, 72],
                [20, 55, 92, 64],
                [25, 82, 72, 64],
                [88, 54, 112, 64],
                [112, 68, 44, 64],
                [255, 255, 255, 72],
            ],
            _ => &[],
        }
    }
    pub fn value(self, s: &ScreenshotPreferences) -> Value {
        use Value::*;
        match self {
            Self::SavePath => Text(s.save_path.clone()),
            Self::Format => Text(s.format.clone()),
            Self::Quality => Number(s.quality.into()),
            Self::NamingRule => Text(s.naming_rule.clone()),
            Self::CustomFileName => Text(s.custom_file_name.clone()),
            Self::AutoCopy => Toggle(s.auto_copy),
            Self::LongshotMaxHeight => Number(s.longshot_max_height),
            Self::SelectionBorderWidth => Number(s.selection_border_width.into()),
            Self::SelectionBorderColor => Number(u32::from_be_bytes(s.selection_border_color)),
            Self::SelectionMaskColor => Number(u32::from_be_bytes(s.selection_mask_color)),
            Self::MaskOpacity => {
                Number((f64::from(s.selection_mask_color[3]) * 100.0 / 255.0).round() as u32)
            }
            Self::RememberLastTool => Toggle(s.remember_last_tool),
            Self::ShowSelectionSize => Toggle(s.show_selection_size),
            Self::ShowMagnifier => Toggle(s.show_magnifier),
            Self::MagnifierZoom => Number(s.magnifier_zoom.into()),
            Self::PinOpacity => Number(s.pin_opacity.into()),
            Self::PinShadow => Toggle(s.pin_shadow),
            Self::PinPosition => Text(s.pin_position.clone()),
            Self::PinBorder => Toggle(s.pin_border),
            Self::CaptureUiScale => Text(s.capture_ui_scale.clone()),
            Self::PinBorderShortcut => Text(s.pin_border_shortcut.clone()),
            Self::AnnotationShortcut(tool) => Text(
                s.annotation_tool_shortcuts
                    .get(tool)
                    .cloned()
                    .unwrap_or_default(),
            ),
            Self::ResetAnnotationShortcuts => Toggle(false),
        }
    }
    pub(crate) fn apply(self, s: &mut ScreenshotPreferences, value: Value) -> Result<(), String> {
        use Value::*;
        if let Some((min, max, _, _)) = self.range() {
            if !matches!(value,Number(n) if (min..=max).contains(&n)) {
                return Err("截图设置数值超出范围".into());
            }
        }
        if !self.choices().is_empty()
            && !matches!(&value,Text(v) if self.choices().iter().any(|(id,_)| *id == v))
        {
            return Err("未知的截图设置选项".into());
        }
        match (self, value) {
            (Self::SavePath, Text(v)) => s.save_path = v,
            (Self::Format, Text(v)) => s.format = v,
            (Self::Quality, Number(v)) => s.quality = v as u8,
            (Self::NamingRule, Text(v)) => s.naming_rule = v,
            (Self::CustomFileName, Text(v)) => s.custom_file_name = v,
            (Self::AutoCopy, Toggle(v)) => s.auto_copy = v,
            (Self::LongshotMaxHeight, Number(v)) if (100..=100_000).contains(&v) => {
                s.longshot_max_height = v
            }
            (Self::SelectionBorderWidth, Number(v)) => s.selection_border_width = v as u8,
            (Self::SelectionBorderColor, Number(v)) => s.selection_border_color = v.to_be_bytes(),
            (Self::SelectionMaskColor, Number(v)) => s.selection_mask_color = v.to_be_bytes(),
            (Self::MaskOpacity, Number(v)) => {
                s.selection_mask_color[3] = (v as f64 * 255.0 / 100.0).round() as u8
            }
            (Self::RememberLastTool, Toggle(v)) => s.remember_last_tool = v,
            (Self::ShowSelectionSize, Toggle(v)) => s.show_selection_size = v,
            (Self::ShowMagnifier, Toggle(v)) => s.show_magnifier = v,
            (Self::MagnifierZoom, Number(v)) => s.magnifier_zoom = v as u8,
            (Self::PinOpacity, Number(v)) => s.pin_opacity = v as u8,
            (Self::PinShadow, Toggle(v)) => s.pin_shadow = v,
            (Self::PinPosition, Text(v)) => s.pin_position = v,
            (Self::PinBorder, Toggle(v)) => s.pin_border = v,
            (Self::CaptureUiScale, Text(v)) => s.capture_ui_scale = v,
            (Self::AnnotationShortcut(tool), Text(v))
                if ANNOTATION_SHORTCUT_LABELS.iter().any(|(id, _)| *id == tool) =>
            {
                s.annotation_tool_shortcuts.insert(tool.into(), v);
            }
            (Self::ResetAnnotationShortcuts, Toggle(true)) => {
                s.annotation_tool_shortcuts = default_annotation_tool_shortcuts()
            }
            (Self::PinBorderShortcut, Text(v)) => s.pin_border_shortcut = v,
            _ => return Err("Setting value does not match its control".into()),
        }
        Ok(())
    }
}

fn default_annotation_tool_shortcuts() -> HashMap<String, String> {
    [
        ("rectangle", "b"),
        ("ellipse", "o"),
        ("arrow", "a"),
        ("line", "l"),
        ("pen", "p"),
        ("highlight", "h"),
        ("region-highlight", "q"),
        ("blur", "u"),
        ("spotlight", "g"),
        ("mosaic", "m"),
        ("text", "t"),
        ("eraser", "e"),
        ("scrolling", "s"),
    ]
    .into_iter()
    .map(|(tool, key)| (tool.into(), key.into()))
    .collect()
}

pub(crate) fn valid_shortcuts(s: &ScreenshotPreferences) -> bool {
    let defaults = default_annotation_tool_shortcuts();
    let mut used = std::collections::HashSet::new();
    for (tool, key) in &s.annotation_tool_shortcuts {
        if !defaults.contains_key(tool) {
            return false;
        }
        let key = key.trim().to_ascii_lowercase();
        if key.is_empty() {
            continue;
        }
        if !valid_main_key(&key)
            || ["c", "f", "r", "f3"].contains(&key.as_str())
            || !used.insert(key)
        {
            return false;
        }
    }
    valid_local_chord(&s.pin_border_shortcut)
}
fn valid_main_key(key: &str) -> bool {
    (key.len() == 1 && key.as_bytes()[0].is_ascii_alphanumeric())
        || key
            .strip_prefix('f')
            .and_then(|s| s.parse::<u8>().ok())
            .is_some_and(|n| (1..=24).contains(&n) && key == format!("f{n}"))
}
pub(crate) fn valid_local_chord(value: &str) -> bool {
    if value.is_empty() {
        return true;
    }
    let parts = value.split('+').collect::<Vec<_>>();
    let (key, modifiers) = parts.split_last().unwrap();
    let mut used = std::collections::HashSet::new();
    valid_main_key(key)
        && modifiers
            .iter()
            .all(|m| ["ctrl", "meta", "alt", "shift"].contains(m) && used.insert(m))
}
