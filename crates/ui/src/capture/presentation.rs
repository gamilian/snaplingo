//! Capture controls in logical coordinates. Native controls and the shared
//! scene consume the same geometry, values and availability.
use super::*;

#[derive(Clone, Debug)]
pub struct SelectionOverlay {
    pub masks: [Rect; 4],
    pub outline: Rect,
}

pub fn selection_overlay(selection: Rect, viewport: Size) -> SelectionOverlay {
    let left = selection.x.clamp(0., viewport.width);
    let top = selection.y.clamp(0., viewport.height);
    let right = (selection.x + selection.width).clamp(left, viewport.width);
    let bottom = (selection.y + selection.height).clamp(top, viewport.height);
    SelectionOverlay {
        masks: [
            box_rect(0., 0., viewport.width, top),
            box_rect(0., bottom, viewport.width, viewport.height - bottom),
            box_rect(0., top, left, bottom - top),
            box_rect(right, top, viewport.width - right, bottom - top),
        ],
        outline: box_rect(
            selection.x + 0.5,
            selection.y + 0.5,
            selection.width - 1.,
            selection.height - 1.,
        ),
    }
}

pub fn selection_badge(selection: Rect, viewport: Size, text_width: f32, scale: f32) -> Rect {
    let width = text_width + 16. * scale;
    let height = 20. * scale;
    let above = selection.y - height - 6. * scale;
    box_rect(
        selection
            .x
            .clamp(0., (viewport.width - width).max(0.))
            .round(),
        if above >= 0. { above } else { selection.y }
            .clamp(0., (viewport.height - height).max(0.))
            .round(),
        width,
        height,
    )
}

pub fn hint_frame(viewport: Size, measured: Size, scale: f32) -> Rect {
    let width = measured.width.min((viewport.width - 32. * scale).max(0.));
    let height = measured.height.min(64. * scale);
    box_rect(
        ((viewport.width - width) / 2.).max(0.),
        viewport.height - height - 20. * scale,
        width,
        height,
    )
}

#[derive(Clone, Debug)]
pub struct MagnifierPresentation {
    pub size: Size,
    pub image: Rect,
    pub label: Rect,
    pub crosshair: [(Point, Point); 2],
    pub sample_outline: Rect,
    scale: f32,
}

impl MagnifierPresentation {
    pub const BACKGROUND: Color = [22, 25, 32, 215];
    pub const CROSSHAIR: Color = [96, 165, 250, 180];
    pub const BORDER: Color = [255, 255, 255, 90];

    pub fn new(scale: f32) -> Self {
        Self {
            size: Size {
                width: 228. * scale,
                height: 226. * scale,
            },
            image: box_rect(0., 0., 228. * scale, 132. * scale),
            label: box_rect(4. * scale, 140. * scale, 220. * scale, 82. * scale),
            crosshair: [
                (
                    Point {
                        x: 114. * scale,
                        y: 0.,
                    },
                    Point {
                        x: 114. * scale,
                        y: 132. * scale,
                    },
                ),
                (
                    Point {
                        x: 0.,
                        y: 66. * scale,
                    },
                    Point {
                        x: 228. * scale,
                        y: 66. * scale,
                    },
                ),
            ],
            sample_outline: box_rect(108. * scale, 60. * scale, 12. * scale, 12. * scale),
            scale,
        }
    }

