//! Shared capture pixels and controls. Selection and annotation transitions stay
//! in the portable capture application; window hosts normalize OS events.
use crate::theme::palette;
use crate::*;
use std::collections::HashMap;
pub mod presentation;
use presentation::{
    hint_frame, selection_badge, selection_overlay, toolbar_frame, toolbar_icon, toolbar_intent,
    toolbar_size, MagnifierPresentation, PalettePresentation, ToolbarLabel, ToolbarPresentation,
};
#[cfg(test)]
mod tests;

/// Tint variants for the existing alpha-template toolbar assets: normal,
/// selected, disabled and the primary copy command.
pub fn icon_colors(theme: Theme, dark: bool) -> [Color; 4] {
    let colors = palette(theme, dark);
    let mut disabled = colors.muted;
    disabled[3] = 100;
    [colors.muted, colors.accent, disabled, colors.accent_on]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CaptureCommand {
    Select,
    Undo,
    Redo,
    Clear,
    Cancel,
    Refresh,
    Copy,
    Save,
    QuickSave,
    Pin,
    Favorite,
    Ocr,
    Scrolling,
    Print,
    Primary,
}
impl CaptureCommand {
    pub fn name(self) -> &'static str {
        match self {
            Self::Select => "select",
            Self::Undo => "undo",
            Self::Redo => "redo",
            Self::Clear => "clear",
            Self::Cancel => "cancel",
            Self::Refresh => "refresh",
            Self::Copy => "copy",
            Self::Save => "save",
            Self::QuickSave => "quick-save",
            Self::Pin => "pin",
            Self::Favorite => "favorite",
            Self::Ocr => "ocr",
            Self::Scrolling => "scrolling",
            Self::Print => "print",
            Self::Primary => "primary",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Select => "选择和移动",
            Self::Undo => "撤销",
            Self::Redo => "重做",
            Self::Clear => "清除标注",
            Self::Cancel => "取消",
            Self::Refresh => "重新截图",
            Self::Copy => "复制",
            Self::Save => "另存为",
            Self::QuickSave => "快捷保存",
            Self::Pin => "贴图",
            Self::Favorite => "收藏",
            Self::Ocr => "OCR",
            Self::Scrolling => "长截图",
            Self::Print => "打印",
            Self::Primary => "完成",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CaptureTool {
    Rectangle,
    Ellipse,
    Arrow,
    Line,
    Pen,
    Text,
    Mosaic,
    Eraser,
    Highlight,
    RegionHighlight,
    Blur,
    Spotlight,
}
impl CaptureTool {
    pub const ALL: [Self; 12] = [
        Self::Rectangle,
        Self::Ellipse,
        Self::Arrow,
        Self::Line,
        Self::Pen,
        Self::Text,
        Self::Mosaic,
        Self::Eraser,
        Self::Highlight,
        Self::RegionHighlight,
        Self::Blur,
        Self::Spotlight,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Rectangle => "rectangle",
            Self::Ellipse => "ellipse",
            Self::Arrow => "arrow",
            Self::Line => "line",
            Self::Pen => "pen",
            Self::Text => "text",
            Self::Mosaic => "mosaic",
            Self::Eraser => "eraser",
            Self::Highlight => "highlight",
            Self::RegionHighlight => "region-highlight",
            Self::Blur => "blur",
            Self::Spotlight => "spotlight",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Rectangle => "矩形",
            Self::Ellipse => "椭圆",
            Self::Arrow => "箭头",
            Self::Line => "直线",
            Self::Pen => "画笔",
            Self::Text => "文字",
            Self::Mosaic => "马赛克",
            Self::Eraser => "橡皮擦",
            Self::Highlight => "荧光笔",
            Self::RegionHighlight => "区域高亮",
            Self::Blur => "模糊",
            Self::Spotlight => "聚光灯",
        }
    }
}
#[derive(Clone, Debug)]
pub struct CaptureImage {
    pub rect: Rect,
    pub image: RasterImage,
}
#[derive(Clone, Debug)]
pub struct CaptureStyle {
    pub color: Color,
    pub stroke_width: u32,
    pub font_size: u32,
    pub filled: bool,
    pub corner_radius: f32,
    pub blur_radius: f32,
    pub bold: bool,
    pub italic: bool,
    pub font_family: String,
    pub fonts: Vec<String>,
    pub presets: Vec<Color>,
    pub text_mode: bool,
    pub fill_enabled: bool,
    pub round_enabled: bool,
    pub blur_mode: bool,
    pub error: Option<String>,
}
impl Default for CaptureStyle {
    fn default() -> Self {
        Self {
            color: [255, 77, 79, 255],
            stroke_width: 2,
            font_size: 24,
            filled: false,
            corner_radius: 0.,
            blur_radius: 16.,
            bold: false,
            italic: false,
            font_family: String::new(),
            fonts: vec![],
            presets: vec![],
            text_mode: false,
            fill_enabled: true,
            round_enabled: true,
            blur_mode: false,
            error: None,
        }
    }
}
#[derive(Clone, Debug)]
pub struct CaptureSnapshot {
    pub size: Size,
    pub theme: Theme,
    pub dark: bool,
    pub ui_scale: f32,
    pub desktop_scale: f32,
    pub images: Vec<CaptureImage>,
    pub cursor: Option<CaptureImage>,
    pub preview: Option<CaptureImage>,
    pub selection: Option<Rect>,
    pub selection_tinted: bool,
    pub handles: Vec<Point>,
    pub point: Option<Point>,
    pub magnifier: Option<CaptureImage>,
    pub selection_size: String,
    pub sampled_color: Option<String>,
    pub magnifier_label: String,
    pub show_size: bool,
    pub border_color: Color,
    pub border_width: f32,
    pub mask_color: Color,
    pub editing: bool,
    pub tool: Option<CaptureTool>,
    pub style: CaptureStyle,
    pub can_undo: bool,
    pub can_redo: bool,
    pub busy: bool,
    pub hint: String,
    pub icons: HashMap<String, RasterImage>,
    pub text_caret: Option<Rect>,
}
impl Default for CaptureSnapshot {
    fn default() -> Self {
        Self {
            size: Size {
                width: 900.,
                height: 650.,
            },
            theme: Theme::Native,
            dark: false,
            ui_scale: 1.,
            desktop_scale: 1.,
            images: vec![],
            cursor: None,
            preview: None,
            selection: None,
            selection_tinted: false,
            handles: vec![],
            point: None,
            magnifier: None,
            selection_size: String::new(),
            sampled_color: None,
            magnifier_label: String::new(),
            show_size: true,
            border_color: [91, 127, 255, 242],
            border_width: 2.,
            mask_color: [0, 0, 0, 46],
            editing: false,
            tool: None,
            style: CaptureStyle::default(),
            can_undo: false,
            can_redo: false,
            busy: false,
            hint: "拖动选区或点击窗口 · Enter 完成 · Tab 界面元素 · Alt 放大镜 · Esc 取消".into(),
            icons: HashMap::new(),
            text_caret: None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CaptureTarget {
    Command(CaptureCommand),
    Tool(CaptureTool),
    Color,
    PaletteMenu,
    PaletteClose,
    ShapeMenu,
    ArrowMenu,
    Palette(usize),
    SystemColor,
    AddColor,
    ReplaceColor,
    DeleteColor,
    Size,
    Fill,
    Round,
    Blur,
    Font,
    Bold,
    Italic,
    FontDecrease,
    FontIncrease,
    FontOption(usize),
}
#[derive(Clone, Debug)]
pub struct CaptureControl {
    pub target: CaptureTarget,
    pub rect: Rect,
    pub label: String,
    pub enabled: bool,
    pub selected: bool,
    pub value: Option<f64>,
    pub range: Option<(f64, f64)>,
}
#[derive(Clone, Debug)]
pub struct CaptureFrame {
    pub scene: Scene,
    pub ime_cursor: Option<Rect>,
    pub controls: Vec<CaptureControl>,
}
impl CaptureFrame {
    pub fn hit_test(&self, point: Point) -> Option<CaptureTarget> {
        self.controls
            .iter()
            .rev()
            .find(|c| c.enabled && c.rect.contains(point))
            .map(|c| c.target)
    }
}
#[derive(Clone, Debug)]
pub enum CaptureStyleValue {
    Color(Color),
    StrokeWidth(u32),
    FontSize(u32),
    Filled(bool),
    CornerRadius(f64),
    BlurRadius(f64),
    FontFamily(String),
    Bold(bool),
    Italic(bool),
}
#[derive(Clone, Debug)]
pub enum CaptureIntent {
    Command(CaptureCommand),
    Tool(CaptureTool),
    Style {
        value: CaptureStyleValue,
        commit: bool,
    },
    Presets(Vec<Color>),
    SystemColor(Color),
}
#[derive(Default)]
pub struct CaptureUi {
    frame: Option<CaptureFrame>,
    palette: bool,
    font_menu: bool,
    selected_color: Option<usize>,
    dragging: Option<CaptureTarget>,
    tool_menu: Option<bool>,
    panels: Vec<Rect>,
    font_scroll: f32,
    font_view: Rect,
}
impl CaptureUi {
    pub fn frame(&mut self, s: &CaptureSnapshot) -> CaptureFrame {
        self.frame_inner(s, None)
    }
    pub fn frame_with_text(
        &mut self,
        s: &CaptureSnapshot,
        metrics: &mut impl TextMeasurer,
    ) -> CaptureFrame {
        self.frame_inner(s, Some(metrics))
    }
    fn frame_inner(
        &mut self,
        s: &CaptureSnapshot,
        mut metrics: Option<&mut dyn TextMeasurer>,
    ) -> CaptureFrame {
        self.panels.clear();
        let bounds = box_rect(0., 0., s.size.width, s.size.height);
        let mut b = Painter {
            scene: Scene {
                size: s.size,
                commands: vec![],
            },
            controls: vec![],
            scale: s.ui_scale.clamp(0.5, 3.),
            icons: &s.icons,
        };
        for image in &s.images {
            b.image(image);
        }
        if let Some(image) = &s.cursor {
            b.image(image);
        }
        if let Some(selection) = s.selection {
            let r = selection.intersect(bounds);
            let overlay = selection_overlay(r, s.size);
            for area in overlay.masks {
                b.fill(area, 0., s.mask_color);
            }
            if let Some(image) = &s.preview {
                b.image(image);
            }
            if s.selection_tinted {
                b.fill(r, 0., [255, 255, 255, 13]);
            }
            b.stroke(overlay.outline, 0., s.border_width, s.border_color);
            for point in &s.handles {
                b.fill(box_rect(point.x - 3., point.y - 3., 6., 6.), 1., [255; 4]);
                b.stroke(
                    box_rect(point.x - 3., point.y - 3., 6., 6.),
                    1.,
                    1.,
                    s.border_color,
                );
            }
            if s.show_size {
                let text = if s.selection_size.is_empty() {
                    format!(
                        "{} x {} px",
                        (r.width * s.desktop_scale).ceil() as u32,
                        (r.height * s.desktop_scale).ceil() as u32
                    )
                } else {
                    s.selection_size.clone()
                };
                let badge = selection_badge(
                    r,
                    s.size,
                    text_width(&text, 12. * b.scale, &mut metrics),
                    b.scale,
                );
                b.fill(badge, 0., [0, 0, 0, 210]);
                b.text(
                    box_rect(
                        badge.x + 2. * b.scale,
                        badge.y + 2. * b.scale,
                        badge.width - 4. * b.scale,
                        18. * b.scale,
                    ),
                    &text,
                    12. * b.scale,
                    [255; 4],
                );
                if let Some(DrawCommand::Text { align, .. }) = b.scene.commands.last_mut() {
                    *align = TextAlign::Center;
                }
            }
            if s.editing {
                self.toolbar(&mut b, s, r);
            }
        }
        if let Some(lens) = &s.magnifier {
            let scale = b.scale;
            let layout = MagnifierPresentation::new(scale);
            let place = |rect: Rect| Rect {
                x: lens.rect.x + rect.x,
                y: lens.rect.y + rect.y,
                ..rect
            };
            let point = |point: Point| Point {
                x: lens.rect.x + point.x,
                y: lens.rect.y + point.y,
            };
            let panel = box_rect(
                lens.rect.x,
                lens.rect.y,
                layout.size.width,
                layout.size.height,
            );
            b.fill(panel, 0., MagnifierPresentation::BACKGROUND);
            b.image(lens);
            for (from, to) in layout.crosshair {
                b.line(
                    point(from),
                    point(to),
                    scale,
                    MagnifierPresentation::CROSSHAIR,
                );
            }
            b.stroke(place(layout.sample_outline), 0., 2. * scale, [255; 4]);
            b.stroke(panel, 0., 1., MagnifierPresentation::BORDER);
            let label = if s.magnifier_label.is_empty() {
                s.sampled_color.as_deref().unwrap_or("")
            } else {
                &s.magnifier_label
            };
            for (index, line) in label.lines().enumerate() {
                b.scene.commands.push(DrawCommand::Text {
                    rect: box_rect(
                        lens.rect.x + layout.label.x,
                        lens.rect.y + layout.label.y + index as f32 * 15. * scale,
                        layout.label.width,
                        15. * scale,
                    ),
                    text: line.into(),
                    size: 12. * scale,
                    weight: 400,
                    color: [255; 4],
                    font: FontFamily::Sans,
                    align: TextAlign::Center,
                });
            }
        }
        if !s.hint.is_empty() {
            let maximum = (bounds.width - 32. * b.scale).max(0.);
            let width = text_width(&s.hint, 12. * b.scale, &mut metrics).min(maximum);
            let height = if let Some(metrics) = metrics.as_deref_mut() {
                metrics
                    .line_layout(
                        &s.hint,
                        width,
                        12. * b.scale,
                        400,
                        FontFamily::Sans,
                        15. * b.scale,
                    )
                    .height
            } else {
                (text_width(&s.hint, 12. * b.scale, &mut metrics) / width.max(1.)).ceil()
                    * 15.
                    * b.scale
            }
            .min(64. * b.scale);
            let hint = hint_frame(s.size, Size { width, height }, b.scale);
            b.fill(hint, 0., [0, 0, 0, 210]);
            if let Some(metrics) = metrics.as_deref_mut() {
                let layout = metrics.line_layout(
                    &s.hint,
                    width,
                    12. * b.scale,
                    400,
                    FontFamily::Sans,
                    15. * b.scale,
                );
                let mut starts = vec![(0, 0.)];
                for (byte, point) in layout.caret_stops {
                    if point.y > starts.last().unwrap().1 {
                        starts.push((byte, point.y));
                    }
                }
                for (index, (start, yy)) in starts.iter().copied().enumerate() {
                    let end = starts
                        .get(index + 1)
                        .map_or(s.hint.len(), |(byte, _)| *byte);
                    b.text(
                        box_rect(hint.x, hint.y + yy, hint.width, 15. * b.scale),
                        s.hint[start..end].trim_end_matches(['\n', '\r']),
                        12. * b.scale,
                        [255; 4],
                    );
                    if let Some(DrawCommand::Text { align, .. }) = b.scene.commands.last_mut() {
                        *align = TextAlign::Center;
                    }
                }
            } else {
                b.text(hint, &s.hint, 12. * b.scale, [255; 4]);
                if let Some(DrawCommand::Text { align, .. }) = b.scene.commands.last_mut() {
                    *align = TextAlign::Center;
                }
            }
        }
        let f = CaptureFrame {
            scene: b.scene,
            ime_cursor: s.text_caret,
            controls: b.controls,
        };
        self.frame = Some(f.clone());
        f
    }
    fn toolbar(&mut self, b: &mut Painter, s: &CaptureSnapshot, selection: Rect) {
        let p = palette(s.theme, s.dark);
        let scale = b.scale;
        let size = toolbar_size(&s.style, s.size.width - 16. * scale, scale);
        let layout = ToolbarPresentation::new(size.width, scale, s);
        let toolbar = toolbar_frame(selection, s.size, layout.size, scale);
        let (x, y) = (toolbar.x, toolbar.y);
        let place = |rect: Rect| Rect {
            x: x + rect.x,
            y: y + rect.y,
            ..rect
        };
        let at = |xx: f32, yy: f32, w: f32, h: f32| {
            box_rect(x + xx * scale, y + yy * scale, w * scale, h * scale)
        };
        for panel in &layout.panels {
            let panel = place(*panel);
            self.panels.push(panel);
            b.fill(panel, 11. * scale, p.surface);
            b.stroke(panel, 11. * scale, 0.5, p.line);
        }
        for control in &layout.controls {
            let rect = place(control.rect);
            if matches!(
                control.target,
                CaptureTarget::ShapeMenu | CaptureTarget::ArrowMenu
            ) {
                b.chevron(rect, p.muted);
                b.control(
                    control.target,
                    rect,
                    &control.label,
                    control.enabled,
                    control.selected,
                );
            } else if matches!(
                control.target,
                CaptureTarget::Tool(_) | CaptureTarget::Command(_)
            ) {
                let primary = control.target == CaptureTarget::Command(CaptureCommand::Copy);
                if primary {
                    b.fill(rect, 4. * scale, p.accent);
                }
                b.button(
                    control.target,
                    rect,
                    toolbar_icon(control.target),
                    &control.label,
                    control.enabled,
                    control.selected,
                    if primary {
                        p.accent_on
                    } else if control.selected {
                        p.accent
                    } else {
                        p.muted
                    },
                    p.accent_soft,
                );
            }
        }
        let color = place(layout.control(CaptureTarget::Color).unwrap().rect);
        b.fill(color, 0., [0, 0, 0, 255]);
        b.fill(
            box_rect(
                color.x + 3. * scale,
                color.y + 2. * scale,
                22. * scale,
                24. * scale,
            ),
            4. * scale,
            p.surface,
        );
        b.stroke(
            box_rect(
                color.x + 3. * scale,
                color.y + 2. * scale,
                22. * scale,
                24. * scale,
            ),
            4. * scale,
            0.5,
            p.line,
        );
        let well = box_rect(
            color.x + 5. * scale,
            color.y + 5. * scale,
            18. * scale,
            20. * scale,
        );
        b.fill(well, 3. * scale, [0, 0, 0, 255]);
        for row in 0..(well.height.ceil() as u32) {
            let fraction = row as f32 / well.height;
            let start = well.width * (1. - fraction);
            b.fill(
                box_rect(well.x + start, well.y + row as f32, well.width - start, 1.),
                0.,
                [255; 4],
            );
        }
        b.fill(well, 3. * scale, s.style.color);
        b.control(
            CaptureTarget::Color,
            color,
            "颜色 · 支持 RGB、HEX 和透明度",
            !s.busy,
            false,
        );
        let palette_toggle = place(layout.control(CaptureTarget::PaletteMenu).unwrap().rect);
        b.chevron(palette_toggle, p.muted);
        b.control(
            CaptureTarget::PaletteMenu,
            palette_toggle,
            "颜色预设",
            !s.busy,
            self.palette,
        );
        let size = layout.control(CaptureTarget::Size).unwrap();
        let (min, max) = size.range.unwrap();
        b.slider(
            CaptureTarget::Size,
            place(size.rect),
            size.value.unwrap() as f32,
            min as f32,
            max as f32,
            &size.label,
            size.enabled,
            p.accent,
            p.line,
        );
        let value = layout.label(ToolbarLabel::SizeValue).unwrap();
        b.text(place(value.rect), &value.text, 11. * scale, p.muted);
        let fill_control = layout.control(CaptureTarget::Fill).unwrap();
        let fill = place(fill_control.rect);
        let check = box_rect(
            fill.x + 2. * scale,
            fill.y + 7. * scale,
            14. * scale,
            14. * scale,
        );
        for row in 0..(check.height.ceil() as u32) {
            let shade = if s.dark {
                60. - row as f32 / check.height * 10.
            } else {
                252. - row as f32 / check.height * 7.
            };
            let channel = shade.round() as u8;
            b.fill(
                box_rect(check.x, check.y + row as f32, check.width, 1.),
                3. * scale,
                [channel, channel, channel, 255],
            );
        }
        b.stroke(
            check,
            3. * scale,
            0.5,
            if s.dark {
                p.line_strong
            } else {
                [180, 180, 180, 255]
            },
        );
        if s.style.filled {
            b.fill(check, 3. * scale, p.accent);
            b.line(
                Point {
                    x: check.x + 3. * scale,
                    y: check.y + 7. * scale,
                },
                Point {
                    x: check.x + 6. * scale,
                    y: check.y + 10. * scale,
                },
                1.5 * scale,
                p.accent_on,
            );
            b.line(
                Point {
                    x: check.x + 6. * scale,
                    y: check.y + 10. * scale,
                },
                Point {
                    x: check.x + 11. * scale,
                    y: check.y + 4. * scale,
                },
                1.5 * scale,
                p.accent_on,
            );
        }
        b.control(
            CaptureTarget::Fill,
            fill,
            "填充",
            fill_control.enabled,
            fill_control.selected,
        );
        if let Some(font) = layout.control(CaptureTarget::Font) {
            let family = font.rect.width / scale;
            let yy = font.rect.y / scale;
            let font_label = &font.label;
            let mut font_ink = if s.dark { p.ink } else { [40, 40, 40, 255] };
            if s.busy {
                font_ink[3] = 100;
            }
            b.fill(
                at(14.5, yy + 3.5, family - 6., 20.5),
                5.5 * scale,
                [0, 0, 0, 20],
            );
            let bezel = at(14.5, yy + 2.5, family - 6., 20.);
            b.fill(bezel, 5.5 * scale, p.surface);
            b.stroke(bezel, 5.5 * scale, 0.5, p.line);
            b.text(
                at(24., yy + 6., family - 40., 18.),
                font_label,
                11. * scale,
                font_ink,
            );
            for (offset, direction) in [(10., -1.), (16., 1.)] {
                let center = at(family - 2., yy + offset, 0., 0.);
                for sign in [-1., 1.] {
                    b.line(
                        Point {
                            x: center.x + sign * 3. * scale,
                            y: center.y - direction * 1.5 * scale,
                        },
                        Point {
                            x: center.x,
                            y: center.y + direction * 1.5 * scale,
                        },
                        1.5 * scale,
                        font_ink,
                    );
                }
            }
            b.control(
                CaptureTarget::Font,
                place(font.rect),
                font_label,
                font.enabled,
                font.selected,
            );
            for target in [
                CaptureTarget::Bold,
                CaptureTarget::Italic,
                CaptureTarget::FontDecrease,
                CaptureTarget::FontIncrease,
            ] {
                let control = layout.control(target).unwrap();
                b.button(
                    target,
                    place(control.rect),
                    "",
                    &control.label,
                    control.enabled,
                    control.selected,
                    if control.selected { p.accent } else { p.muted },
                    p.accent_soft,
                );
            }
            let label = layout.label(ToolbarLabel::FontValue).unwrap();
            b.text(place(label.rect), &label.text, 11. * scale, p.muted);
        }
        if let Some(round) = layout.control(CaptureTarget::Round) {
            let (min, max) = round.range.unwrap();
            b.slider(
                CaptureTarget::Round,
                place(round.rect),
                round.value.unwrap() as f32,
                min as f32,
                max as f32,
                &round.label,
                round.enabled,
                p.accent,
                p.line,
            );
            for kind in [ToolbarLabel::CornerTitle, ToolbarLabel::CornerValue] {
                let label = layout.label(kind).unwrap();
                b.text(place(label.rect), &label.text, 11. * scale, p.muted);
            }
        }
        if let Some(error) = layout.label(ToolbarLabel::Error) {
            b.text(place(error.rect), &error.text, 11. * scale, p.muted);
        }
        if let Some(arrows) = self.tool_menu {
            let choices = if arrows {
                [CaptureTool::Arrow, CaptureTool::Line]
            } else {
                [CaptureTool::Rectangle, CaptureTool::Ellipse]
            };
            let menu = at(if arrows { 78. } else { 36. }, 34., 150., 64.);
            self.panels.push(menu);
            b.fill(menu, 6. * scale, p.surface);
            b.stroke(menu, 6. * scale, 0.5, p.line);
            for (index, tool) in choices.into_iter().enumerate() {
                b.button(
                    CaptureTarget::Tool(tool),
                    box_rect(
                        menu.x + 4. * scale,
                        menu.y + (4. + index as f32 * 28.) * scale,
                        142. * scale,
                        28. * scale,
                    ),
                    tool.name(),
                    tool.label(),
                    !s.busy,
                    s.tool == Some(tool),
                    p.muted,
                    p.accent_soft,
                );
                b.text(
                    box_rect(
                        menu.x + 30. * scale,
                        menu.y + (10. + index as f32 * 28.) * scale,
                        110. * scale,
                        18. * scale,
                    ),
                    tool.label(),
                    11. * scale,
                    p.muted,
                );
            }
        }
        if self.palette {
            let layout =
                PalettePresentation::new(&s.style, self.selected_color, s.busy, false, scale);
            let palette = layout.frame(
                toolbar,
                box_rect(0., 0., s.size.width, s.size.height),
                scale,
            );
            self.panels.push(palette);
            b.fill(palette, 6. * scale, p.surface);
            b.stroke(palette, 6. * scale, 0.5, p.line);
            let place = |rect: Rect| Rect {
                x: palette.x + rect.x,
                y: palette.y + rect.y,
                ..rect
            };
            b.text(place(layout.title), "颜色", 11. * scale, p.muted);
            if let Some(DrawCommand::Text { weight, .. }) = b.scene.commands.last_mut() {
                *weight = 600;
            }
            b.button(
                CaptureTarget::PaletteClose,
                place(layout.control(CaptureTarget::PaletteClose).unwrap().rect),
                "",
                "×",
                true,
                false,
                p.muted,
                p.accent_soft,
            );
            for (index, color) in s.style.presets.iter().enumerate() {
                let control = layout.control(CaptureTarget::Palette(index)).unwrap();
                let rect = place(control.rect);
                b.checker(rect, *color);
                if self.selected_color == Some(index) {
                    b.stroke(
                        box_rect(
                            rect.x - 2. * scale,
                            rect.y - 2. * scale,
                            rect.width + 4. * scale,
                            rect.height + 4. * scale,
                        ),
                        4. * scale,
                        2. * scale,
                        p.accent,
                    );
                }
                b.control(
                    CaptureTarget::Palette(index),
                    rect,
                    "预设颜色",
                    control.enabled,
                    control.selected,
                );
            }
            for target in [
                CaptureTarget::SystemColor,
                CaptureTarget::AddColor,
                CaptureTarget::ReplaceColor,
                CaptureTarget::DeleteColor,
            ] {
                let control = layout.control(target).unwrap();
                let rect = place(control.rect);
                let enabled = control.enabled;
                let label = &control.label;
                for row in 0..rect.height.ceil() as u32 {
                    let shade = if s.dark {
                        64. - row as f32 / rect.height * 9.
                    } else {
                        252. - row as f32 / rect.height * 7.
                    };
                    let channel = shade.round() as u8;
                    b.fill(
                        box_rect(rect.x, rect.y + row as f32, rect.width, 1.),
                        0.,
                        [channel, channel, channel, 255],
                    );
                }
                let border = if s.dark {
                    90
                } else if enabled {
                    180
                } else {
                    217
                };
                b.stroke(rect, 0., 1., [border, border, border, 255]);
                b.button(
                    target,
                    rect,
                    "",
                    label,
                    enabled,
                    false,
                    if s.dark {
                        [235, 235, 235, 255]
                    } else {
                        [0, 0, 0, 255]
                    },
                    p.accent_soft,
                );
                if let Some(DrawCommand::Text {
                    size,
                    rect: text_rect,
                    ..
                }) = b.scene.commands.last_mut()
                {
                    *size = 13. * scale;
                    text_rect.y = rect.y + (rect.height - 18. * scale) / 2.;
                    text_rect.height = 22. * scale;
                }
            }
        }
        if self.font_menu {
            let fonts = std::iter::once("默认字体")
                .chain(s.style.fonts.iter().map(String::as_str))
                .collect::<Vec<_>>();
            let row = 28. * scale;
            let h = (fonts.len() as f32 * row).min(s.size.height - 24.);
            let menu = box_rect(x, (y - h - 8. * scale).max(8.), 220. * scale, h);
            self.font_view = menu;
            self.font_scroll = self
                .font_scroll
                .clamp(0., (fonts.len() as f32 * row - h).max(0.));
            b.fill(menu, 6. * scale, p.surface);
            b.scene.commands.push(DrawCommand::PushClip(menu));
            for (index, font) in fonts.iter().enumerate() {
                let rect = box_rect(
                    menu.x,
                    menu.y + index as f32 * row - self.font_scroll,
                    menu.width,
                    row,
                )
                .intersect(menu);
                if rect.height <= 0. {
                    continue;
                }
                b.button(
                    CaptureTarget::FontOption(index),
                    rect,
                    "",
                    font,
                    !s.busy,
                    false,
                    p.ink,
                    p.accent_soft,
                );
            }
            b.scene.commands.push(DrawCommand::PopClip);
        }
    }
    pub fn dismiss_menus(&mut self) -> bool {
        let visible = self.palette || self.font_menu || self.tool_menu.is_some();
        self.palette = false;
        self.font_menu = false;
        self.tool_menu = None;
        visible
    }
    pub fn scroll(&mut self, point: Point, delta_y: f32, s: &CaptureSnapshot) -> bool {
        if !self.font_menu || !self.font_view.contains(point) || !delta_y.is_finite() {
            return false;
        }
        let total = (s.style.fonts.len() + 1) as f32 * 28. * s.ui_scale.clamp(0.5, 3.);
        self.font_scroll =
            (self.font_scroll + delta_y).clamp(0., (total - self.font_view.height).max(0.));
        true
    }
    pub fn activate(&mut self, target: CaptureTarget, s: &CaptureSnapshot) -> Vec<CaptureIntent> {
        if s.busy && target != CaptureTarget::PaletteClose {
            return vec![];
        }
        if !self
            .frame
            .as_ref()
            .is_some_and(|f| f.controls.iter().any(|c| c.target == target && c.enabled))
        {
            return vec![];
        }
        match target {
            CaptureTarget::Command(_) | CaptureTarget::Tool(_) => {
                let menu =
                    matches!(target, CaptureTarget::Tool(_)) && self.tool_menu.take().is_some();
                toolbar_intent(target, s, menu, false).into_iter().collect()
            }
            CaptureTarget::Color => vec![CaptureIntent::SystemColor(s.style.color)],
            CaptureTarget::PaletteMenu => {
                self.palette = !self.palette;
                self.font_menu = false;
                self.tool_menu = None;
                vec![]
            }
            CaptureTarget::PaletteClose => {
                self.palette = false;
                vec![]
            }
            CaptureTarget::ShapeMenu | CaptureTarget::ArrowMenu => {
                let arrows = target == CaptureTarget::ArrowMenu;
                self.tool_menu = if self.tool_menu == Some(arrows) {
                    None
                } else {
                    Some(arrows)
                };
                self.palette = false;
                self.font_menu = false;
                vec![]
            }
            CaptureTarget::Font => {
                self.font_menu = !self.font_menu;
                self.palette = false;
                self.tool_menu = None;
                vec![]
            }
            CaptureTarget::Palette(index) => {
                self.selected_color = Some(index);
                self.palette = false;
                s.style
                    .presets
                    .get(index)
                    .map(|c| {
                        vec![CaptureIntent::Style {
                            value: CaptureStyleValue::Color(*c),
                            commit: true,
                        }]
                    })
                    .unwrap_or_default()
            }
            CaptureTarget::SystemColor => vec![CaptureIntent::SystemColor(s.style.color)],
            CaptureTarget::AddColor => {
                let mut colors = s.style.presets.clone();
                if !colors.contains(&s.style.color) && colors.len() < 24 {
                    colors.push(s.style.color);
                    self.selected_color = Some(colors.len() - 1);
                    vec![CaptureIntent::Presets(colors)]
                } else {
                    vec![]
                }
            }
            CaptureTarget::ReplaceColor | CaptureTarget::DeleteColor => {
                let mut colors = s.style.presets.clone();
                let Some(index) = self.selected_color.filter(|i| *i < colors.len()) else {
                    return vec![];
                };
                if target == CaptureTarget::DeleteColor {
                    colors.remove(index);
                    self.selected_color = None;
                } else {
                    colors[index] = s.style.color;
                }
                vec![CaptureIntent::Presets(colors)]
            }
            CaptureTarget::Fill => vec![CaptureIntent::Style {
                value: CaptureStyleValue::Filled(!s.style.filled),
                commit: true,
            }],
            CaptureTarget::Bold => vec![CaptureIntent::Style {
                value: CaptureStyleValue::Bold(!s.style.bold),
                commit: true,
            }],
            CaptureTarget::FontDecrease | CaptureTarget::FontIncrease => {
                vec![CaptureIntent::Style {
                    value: CaptureStyleValue::FontSize(if target == CaptureTarget::FontIncrease {
                        s.style.font_size.saturating_add(2).clamp(10, 96)
                    } else {
                        s.style.font_size.saturating_sub(2).clamp(10, 96)
                    }),
                    commit: true,
                }]
            }
            CaptureTarget::Italic => vec![CaptureIntent::Style {
                value: CaptureStyleValue::Italic(!s.style.italic),
                commit: true,
            }],
            CaptureTarget::FontOption(index) => {
                self.font_menu = false;
                let font = if index == 0 {
                    Some(String::new())
                } else {
                    s.style.fonts.get(index - 1).cloned()
                };
                font.map(|font| {
                    vec![CaptureIntent::Style {
                        value: CaptureStyleValue::FontFamily(font),
                        commit: true,
                    }]
                })
                .unwrap_or_default()
            }
            _ => vec![],
        }
    }
    pub fn pointer_down(
        &mut self,
        point: Point,
        s: &CaptureSnapshot,
    ) -> Option<Vec<CaptureIntent>> {
        let Some(target) = self.frame.as_ref()?.hit_test(point) else {
            if self.tool_menu.take().is_some() || self.font_menu || self.palette {
                self.font_menu = false;
                self.palette = false;
                return Some(vec![]);
            }
            return self
                .panels
                .iter()
                .any(|panel| panel.contains(point))
                .then(Vec::new);
        };
        if matches!(
            target,
            CaptureTarget::Size | CaptureTarget::Round | CaptureTarget::Blur
        ) {
            self.dragging = Some(target);
            Some(self.slider(point, s, false))
        } else {
            Some(self.activate(target, s))
        }
    }
    pub fn pointer_move(&mut self, point: Point, s: &CaptureSnapshot) -> Vec<CaptureIntent> {
        if self.dragging.is_some() {
            self.slider(point, s, false)
        } else {
            vec![]
        }
    }
    pub fn pointer_up(&mut self, point: Point, s: &CaptureSnapshot) -> Vec<CaptureIntent> {
        let result = if self.dragging.is_some() {
            self.slider(point, s, true)
        } else {
            vec![]
        };
        self.dragging = None;
        result
    }
    fn slider(&self, point: Point, s: &CaptureSnapshot, commit: bool) -> Vec<CaptureIntent> {
        if s.busy {
            return vec![];
        }
        let Some(target) = self.dragging else {
            return vec![];
        };
        let Some(control) = self
            .frame
            .as_ref()
            .and_then(|f| f.controls.iter().find(|c| c.target == target))
        else {
            return vec![];
        };
        let Some((min, max)) = control.range else {
            return vec![];
        };
        let n = (min
            + (max - min) * ((point.x - control.rect.x) / control.rect.width).clamp(0., 1.) as f64)
            .round();
        let value = match target {
            CaptureTarget::Round => CaptureStyleValue::CornerRadius(n),
            CaptureTarget::Blur => CaptureStyleValue::BlurRadius(n),
            _ if s.style.text_mode => CaptureStyleValue::FontSize(n as u32),
            _ if s.style.blur_mode => CaptureStyleValue::BlurRadius(n),
            _ => CaptureStyleValue::StrokeWidth(n as u32),
        };
        vec![CaptureIntent::Style { value, commit }]
    }
}
fn box_rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect {
        x,
        y,
        width: width.max(0.),
        height: height.max(0.),
    }
}
struct Painter<'a> {
    scene: Scene,
    controls: Vec<CaptureControl>,
    scale: f32,
    icons: &'a HashMap<String, RasterImage>,
}
impl Painter<'_> {
    fn chevron(&mut self, rect: Rect, color: Color) {
        let x = rect.x + rect.width / 2.;
        let y = rect.y + rect.height / 2.;
        self.line(
            Point {
                x: x - 2.5 * self.scale,
                y: y - 1.5 * self.scale,
            },
            Point {
                x,
                y: y + 2. * self.scale,
            },
            self.scale,
            color,
        );
        self.line(
            Point {
                x,
                y: y + 2. * self.scale,
            },
            Point {
                x: x + 2.5 * self.scale,
                y: y - 1.5 * self.scale,
            },
            self.scale,
            color,
        );
    }
    fn checker(&mut self, rect: Rect, color: Color) {
        self.fill(rect, 4. * self.scale, [210, 210, 210, 255]);
        let tile = 4. * self.scale;
        for row in 0..(rect.height / tile).ceil() as usize {
            for col in 0..(rect.width / tile).ceil() as usize {
                if (row + col) % 2 == 0 {
                    self.fill(
                        box_rect(
                            rect.x + col as f32 * tile,
                            rect.y + row as f32 * tile,
                            tile.min(rect.width - col as f32 * tile),
                            tile.min(rect.height - row as f32 * tile),
                        ),
                        0.,
                        [245, 245, 245, 255],
                    );
                }
            }
        }
        self.fill(rect, 4. * self.scale, color);
    }
    fn fill(&mut self, rect: Rect, radius: f32, color: Color) {
        self.scene.commands.push(DrawCommand::FillRect {
            rect,
            radius,
            color,
        });
    }
    fn stroke(&mut self, rect: Rect, radius: f32, width: f32, color: Color) {
        self.scene.commands.push(DrawCommand::StrokeRect {
            rect,
            radius,
            width,
            color,
        });
    }
    fn image(&mut self, image: &CaptureImage) {
        self.scene.commands.push(DrawCommand::Image {
            rect: image.rect,
            image: image.image.clone(),
        });
    }
    fn text(&mut self, rect: Rect, text: &str, size: f32, color: Color) {
        self.scene.commands.push(DrawCommand::Text {
            rect,
            text: text.into(),
            size,
            weight: 400,
            color,
            font: FontFamily::Sans,
            align: TextAlign::Left,
        });
    }
    fn line(&mut self, from: Point, to: Point, width: f32, color: Color) {
        self.scene.commands.push(DrawCommand::Line {
            from,
            to,
            width,
            color,
        });
    }
    fn control(
        &mut self,
        target: CaptureTarget,
        rect: Rect,
        label: &str,
        enabled: bool,
        selected: bool,
    ) {
        self.controls.push(CaptureControl {
            target,
            rect,
            label: label.into(),
            enabled,
            selected,
            value: None,
            range: None,
        });
    }
    fn button(
        &mut self,
        target: CaptureTarget,
        rect: Rect,
        icon: &str,
        label: &str,
        enabled: bool,
        selected: bool,
        color: Color,
        active: Color,
    ) {
        if selected {
            self.fill(rect, 5., active);
            let mut border = color;
            if !matches!(target, CaptureTarget::Bold | CaptureTarget::Italic) {
                border[3] = 102;
            }
            self.stroke(rect, 4. * self.scale, 0.5, border);
        }
        let variant = if !enabled {
            format!("{icon}-disabled")
        } else if target == CaptureTarget::Command(CaptureCommand::Copy) {
            format!("{icon}-on")
        } else if selected {
            format!("{icon}-active")
        } else {
            icon.into()
        };
        if let Some(image) = self.icons.get(&variant).or_else(|| self.icons.get(icon)) {
            let extent = 17. * self.scale;
            self.scene.commands.push(DrawCommand::Image {
                rect: box_rect(
                    rect.x + (rect.width - extent) / 2.,
                    rect.y + (rect.height - extent) / 2.,
                    extent,
                    extent,
                ),
                image: image.clone(),
            });
        } else {
            self.text(
                box_rect(
                    rect.x + 4. * self.scale,
                    rect.y + (rect.height - 14. * self.scale) / 2.,
                    rect.width - 8. * self.scale,
                    18. * self.scale,
                ),
                label,
                11. * self.scale,
                if enabled {
                    color
                } else {
                    [color[0], color[1], color[2], 100]
                },
            );
            if let Some(DrawCommand::Text { align, .. }) = self.scene.commands.last_mut() {
                *align = TextAlign::Center;
            }
        }
        self.control(target, rect, label, enabled, selected);
    }
    fn slider(
        &mut self,
        target: CaptureTarget,
        rect: Rect,
        value: f32,
        min: f32,
        max: f32,
        label: &str,
        enabled: bool,
        _accent: Color,
        line: Color,
    ) {
        self.fill(
            box_rect(
                rect.x + 2. * self.scale,
                rect.y + 10. * self.scale,
                (rect.width - 4. * self.scale).max(0.),
                4. * self.scale,
            ),
            2.,
            [244, 244, 244, 255],
        );
        let x = rect.x
            + 12. * self.scale
            + ((value - min) / (max - min)).clamp(0., 1.) * (rect.width - 22. * self.scale).max(1.);
        self.fill(
            box_rect(
                x - 10.5 * self.scale,
                rect.y + 2. * self.scale,
                21. * self.scale,
                21. * self.scale,
            ),
            10.5 * self.scale,
            [0, 0, 0, 20],
        );
        self.fill(
            box_rect(
                x - 10. * self.scale,
                rect.y + 2. * self.scale,
                20. * self.scale,
                20. * self.scale,
            ),
            10. * self.scale,
            [255, 255, 255, 255],
        );
        self.stroke(
            box_rect(
                x - 10. * self.scale,
                rect.y + 2. * self.scale,
                20. * self.scale,
                20. * self.scale,
            ),
            10. * self.scale,
            0.5,
            line,
        );
        self.controls.push(CaptureControl {
            target,
            rect,
            label: label.into(),
            enabled,
            selected: false,
            value: Some(value as f64),
            range: Some((min as f64, max as f64)),
        });
    }
}

fn text_width(text: &str, size: f32, metrics: &mut Option<&mut dyn TextMeasurer>) -> f32 {
    if let Some(metrics) = metrics.as_deref_mut() {
        metrics
            .measure(text, size, 400, FontFamily::Sans)
            .caret_stops
            .iter()
            .map(|(_, x)| *x)
            .fold(0., f32::max)
    } else {
        text.chars()
            .map(|ch| if ch.is_ascii() { size * 0.55 } else { size })
            .sum()
    }
}
