use super::GeneralValue;
use crate::domain::ScreenshotSettings;

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
    pub fn value(self, s: &ScreenshotSettings) -> GeneralValue {
        use GeneralValue::*;
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
    pub(super) fn apply(
        self,
        s: &mut ScreenshotSettings,
        value: GeneralValue,
    ) -> crate::Result<()> {
        use GeneralValue::*;
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
                s.annotation_tool_shortcuts =
                    crate::domain::config::default_annotation_tool_shortcuts()
            }
            (Self::PinBorderShortcut, Text(v)) => s.pin_border_shortcut = v,
            _ => return Err("Setting value does not match its control".into()),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::settings::{
        SettingsChangeNotifier, SettingsConfiguration, SettingsStore,
    };
    use crate::domain::SettingsSnapshot;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct Store(Mutex<SettingsSnapshot>);
    impl SettingsStore for Store {
        fn load_settings(&self) -> crate::Result<SettingsSnapshot> {
            Ok(self.0.lock().unwrap().clone())
        }
        fn save_settings(&self, snapshot: &SettingsSnapshot) -> crate::Result<()> {
            *self.0.lock().unwrap() = snapshot.clone();
            Ok(())
        }
    }
    #[derive(Default)]
    struct Changes(AtomicUsize);
    impl SettingsChangeNotifier for Changes {
        fn settings_changed(&self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    fn fixture() -> (Arc<SettingsConfiguration>, Arc<Changes>) {
        let changes = Arc::new(Changes::default());
        (
            Arc::new(SettingsConfiguration::with_change_notifier(
                Arc::new(Store::default()),
                changes.clone(),
            )),
            changes,
        )
    }
    #[test]
    fn color_and_opacity_edits_preserve_each_other_and_save_dialog_choices() {
        use GeneralValue::*;
        let (configuration, _) = fixture();
        configuration
            .record_screenshot_save_choice(std::path::Path::new("/chosen/region.webp"), "webp")
            .unwrap();
        configuration
            .edit_screenshot(
                ScreenshotField::SelectionMaskColor,
                Number(u32::from_be_bytes([25, 82, 72, 64])),
            )
            .unwrap();
        let snapshot = configuration
            .edit_screenshot(ScreenshotField::MaskOpacity, Number(67))
            .unwrap();
        assert_eq!(snapshot.screenshot.selection_mask_color, [25, 82, 72, 171]);
        assert_eq!(
            ScreenshotField::MaskOpacity.value(&snapshot.screenshot),
            Number(67)
        );
        let snapshot = configuration
            .edit_screenshot(
                ScreenshotField::SelectionBorderColor,
                Number(u32::from_be_bytes([1, 2, 3, 242])),
            )
            .unwrap();
        assert_eq!(snapshot.screenshot.selection_border_color, [1, 2, 3, 242]);
        assert_eq!(snapshot.screenshot.selection_mask_color, [25, 82, 72, 171]);
        assert_eq!(
            snapshot.screenshot.last_save_directory.as_deref(),
            Some("/chosen")
        );
        assert_eq!(
            snapshot.screenshot.last_save_format.as_deref(),
            Some("webp")
        );
    }
    #[test]
    fn local_shortcut_validation_is_atomic_and_supports_clear_and_restore() {
        use GeneralValue::*;
        let (configuration, changes) = fixture();
        let before = configuration.snapshot().unwrap();
        for (field, value) in [
            (
                ScreenshotField::AnnotationShortcut("arrow"),
                Text("f3".into()),
            ),
            (
                ScreenshotField::AnnotationShortcut("arrow"),
                Text("ctrl+x".into()),
            ),
            (
                ScreenshotField::AnnotationShortcut("arrow"),
                Text("t".into()),
            ),
            (
                ScreenshotField::AnnotationShortcut("not-a-tool"),
                Text("z".into()),
            ),
            (
                ScreenshotField::PinBorderShortcut,
                Text("ctrl+ctrl+b".into()),
            ),
        ] {
            assert!(configuration.edit_screenshot(field, value).is_err());
        }
        assert_eq!(configuration.snapshot().unwrap(), before);
        assert_eq!(changes.0.load(Ordering::SeqCst), 0);
        let cleared = configuration
            .edit_screenshot(
                ScreenshotField::AnnotationShortcut("arrow"),
                Text(String::new()),
            )
            .unwrap();
        assert_eq!(cleared.screenshot.annotation_tool_shortcuts["arrow"], "");
        let custom = configuration
            .edit_screenshot(
                ScreenshotField::AnnotationShortcut("arrow"),
                Text(" F24 ".into()),
            )
            .unwrap();
        assert_eq!(custom.screenshot.annotation_tool_shortcuts["arrow"], "f24");
        let pin = configuration
            .edit_screenshot(
                ScreenshotField::PinBorderShortcut,
                Text("SHIFT+CTRL+F4".into()),
            )
            .unwrap();
        assert_eq!(pin.screenshot.pin_border_shortcut, "ctrl+shift+f4");
        let restored = configuration
            .edit_screenshot(ScreenshotField::ResetAnnotationShortcuts, Toggle(true))
            .unwrap();
        assert_eq!(
            restored.screenshot.annotation_tool_shortcuts,
            crate::domain::config::default_annotation_tool_shortcuts()
        );
        assert_eq!(restored.screenshot.pin_border_shortcut, "ctrl+shift+f4");
    }
    #[test]
    fn invalid_native_controls_do_not_write_and_unchanged_controls_do_not_notify() {
        use GeneralValue::*;
        let (configuration, changes) = fixture();
        let before = configuration.snapshot().unwrap();
        for (field, value) in [
            (ScreenshotField::Format, Text("gif".into())),
            (ScreenshotField::Quality, Number(49)),
            (ScreenshotField::MaskOpacity, Number(91)),
            (ScreenshotField::PinOpacity, Number(0)),
            (ScreenshotField::LongshotMaxHeight, Number(99)),
            (ScreenshotField::LongshotMaxHeight, Number(100_001)),
            (ScreenshotField::CaptureUiScale, Text("250".into())),
            (ScreenshotField::ShowMagnifier, Text("true".into())),
        ] {
            assert!(configuration.edit_screenshot(field, value).is_err());
        }
        configuration
            .edit_screenshot(
                ScreenshotField::AutoCopy,
                Toggle(before.screenshot.auto_copy),
            )
            .unwrap();
        configuration
            .edit_screenshot(
                ScreenshotField::PinOpacity,
                Number(before.screenshot.pin_opacity.into()),
            )
            .unwrap();
        assert_eq!(configuration.snapshot().unwrap(), before);
        assert_eq!(changes.0.load(Ordering::SeqCst), 0);
    }
    #[test]
    fn concurrent_screenshot_controls_preserve_output_choices() {
        use GeneralValue::*;
        let (configuration, _) = fixture();
        let start = Arc::new(std::sync::Barrier::new(3));
        let mut workers = Vec::new();
        for task in 0..3 {
            let configuration = configuration.clone();
            let start = start.clone();
            workers.push(std::thread::spawn(move || {
                for index in 0..30 {
                    start.wait();
                    match task {
                        0 => {
                            configuration
                                .edit_screenshot(ScreenshotField::PinOpacity, Number(50 + index))
                                .unwrap();
                        }
                        1 => {
                            configuration
                                .edit_screenshot(
                                    ScreenshotField::LongshotMaxHeight,
                                    Number(1000 + index * 100),
                                )
                                .unwrap();
                        }
                        _ => {
                            configuration
                                .record_screenshot_save_choice(
                                    std::path::Path::new(&format!("/chosen/{index}/region.webp")),
                                    "webp",
                                )
                                .unwrap();
                        }
                    }
                    start.wait();
                    if task == 0 {
                        let s = configuration.snapshot().unwrap().screenshot;
                        assert_eq!(s.pin_opacity, 50 + index as u8);
                        assert_eq!(s.longshot_max_height, 1000 + index * 100);
                        assert_eq!(s.last_save_directory, Some(format!("/chosen/{index}")));
                    }
                    start.wait();
                }
            }));
        }
        for worker in workers {
            worker.join().unwrap();
        }
    }
}