    pub fn frame(&self, pointer: Point, viewport: Size) -> Rect {
        let gap = 14. * self.scale;
        let x = if pointer.x + gap + self.size.width <= viewport.width {
            pointer.x + gap
        } else {
            pointer.x - self.size.width - gap
        };
        let y = if pointer.y + gap + self.size.height <= viewport.height {
            pointer.y + gap
        } else {
            pointer.y - self.size.height - gap
        };
        box_rect(
            x.clamp(0., (viewport.width - self.size.width).max(0.)),
            y.clamp(0., (viewport.height - self.size.height).max(0.)),
            self.size.width,
            self.size.height,
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MagnifierSample {
    /// Requested source rectangle, including transparent pixels beyond an edge.
    pub desired: Rect,
    /// Visible source pixels, in top-left pixel coordinates.
    pub source: Rect,
    /// Position of those pixels inside the lens in logical coordinates.
    pub destination: Rect,
}

pub fn magnifier_sample(pixel: Point, source: Size, zoom: u8, scale: f32) -> MagnifierSample {
    let zoom = zoom.clamp(4, 20) as f32;
    let width = (228. / zoom).round().max(1.).min(source.width);
    let height = (132. / zoom).round().max(1.).min(source.height);
    let desired = box_rect(
        pixel.x - (width / 2.).floor(),
        pixel.y - (height / 2.).floor(),
        width,
        height,
    );
    let left = desired.x.max(0.);
    let top = desired.y.max(0.);
    let right = (desired.x + width).min(source.width);
    let bottom = (desired.y + height).min(source.height);
    MagnifierSample {
        desired,
        source: box_rect(left, top, right - left, bottom - top),
        destination: box_rect(
            (left - desired.x) * zoom * scale,
            (top - desired.y) * zoom * scale,
            (right - left) * zoom * scale,
            (bottom - top) * zoom * scale,
        ),
    }
}

pub fn color_text(color: [u8; 3], rgb: bool) -> String {
    if rgb {
        format!("rgb({}, {}, {})", color[0], color[1], color[2])
    } else {
        format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2])
    }
}

pub fn magnifier_label(point: Point, color: [u8; 3], rgb: bool) -> String {
    format!(
        "({:.0}, {:.0})\n{}\n按 C 复制颜色值\n按 Shift 切换 RGB / HEX",
        point.x,
        point.y,
        color_text(color, rgb)
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolbarLabel {
    SizeValue,
    FontValue,
    CornerTitle,
    CornerValue,
    Error,
}

#[derive(Clone, Debug)]
pub struct ToolbarText {
    pub kind: ToolbarLabel,
    pub rect: Rect,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct ToolbarPresentation {
    pub size: Size,
    pub main_height: f32,
    pub panels: Vec<Rect>,
    pub controls: Vec<CaptureControl>,
    pub labels: Vec<ToolbarText>,
}

pub fn toolbar_size(style: &CaptureStyle, available_width: f32, scale: f32) -> Size {
    let width = (700. * scale).min(available_width);
    let main = if width / scale < 460. { 100. } else { 68. };
    let rows = usize::from(style.text_mode)
        + usize::from(style.round_enabled)
        + usize::from(style.error.is_some());
    Size {
        width,
        height: (main + if rows > 0 { 8. + rows as f32 * 36. } else { 0. }) * scale,
    }
}

/// The toolbar follows the selection's left edge and flips above it when the
/// space below is insufficient. This preserves the native macOS placement.
pub fn toolbar_frame(selection: Rect, viewport: Size, size: Size, scale: f32) -> Rect {
    let edge = 8. * scale;
    let gap = 14. * scale;
    let below = selection.y + selection.height + gap;
    Rect {
        x: selection
            .x
            .clamp(edge, (viewport.width - size.width - edge).max(edge)),
        y: if below + size.height < viewport.height - edge {
            below
        } else {
            (selection.y - size.height - gap).max(edge)
        },
        width: size.width,
        height: size.height,
    }
}

impl ToolbarPresentation {
    /// `width` is the actual native toolbar width, including interface scale.
    /// All returned rectangles are relative to that toolbar's top-left corner.
    pub fn new(width: f32, scale: f32, snapshot: &CaptureSnapshot) -> Self {
        let style = &snapshot.style;
        let size = Size {
            width,
            ..toolbar_size(style, width, scale)
        };
        let compact = size.width / scale >= 460.;
        let main = if compact { 68. } else { 100. };
        let at =
            |x, y, width, height| box_rect(x * scale, y * scale, width * scale, height * scale);
        let mut presentation = Self {
            size,
            main_height: main * scale,
            panels: vec![box_rect(0.5, 0.5, size.width - 1., main * scale - 1.)],
            controls: vec![],
            labels: vec![],
        };
        if size.height > main * scale {
            presentation.panels.push(box_rect(
                0.5,
                (main + 8.) * scale,
                size.width - 1.,
                size.height - (main + 8.) * scale - 0.5,
            ));
        }
        let mut control = |target, rect, label: &str, enabled, selected| {
            presentation.controls.push(CaptureControl {
                target,
                rect,
                label: label.into(),
                enabled,
                selected,
                value: None,
                range: None,
            });
        };
        let shape = if snapshot.tool == Some(CaptureTool::Ellipse) {
            CaptureTool::Ellipse
        } else {
            CaptureTool::Rectangle
        };
        let arrow = if snapshot.tool == Some(CaptureTool::Line) {
            CaptureTool::Line
        } else {
            CaptureTool::Arrow
        };
        let mut x = 6.;
        for (index, tool) in [
            None,
            Some(shape),
            Some(arrow),
            Some(CaptureTool::Pen),
            Some(CaptureTool::Text),
            Some(CaptureTool::Mosaic),
            Some(CaptureTool::Eraser),
            Some(CaptureTool::Highlight),
            Some(CaptureTool::RegionHighlight),
            Some(CaptureTool::Blur),
            Some(CaptureTool::Spotlight),
        ]
        .into_iter()
        .enumerate()
        {
            let (target, label, selected) = tool
                .map(|tool| {
                    (
                        CaptureTarget::Tool(tool),
                        tool.label(),
                        snapshot.tool == Some(tool),
                    )
                })
                .unwrap_or((
                    CaptureTarget::Command(CaptureCommand::Select),
                    "选择并移动",
                    snapshot.tool.is_none(),
                ));
            control(target, at(x, 4., 28., 28.), label, !snapshot.busy, selected);
            if index == 1 || index == 2 {
                control(
                    if index == 1 {
                        CaptureTarget::ShapeMenu
                    } else {
                        CaptureTarget::ArrowMenu
                    },
                    at(x + 28., 4., 14., 28.),
                    if index == 1 {
                        "形状"
                    } else {
                        "箭头和直线"
                    },
                    !snapshot.busy,
                    false,
                );
                x += 42.;
            } else {
                x += 30.;
            }
        }
        for (command, enabled) in [
            (CaptureCommand::Undo, snapshot.can_undo),
            (CaptureCommand::Redo, snapshot.can_redo),
        ] {
            control(
                CaptureTarget::Command(command),
                at(x, 4., 28., 28.),
                command.label(),
                !snapshot.busy && enabled,
                false,
            );
            x += 30.;
        }
        let y = if compact { 36. } else { 68. };
        let mut x = if compact { 194. } else { 6. };
        for (command, title, slot) in [
            (CaptureCommand::Cancel, "ESC", 38.),
            (CaptureCommand::Ocr, "OCR", 38.),
            (CaptureCommand::Scrolling, "长截图", 54.),
            (CaptureCommand::Clear, "清空", 40.),
            (CaptureCommand::Favorite, "收藏截图", 30.),
            (CaptureCommand::Pin, "贴图", 30.),
            (CaptureCommand::Save, "保存", 30.),
            (CaptureCommand::Copy, "复制", 30.),
        ] {
            control(
                CaptureTarget::Command(command),
                at(x, y, slot - 2., 28.),
                title,
                !snapshot.busy && (command != CaptureCommand::Clear || snapshot.can_undo),
                false,
            );
            x += slot;
        }
        let logical_width = size.width / scale;
        control(
            CaptureTarget::Color,
            at(logical_width - 40., 4., 28., 28.),
            "颜色 · 支持 RGB、HEX 和透明度",
            !snapshot.busy,
            false,
        );
        control(
            CaptureTarget::PaletteMenu,
            at(logical_width - 16., 4., 12., 28.),
            "颜色预设",
            !snapshot.busy,
            false,
        );
        let (title, value, min, max) = if style.text_mode {
            (
                "字号",
                style.font_size as f64,
                style.font_size.min(10) as f64,
                style.font_size.max(96) as f64,
            )
        } else if style.blur_mode {
            ("模糊", style.blur_radius as f64, 4., 64.)
        } else {
            ("线宽", style.stroke_width as f64, 1., 32.)
        };
        control(
            CaptureTarget::Size,
            at(12., 36., 86., 28.),
            title,
            !snapshot.busy,
            false,
        );
        control(
            CaptureTarget::Fill,
            at(148., 36., 32., 28.),
            "填充",
            !snapshot.busy && style.fill_enabled,
            style.filled,
        );
        let mut y = main + 12.;
        if style.text_mode {
            let family = (logical_width - 190.).clamp(60., 220.);
            control(
                CaptureTarget::Font,
                at(12., y, family, 28.),
                if style.font_family.is_empty() {
                    "默认字体"
                } else {
                    &style.font_family
                },
                !snapshot.busy,
                false,
            );
            for (target, label, offset, selected) in [
                (CaptureTarget::Bold, "B", 24., style.bold),
                (CaptureTarget::Italic, "I", 60., style.italic),
                (CaptureTarget::FontDecrease, "−", 100., false),
                (CaptureTarget::FontIncrease, "+", 172., false),
            ] {
                control(
                    target,
                    at(family + offset, y, 28., 28.),
                    label,
                    !snapshot.busy,
                    selected,
                );
            }
            presentation.labels.push(ToolbarText {
                kind: ToolbarLabel::FontValue,
                rect: at(family + 136., y, 28., 28.),
                text: style.font_size.to_string(),
            });
            y += 36.;
        }
        if style.round_enabled {
            let width = (logical_width - 112.).clamp(32., 160.);
            control(
                CaptureTarget::Round,
                at(56., y, width, 28.),
                "圆角",
                !snapshot.busy,
                false,
            );
            presentation.labels.push(ToolbarText {
                kind: ToolbarLabel::CornerTitle,
                rect: at(12., y, 36., 28.),
                text: "圆角".into(),
            });
            presentation.labels.push(ToolbarText {
                kind: ToolbarLabel::CornerValue,
                rect: at(64. + width, y, 36., 28.),
                text: format!("{:.0}", style.corner_radius),
            });
            y += 36.;
        }
        if let Some(error) = &style.error {
            presentation.labels.push(ToolbarText {
                kind: ToolbarLabel::Error,
                rect: at(12., y, logical_width - 24., 28.),
                text: error.clone(),
            });
        }
        presentation.labels.push(ToolbarText {
            kind: ToolbarLabel::SizeValue,
            rect: at(110., 36., 28., 28.),
            text: format!("{value:.0}"),
        });
        for control in &mut presentation.controls {
            if control.target == CaptureTarget::Size {
                control.value = Some(value);
                control.range = Some((min, max));
            } else if control.target == CaptureTarget::Round {
                control.value = Some(style.corner_radius as f64);
                control.range = Some((0., 100.));
            }
        }
        presentation
    }

    pub fn control(&self, target: CaptureTarget) -> Option<&CaptureControl> {
        self.controls
            .iter()
            .find(|control| control.target == target)
    }

    pub fn label(&self, kind: ToolbarLabel) -> Option<&ToolbarText> {
        self.labels.iter().find(|label| label.kind == kind)
    }
}

pub fn toolbar_icon(target: CaptureTarget) -> &'static str {
    match target {
        CaptureTarget::Tool(CaptureTool::Rectangle | CaptureTool::Ellipse) => "shapes",
        CaptureTarget::Tool(CaptureTool::Arrow | CaptureTool::Line) => "arrow",
        CaptureTarget::Tool(tool) => tool.name(),
        CaptureTarget::Command(
            CaptureCommand::Cancel
            | CaptureCommand::Ocr
            | CaptureCommand::Scrolling
            | CaptureCommand::Clear,
        ) => "",
        CaptureTarget::Command(command) => command.name(),
        _ => "",
    }
}

/// Button/menu intent after normalizing native modifiers. Repeated grouped
/// tools stay selected; other selected tools return to selection mode.
pub fn toolbar_intent(
    target: CaptureTarget,
    snapshot: &CaptureSnapshot,
    menu_choice: bool,
    shift: bool,
) -> Option<CaptureIntent> {
    if snapshot.busy {
        return None;
    }
    match target {
        CaptureTarget::Command(command) => {
            if matches!(command, CaptureCommand::Undo | CaptureCommand::Clear) && !snapshot.can_undo
                || command == CaptureCommand::Redo && !snapshot.can_redo
            {
                return None;
            }
            Some(CaptureIntent::Command(
                if command == CaptureCommand::Save && shift {
                    CaptureCommand::QuickSave
                } else {
                    command
                },
            ))
        }
        CaptureTarget::Tool(tool) if !menu_choice && snapshot.tool == Some(tool) => {
            if matches!(
                tool,
                CaptureTool::Rectangle
                    | CaptureTool::Ellipse
                    | CaptureTool::Arrow
                    | CaptureTool::Line
            ) {
                None
            } else {
                Some(CaptureIntent::Command(CaptureCommand::Select))
            }
        }
        CaptureTarget::Tool(tool) => Some(CaptureIntent::Tool(tool)),
        _ => None,
    }
}

#[derive(Clone, Debug)]
pub struct PalettePresentation {
    pub size: Size,
    pub title: Rect,
    pub controls: Vec<CaptureControl>,
}

impl PalettePresentation {
    pub fn new(
        style: &CaptureStyle,
        selected: Option<usize>,
        busy: bool,
        pending: bool,
        scale: f32,
    ) -> Self {
        let selected = selected.filter(|index| *index < style.presets.len());
        let rows = style.presets.len().div_ceil(7).max(1);
        let bottom = 30. + rows as f32 * 30.;
        let at =
            |x, y, width, height| box_rect(x * scale, y * scale, width * scale, height * scale);
        let mut presentation = Self {
            size: Size {
                width: 248. * scale,
                height: (bottom + 76.) * scale,
            },
            title: at(12., 6., 224., 20.),
            controls: vec![],
        };
        let mut control = |target, rect, label: &str, enabled, selected| {
            presentation.controls.push(CaptureControl {
                target,
                rect,
                label: label.into(),
                enabled,
                selected,
                value: None,
                range: None,
            });
        };
        control(
            CaptureTarget::PaletteClose,
            at(220., 6., 20., 20.),
            "×",
            true,
            false,
        );
        for index in 0..style.presets.len() {
            control(
                CaptureTarget::Palette(index),
                at(
                    12. + (index % 7) as f32 * 32.,
                    30. + (index / 7) as f32 * 30.,
                    24.,
                    24.,
                ),
                "预设颜色",
                !busy,
                selected == Some(index),
            );
        }
        let editable = !busy && !pending;
        for (target, title, x, y, width, height, enabled) in [
            (
                CaptureTarget::SystemColor,
                "系统调色板",
                12.,
                bottom + 4.,
                224.,
                30.,
                !busy,
            ),
            (
                CaptureTarget::AddColor,
                "新增",
                12.,
                bottom + 42.,
                72.,
                26.,
                editable && !style.presets.contains(&style.color),
            ),
            (
                CaptureTarget::ReplaceColor,
                "修改",
                88.,
                bottom + 42.,
                72.,
                26.,
                editable
                    && selected.is_some_and(|index| {
                        style.presets[index] != style.color
                            && !style
                                .presets
                                .iter()
                                .enumerate()
                                .any(|(i, color)| i != index && *color == style.color)
                    }),
            ),
            (
                CaptureTarget::DeleteColor,
                "删除",
                164.,
                bottom + 42.,
                72.,
                26.,
                editable && selected.is_some(),
            ),
        ] {
            control(target, at(x, y, width, height), title, enabled, false);
        }
        presentation
    }

    pub fn control(&self, target: CaptureTarget) -> Option<&CaptureControl> {
        self.controls
            .iter()
            .find(|control| control.target == target)
    }

    /// The screen and toolbar use top-left logical coordinates. Platform hosts
    /// convert their screen origin convention before calling this method.
    pub fn frame(&self, toolbar: Rect, screen: Rect, scale: f32) -> Rect {
        let below = toolbar.y + toolbar.height + 4. * scale;
        let above = toolbar.y - self.size.height - 4. * scale;
        Rect {
            x: (toolbar.x + toolbar.width - self.size.width).clamp(
                screen.x,
                (screen.x + screen.width - self.size.width).max(screen.x),
            ),
            y: if below + self.size.height <= screen.y + screen.height {
                below
            } else {
                above
                    .min(screen.y + screen.height - self.size.height)
                    .max(screen.y)
            },
            width: self.size.width,
            height: self.size.height,
        }
    }
}
