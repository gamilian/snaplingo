use super::r;
use crate::*;
pub(crate) fn fields(section: Section, s: &ScreenshotPreferences) -> Vec<ScreenshotField> {
    use ScreenshotField::*;
    match section {
        Section::Output => {
            let mut fields = vec![SavePath, Format];
            if s.format == "jpg" {
                fields.push(Quality);
            }
            fields.push(NamingRule);
            if s.naming_rule == "custom" {
                fields.push(CustomFileName);
            }
            fields.extend([AutoCopy, LongshotMaxHeight]);
            fields
        }
        Section::CaptureInterface => vec![
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
        ],
        _ => vec![],
    }
}
pub(crate) fn field_order(field: ScreenshotField) -> usize {
    use ScreenshotField::*;
    match field {
        AnnotationShortcut(tool) => ANNOTATION_SHORTCUT_LABELS
            .iter()
            .position(|(id, _)| *id == tool)
            .unwrap_or(0),
        ResetAnnotationShortcuts => 14,
        PinBorderShortcut => 15,
        SavePath => 20,
        Format => 21,
        Quality => 22,
        NamingRule => 23,
        CustomFileName => 24,
        AutoCopy => 25,
        LongshotMaxHeight => 26,
        SelectionBorderWidth => 30,
        SelectionBorderColor => 31,
        SelectionMaskColor => 32,
        MaskOpacity => 33,
        RememberLastTool => 34,
        ShowSelectionSize => 35,
        ShowMagnifier => 36,
        MagnifierZoom => 37,
        PinOpacity => 38,
        PinShadow => 39,
        PinPosition => 40,
        PinBorder => 41,
        CaptureUiScale => 42,
    }
}

// The annotation panel has 16px vertical insets, two 15px help lines,
// 44px shortcut rows, a 16px gap and a 32px restore button.
fn annotation_height(width: f32) -> f32 {
    let rows = if width < 768. { 13. } else { 7. };
    rows * 44. + 114.
}
fn pin_height(width: f32, theme: Theme) -> f32 {
    let padding = match theme {
        Theme::Paper => 16.,
        Theme::Studio => 9.,
        _ => 13.,
    };
    padding * 2. + 37. + if width <= 700. { 46. } else { 0. }
}
pub(crate) fn local_height(width: f32, theme: Theme) -> f32 {
    annotation_height(width)
        + pin_height(width, theme)
        + if theme == Theme::Paper { 0. } else { 1. }
}
impl SettingsUi {
    pub(crate) fn paint_colors(
        &self,
        b: &mut super::Paint,
        field: ScreenshotField,
        area: Rect,
        narrow: bool,
    ) {
        let p = b.p;
        let colors = field.colors();
        // Keep the retained label's circle, gap, inset and text width, rather
        // than treating its invisible native color input as the hit target.
        let picker_width = 82.203125;
        let total = colors.len() as f32 * 42. + 4. + picker_width;
        let available = if narrow && field == ScreenshotField::SelectionMaskColor {
            area.width.min(520.)
        } else {
            area.width
        };
        let x = if narrow && field == ScreenshotField::SelectionBorderColor {
            area.x
        } else {
            area.x + available - total
        };
        let value = field.value(&self.snapshot.screenshot);
        for (i, color) in colors.iter().enumerate() {
            let rect = r(x + i as f32 * 42., area.y + 2., 32., 32.);
            b.fill(rect, 8., [255; 4]);
            b.fill(r(rect.x + 3., rect.y + 3., 26., 26.), 5., *color);
            let selected = value == Value::Number(u32::from_be_bytes(*color));
            b.stroke(
                r(rect.x - 1., rect.y - 1., 34., 34.),
                9.,
                if selected { 2. } else { 1. },
                if selected { p.accent } else { p.line },
            );
            b.hit(rect, Target::ScreenshotColor(field, i), field.label());
        }
        let rect = r(
            x + colors.len() as f32 * 42. + 4.,
            area.y,
            picker_width,
            36.,
        );
        b.fill(rect, 8., p.surface);
        b.stroke(rect, 8., 1., p.line);
        let color = match value {
            Value::Number(value) => value.to_be_bytes(),
            _ => unreachable!(),
        };
        b.fill(
            r(rect.x + 13., rect.y + 10., 16., 16.),
            8.,
            [color[0], color[1], color[2], 255],
        );
        b.text(
            r(rect.x + 37., rect.y + 10., rect.width - 38., 18.),
            "调色板",
            10.5,
            500,
            p.muted,
            FontFamily::Sans,
        );
        b.hit(rect, Target::PickScreenshotColor(field), "调色板");
    }
    pub(crate) fn paint_local_shortcuts(
        &self,
        b: &mut super::Paint,
        x: f32,
        y: f32,
        width: f32,
        inner: f32,
        narrow: bool,
    ) {
        let p = b.p;
        let logical_width = self.size.width / self.scale;
        let columns = if logical_width < 768. { 1 } else { 2 };
        let inset = if self.snapshot.theme == Theme::Paper {
            24.
        } else {
            25.
        };
        let available = width - inset * 2.;
        b.text(r(x + inset, y + 16., available, 30.), "标注快捷键仅在截图编辑器中生效。可设置字母、数字或功能键，Delete 可清除。F3 用于贴图；绑定已用作全局快捷键的按键前，请先修改全局绑定。", 10.5, 400, p.muted, FontFamily::Sans);
        let grid_y = y + 62.;
        let column_width = (available - if columns == 2 { 40. } else { 0. }) / columns as f32;
        for (index, (tool, label)) in ANNOTATION_SHORTCUT_LABELS.iter().enumerate() {
            let row = index / columns;
            let column = index % columns;
            let xx = x + inset + column as f32 * (column_width + 40.);
            let yy = grid_y + row as f32 * 44.;
            let field = ScreenshotField::AnnotationShortcut(tool);
            b.text(
                r(xx, yy + 8., column_width - 124., 20.),
                *label,
                10.5,
                400,
                p.ink,
                FontFamily::Sans,
            );
            let value = self
                .snapshot
                .screenshot
                .annotation_tool_shortcuts
                .get(*tool)
                .map(String::as_str)
                .unwrap_or("")
                .to_uppercase();
            let recording = self.local_recording == Some(field);
            b.button(
                r(xx + column_width - 112., yy, 112., 32.),
                Target::Screenshot(field),
                if recording {
                    "按下按键"
                } else if value.is_empty() {
                    "未设置"
                } else {
                    &value
                },
                recording,
            );
        }
        let rows = ANNOTATION_SHORTCUT_LABELS.len().div_ceil(columns) as f32;
        let restore = r(x + inset, grid_y + rows * 44. + 4., 164., 32.);
        b.button(
            restore,
            Target::Screenshot(ScreenshotField::ResetAnnotationShortcuts),
            "恢复标注快捷键默认值",
            false,
        );
        let border = if self.snapshot.theme == Theme::Paper {
            0.
        } else {
            1.
        };
        let pin_y = y + annotation_height(logical_width) + border;
        if border > 0. {
            b.line(x, pin_y - 1., x + width, pin_y - 1., p.line);
        }
        let padding = match self.snapshot.theme {
            Theme::Paper => 16.,
            Theme::Studio => 9.,
            _ => 13.,
        };
        let label_y = pin_y + padding;
        b.text(
            r(
                x + inner,
                label_y,
                width - inner * 2. - if narrow { 0. } else { 200. },
                20.,
            ),
            "贴图边框快捷键",
            12.5,
            600,
            p.ink,
            FontFamily::Sans,
        );
        b.text(
            r(x + inner, label_y + 21., width - inner * 2., 18.),
            "在贴图窗口内切换所有贴图的边框。Delete 清除。",
            10.5,
            400,
            p.muted,
            FontFamily::Sans,
        );
        let field = ScreenshotField::PinBorderShortcut;
        let recording = self.local_recording == Some(field);
        let control_y = if narrow {
            label_y + 47.
        } else {
            pin_y + (pin_height(logical_width, self.snapshot.theme) - 36.) / 2.
        };
        let value = self.snapshot.screenshot.pin_border_shortcut.to_uppercase();
        b.button(
            r(
                if narrow {
                    x + inner + 90.
                } else {
                    x + width - inner - 180.
                },
                control_y,
                180.,
                36.,
            ),
            Target::Screenshot(field),
            if recording {
                "按下快捷键"
            } else if value.is_empty() {
                "未设置"
            } else {
                &value
            },
            recording,
        );
    }
}
