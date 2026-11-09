use std::collections::VecDeque;
use std::sync::Arc;

use super::editor_geometry::AnnotationHandle;
use super::editor_toolbar::{
    CaptureEditorControlAction, CaptureEditorStyleChange, CaptureEditorStylePresentation,
};
use crate::domain::capture::{AnnotationCommand, LogicalPoint, LogicalRect};

const HISTORY_LIMIT: usize = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnotationTool {
    Rectangle,
    Ellipse,
    Arrow,
    Line,
    Pen,
    Text,
    Highlight,
    RegionHighlight,
    Blur,
    Spotlight,
    Mosaic,
    Eraser,
}

impl AnnotationTool {
    pub fn parse(tool: &str) -> Option<Self> {
        Some(match tool {
            "rectangle" => Self::Rectangle,
            "ellipse" => Self::Ellipse,
            "arrow" => Self::Arrow,
            "line" => Self::Line,
            "pen" => Self::Pen,
            "text" => Self::Text,
            "highlight" => Self::Highlight,
            "region-highlight" => Self::RegionHighlight,
            "blur" => Self::Blur,
            "spotlight" => Self::Spotlight,
            "mosaic" => Self::Mosaic,
            "eraser" => Self::Eraser,
            _ => return None,
        })
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Rectangle => "rectangle",
            Self::Ellipse => "ellipse",
            Self::Arrow => "arrow",
            Self::Line => "line",
            Self::Pen => "pen",
            Self::Text => "text",
            Self::Highlight => "highlight",
            Self::RegionHighlight => "region-highlight",
            Self::Blur => "blur",
            Self::Spotlight => "spotlight",
            Self::Mosaic => "mosaic",
            Self::Eraser => "eraser",
        }
    }
    fn point_stroke(self) -> bool {
        matches!(
            self,
            Self::Pen | Self::Highlight | Self::Mosaic | Self::Eraser
        )
    }
}

#[derive(Clone, Debug)]
pub struct EditorTextDraft {
    pub position: LogicalPoint,
    pub text: String,
    pub replacing: Option<usize>,
}

enum Gesture {
    Draw {
        tool: AnnotationTool,
        start: LogicalPoint,
        end: LogicalPoint,
        points: Vec<LogicalPoint>,
        constrained: bool,
    },
    Move {
        index: usize,
        start: LogicalPoint,
        original: Arc<AnnotationCommand>,
        preview: AnnotationCommand,
        resize: Option<AnnotationHandle>,
    },
}

type Annotations = Arc<Vec<Arc<AnnotationCommand>>>;

/// Native controls and canvas input share one editor state, independent of a window framework.
pub struct CaptureEditor {
    pub selection: LogicalRect,
    pub tool: Option<AnnotationTool>,
    pub styling: CaptureEditorStylePresentation,
    pub selected: Option<usize>,
    pub text: Option<EditorTextDraft>,
    annotations: Annotations,
    undo: VecDeque<Annotations>,
    redo: Vec<Annotations>,
    gesture: Option<Gesture>,
    pub revision: u64,
    pub image_revision: u64,
}

impl CaptureEditor {
    pub fn new(
        selection: LogicalRect,
        tool: Option<AnnotationTool>,
        styling: CaptureEditorStylePresentation,
    ) -> Self {
        let mut editor = Self {
            selection,
            tool,
            styling,
            selected: None,
            text: None,
            annotations: Arc::new(Vec::new()),
            undo: VecDeque::new(),
            redo: Vec::new(),
            gesture: None,
            revision: 0,
            image_revision: 0,
        };
        editor.refresh_modes();
        editor
    }

    pub fn annotations(&self) -> impl DoubleEndedIterator<Item = &AnnotationCommand> {
        self.annotations.iter().map(AsRef::as_ref)
    }
    pub fn export_annotations(&self) -> Vec<AnnotationCommand> {
        self.annotations().cloned().collect()
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    pub fn selection_changed(&mut self, selection: LogicalRect) -> bool {
        if self.selection == selection {
            return false;
        }
        self.selection = selection;
        self.image_revision = self.image_revision.wrapping_add(1);
        self.revision = self.revision.wrapping_add(1);
        true
    }

    fn replace(&mut self, annotations: Vec<Arc<AnnotationCommand>>) {
        if *self.annotations == annotations {
            return;
        }
        self.undo.push_back(self.annotations.clone());
        if self.undo.len() > HISTORY_LIMIT {
            self.undo.pop_front();
        }
        self.annotations = Arc::new(annotations);
        self.image_revision = self.image_revision.wrapping_add(1);
        self.redo.clear();
        self.revision = self.revision.wrapping_add(1);
    }
    fn append(&mut self, annotation: AnnotationCommand) {
        let mut annotations = self.annotations.as_ref().clone();
        annotations.push(Arc::new(annotation));
        self.replace(annotations);
    }
    fn replace_selected(&mut self, index: usize, annotation: AnnotationCommand) {
        let mut annotations = self.annotations.as_ref().clone();
        if let Some(current) = annotations.get_mut(index) {
            *current = Arc::new(annotation);
            self.replace(annotations);
        }
    }

    pub fn control(&mut self, action: &CaptureEditorControlAction) -> bool {
        match action {
            CaptureEditorControlAction::Command(command) => match *command {
                "select" => {
                    self.commit_text();
                    self.tool = None;
                    self.gesture = None;
                    self.selected = None;
                }
                "undo" => {
                    if self.text.take().is_none() {
                        self.gesture = None;
                        self.selected = None;
                        if let Some(previous) = self.undo.pop_back() {
                            self.redo.push(self.annotations.clone());
                            self.annotations = previous;
                            self.image_revision = self.image_revision.wrapping_add(1);
                        }
                    }
                }
                "redo" => {
                    self.text = None;
                    self.gesture = None;
                    self.selected = None;
                    if let Some(next) = self.redo.pop() {
                        self.undo.push_back(self.annotations.clone());
                        self.annotations = next;
                        self.image_revision = self.image_revision.wrapping_add(1);
                    }
                }
                "clear" => {
                    self.text = None;
                    self.gesture = None;
                    self.selected = None;
                    self.annotations = Arc::new(Vec::new());
                    self.undo.clear();
                    self.redo.clear();
                    self.image_revision = self.image_revision.wrapping_add(1);
                }
                "delete" => {
                    if let Some(index) = self.selected.take() {
                        let mut annotations = self.annotations.as_ref().clone();
                        annotations.remove(index);
                        self.replace(annotations);
                    }
                }
                _ => {
                    let Some(tool) = AnnotationTool::parse(command) else {
                        return false;
                    };
                    self.commit_text();
                    self.gesture = None;
                    self.selected = None;
                    self.tool = Some(tool);
                }
            },
            CaptureEditorControlAction::Style { change, .. } => {
                let style = &mut self.styling.style;
                match change {
                    CaptureEditorStyleChange::Color(color) => style.color = *color,
                    CaptureEditorStyleChange::StrokeWidth(width) => style.stroke_width = *width,
                    CaptureEditorStyleChange::Filled(filled) => style.filled = *filled,
                    CaptureEditorStyleChange::CornerRadius(radius) => style.corner_radius = *radius,
                    CaptureEditorStyleChange::BlurRadius(radius) => style.blur_radius = *radius,
                    CaptureEditorStyleChange::FontFamily(family) => {
                        style.font_family = family.clone()
                    }
                    CaptureEditorStyleChange::Bold(bold) => style.bold = *bold,
                    CaptureEditorStyleChange::Italic(italic) => style.italic = *italic,
                    CaptureEditorStyleChange::FontSize(size) => self.styling.font_size = *size,
                }
                if self.text.is_none() {
                    if let Some(index) = self.selected {
                        if let Some(annotation) = self.annotations.get(index) {
                            let mut annotation = annotation.as_ref().clone();
                            apply_style(&mut annotation, &self.styling);
                            self.replace_selected(index, annotation);
                        }
                    }
                }
            }
            CaptureEditorControlAction::Presets(_) => return false,
        }
        self.revision = self.revision.wrapping_add(1);
        self.refresh_modes();
        true
    }

    pub fn pointer_down(&mut self, point: LogicalPoint, detail: u32, select_existing: bool) {
        self.commit_text();
        let point = self.clamp(point);
        if let Some(index) = self.selected {
            let original = self.annotations[index].clone();
            if let Some(handle) = super::editor_geometry::handle_at(&original, &point) {
                if detail >= 2
                    && matches!(
                        handle,
                        AnnotationHandle::Control1 | AnnotationHandle::Control2
                    )
                {
                    let mut annotation = original.as_ref().clone();
                    if let AnnotationCommand::Arrow {
                        control1, control2, ..
                    } = &mut annotation
                    {
                        *control1 = None;
                        *control2 = None;
                    }
                    self.replace_selected(index, annotation);
                    self.gesture = None;
                } else {
                    self.gesture = Some(Gesture::Move {
                        index,
                        start: point,
                        preview: original.as_ref().clone(),
                        original,
                        resize: Some(handle),
                    });
                }
                self.revision = self.revision.wrapping_add(1);
                return;
            }
        }
        if self.tool.is_none() || select_existing {
            let hit = self
                .annotations
                .iter()
                .enumerate()
                .rev()
                .find(|(_, annotation)| super::editor_geometry::hit(annotation, &point, 6.0))
                .map(|(index, _)| index);
            self.selected = hit;
            if let Some(index) = hit {
                let annotation = self.annotations[index].clone();
                self.adopt_style(&annotation);
                if detail >= 2 {
                    if let AnnotationCommand::Text { position, text, .. } = annotation.as_ref() {
                        self.text = Some(EditorTextDraft {
                            position: position.clone(),
                            text: text.clone(),
                            replacing: Some(index),
                        });
                        self.refresh_modes();
                        return;
                    }
                }
                self.gesture = Some(Gesture::Move {
                    index,
                    start: point,
                    original: annotation.clone(),
                    preview: annotation.as_ref().clone(),
                    resize: None,
                });
            }
        } else if self.tool == Some(AnnotationTool::Text) {
            self.selected = None;
            self.text = Some(EditorTextDraft {
                position: point,
                text: String::new(),
                replacing: None,
            });
        } else if let Some(tool) = self.tool {
            self.selected = None;
            self.gesture = Some(Gesture::Draw {
                tool,
                start: point.clone(),
                end: point.clone(),
                points: if tool.point_stroke() {
                    vec![point]
                } else {
                    Vec::new()
                },
                constrained: false,
            });
        }
        self.revision = self.revision.wrapping_add(1);
        self.refresh_modes();
    }

    pub fn pointer_move(&mut self, point: LogicalPoint, constrained: bool) {
        let point = self.clamp(point);
        let selection_bounds = LogicalRect {
            x: 0.0,
            y: 0.0,
            width: self.selection.width,
            height: self.selection.height,
        };
        match self.gesture.as_mut() {
            Some(Gesture::Draw {
                tool,
                start,
                end,
                points,
                constrained: current,
            }) => {
                *end = constrain(*tool, start, &point, constrained);
                *current = constrained;
                if tool.point_stroke() && !constrained && points.last() != Some(&point) {
                    points.push(point);
                }
            }
            Some(Gesture::Move {
                start,
                original,
                preview,
                resize,
                ..
            }) => {
                let mut delta = LogicalPoint {
                    x: point.x - start.x,
                    y: point.y - start.y,
                };
                if let Some(handle) = resize {
                    *preview = super::editor_geometry::resized(
                        original,
                        *handle,
                        &delta,
                        &selection_bounds,
                        constrained,
                    );
                } else {
                    if constrained && delta.x.abs() > delta.y.abs() {
                        delta.y = 0.0;
                    } else if constrained && delta.y.abs() > delta.x.abs() {
                        delta.x = 0.0;
                    }
                    *preview = super::editor_geometry::moved(original, &delta);
                }
            }
            None => return,
        }
        self.revision = self.revision.wrapping_add(1);
    }
    pub fn draft_annotation(&self) -> Option<AnnotationCommand> {
        match self.gesture.as_ref()? {
            Gesture::Draw {
                tool,
                start,
                end,
                points,
                constrained,
            } => Some(drawing(
                *tool,
                start.clone(),
                end.clone(),
                if *constrained && tool.point_stroke() {
                    vec![start.clone(), end.clone()]
                } else {
                    points.clone()
                },
                &self.styling,
            )),
            Gesture::Move { preview, .. } => Some(preview.clone()),
        }
    }
    pub fn moving_index(&self) -> Option<usize> {
        match self.gesture.as_ref() {
            Some(Gesture::Move { index, .. }) => Some(*index),
            _ => None,
        }
    }
    pub fn selected_bounds(&self) -> Option<LogicalRect> {
        if let Some(Gesture::Move { preview, .. }) = self.gesture.as_ref() {
            return Some(super::editor_geometry::bounds(preview));
        }
        self.selected
            .and_then(|index| self.annotations.get(index))
            .map(|annotation| super::editor_geometry::bounds(annotation))
    }
    pub fn selected_handles(&self) -> Vec<(AnnotationHandle, LogicalPoint)> {
        if let Some(Gesture::Move { preview, .. }) = self.gesture.as_ref() {
            return super::editor_geometry::handles(preview);
        }
        self.selected
            .and_then(|index| self.annotations.get(index))
            .map(|annotation| super::editor_geometry::handles(annotation))
            .unwrap_or_default()
    }
    pub fn nudge_selected(&mut self, delta: LogicalPoint) -> bool {
        let Some(index) = self.selected else {
            return false;
        };
        let annotation = super::editor_geometry::moved(&self.annotations[index], &delta);
        self.replace_selected(index, annotation);
        true
    }
    pub fn pointer_up(&mut self, point: LogicalPoint, constrained: bool) {
        self.pointer_move(point, constrained);
        let draft = self.draft_annotation();
        match self.gesture.take() {
            Some(Gesture::Draw { tool, .. }) => {
                if let Some(annotation) = draft.filter(valid_annotation) {
                    self.append(annotation);
                    self.selected =
                        (tool == AnnotationTool::Rectangle).then_some(self.annotations.len() - 1);
                }
            }
            Some(Gesture::Move { index, .. }) => {
                if let Some(annotation) = draft {
                    self.adopt_style(&annotation);
                    self.replace_selected(index, annotation);
                }
            }
            None => {}
        }
        self.revision = self.revision.wrapping_add(1);
        self.refresh_modes();
    }
    pub fn update_text(&mut self, text: String) {
        if let Some(draft) = &mut self.text {
            draft.text = text;
        }
    }
    pub fn commit_text(&mut self) {
        let Some(draft) = self.text.take() else {
            return;
        };
        if !draft.text.trim().is_empty() {
            let style = &self.styling.style;
            let annotation = AnnotationCommand::Text {
                position: draft.position,
                text: draft.text,
                color: style.color,
                font_size: self.styling.font_size,
                font_family: style.font_family.clone(),
                bold: style.bold,
                italic: style.italic,
            };
            if let Some(index) = draft.replacing {
                self.replace_selected(index, annotation);
                self.selected = Some(index);
            } else {
                self.append(annotation);
                self.selected = None;
            }
        } else {
            if let Some(index) = draft.replacing {
                let mut annotations = self.annotations.as_ref().clone();
                annotations.remove(index);
                self.replace(annotations);
            }
            self.selected = None;
        }
        self.revision = self.revision.wrapping_add(1);
        self.refresh_modes();
    }
    pub fn escape(&mut self) -> bool {
        if self.text.take().is_some() {
        } else if self.gesture.take().is_some() {
        } else if self.selected.take().is_some() {
        } else if self.tool.take().is_some() {
        } else {
            return false;
        }
        self.revision = self.revision.wrapping_add(1);
        self.refresh_modes();
        true
    }
    fn clamp(&self, point: LogicalPoint) -> LogicalPoint {
        LogicalPoint {
            x: point.x.clamp(0.0, self.selection.width),
            y: point.y.clamp(0.0, self.selection.height),
        }
    }
    fn refresh_modes(&mut self) {
        let selected = self
            .selected
            .and_then(|index| self.annotations.get(index))
            .map(|annotation| tool_of(annotation));
        self.styling.text_mode = self.text.is_some()
            || self.tool == Some(AnnotationTool::Text)
            || selected == Some(AnnotationTool::Text);
        self.styling.fill_enabled = [self.tool, selected].iter().any(|tool| {
            matches!(
                tool,
                Some(AnnotationTool::Rectangle | AnnotationTool::Ellipse)
            )
        });
        self.styling.round_enabled = [self.tool, selected].iter().any(|tool| {
            matches!(
                tool,
                Some(
                    AnnotationTool::Rectangle
                        | AnnotationTool::RegionHighlight
                        | AnnotationTool::Blur
                        | AnnotationTool::Spotlight
                )
            )
        });
        self.styling.blur_mode =
            self.tool == Some(AnnotationTool::Blur) || selected == Some(AnnotationTool::Blur);
    }
    fn adopt_style(&mut self, annotation: &AnnotationCommand) {
        let style = &mut self.styling.style;
        match annotation {
            AnnotationCommand::Rectangle {
                color,
                stroke_width,
                filled,
                corner_radius,
                ..
            } => {
                style.color = *color;
                style.stroke_width = *stroke_width;
                style.filled = *filled;
                style.corner_radius = *corner_radius;
            }
            AnnotationCommand::Ellipse {
                color,
                stroke_width,
                filled,
                ..
            } => {
                style.color = *color;
                style.stroke_width = *stroke_width;
                style.filled = *filled;
            }
            AnnotationCommand::Arrow {
                color,
                stroke_width,
                ..
            }
            | AnnotationCommand::Line {
                color,
                stroke_width,
                ..
            }
            | AnnotationCommand::Freehand {
                color,
                stroke_width,
                ..
            }
            | AnnotationCommand::Highlight {
                color,
                stroke_width,
                ..
            } => {
                style.color = *color;
                style.stroke_width = *stroke_width;
                style.filled = false;
            }
            AnnotationCommand::Mosaic { stroke_width, .. } => {
                style.stroke_width = ((*stroke_width as f64 / 5.0).round() as u32).max(1)
            }
            AnnotationCommand::Eraser { stroke_width, .. } => {
                style.stroke_width = ((*stroke_width as f64 / 4.0).round() as u32).max(1)
            }
            AnnotationCommand::Text {
                color,
                font_size,
                font_family,
                bold,
                italic,
                ..
            } => {
                style.color = *color;
                self.styling.font_size = *font_size;
                style.font_family = font_family.clone();
                style.bold = *bold;
                style.italic = *italic;
            }
            AnnotationCommand::RegionHighlight {
                color,
                corner_radius,
                ..
            } => {
                style.color = *color;
                style.corner_radius = *corner_radius;
            }
            AnnotationCommand::Blur {
                radius,
                corner_radius,
                ..
            } => {
                style.blur_radius = *radius;
                style.corner_radius = *corner_radius;
            }
            AnnotationCommand::Spotlight { corner_radius, .. } => {
                style.corner_radius = *corner_radius
            }
        }
    }
}

fn tool_of(annotation: &AnnotationCommand) -> AnnotationTool {
    match annotation {
        AnnotationCommand::Rectangle { .. } => AnnotationTool::Rectangle,
        AnnotationCommand::Ellipse { .. } => AnnotationTool::Ellipse,
        AnnotationCommand::Arrow { .. } => AnnotationTool::Arrow,
        AnnotationCommand::Line { .. } => AnnotationTool::Line,
        AnnotationCommand::Freehand { .. } => AnnotationTool::Pen,
        AnnotationCommand::Text { .. } => AnnotationTool::Text,
        AnnotationCommand::Highlight { .. } => AnnotationTool::Highlight,
        AnnotationCommand::RegionHighlight { .. } => AnnotationTool::RegionHighlight,
        AnnotationCommand::Blur { .. } => AnnotationTool::Blur,
        AnnotationCommand::Spotlight { .. } => AnnotationTool::Spotlight,
        AnnotationCommand::Mosaic { .. } => AnnotationTool::Mosaic,
        AnnotationCommand::Eraser { .. } => AnnotationTool::Eraser,
    }
}

pub(super) fn constrain(
    tool: AnnotationTool,
    start: &LogicalPoint,
    end: &LogicalPoint,
    enabled: bool,
) -> LogicalPoint {
    if !enabled || matches!(tool, AnnotationTool::Mosaic | AnnotationTool::Eraser) {
        return end.clone();
    }
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    if matches!(
        tool,
        AnnotationTool::Line
            | AnnotationTool::Arrow
            | AnnotationTool::Pen
            | AnnotationTool::Highlight
    ) {
        if dx.abs() >= dy.abs() * 2.0 {
            return LogicalPoint {
                x: end.x,
                y: start.y,
            };
        }
        if dy.abs() >= dx.abs() * 2.0 {
            return LogicalPoint {
                x: start.x,
                y: end.y,
            };
        }
    }
    let size = dx.abs().min(dy.abs());
    LogicalPoint {
        x: start.x + dx.signum() * size,
        y: start.y + dy.signum() * size,
    }
}

fn drawing(
    tool: AnnotationTool,
    start: LogicalPoint,
    end: LogicalPoint,
    points: Vec<LogicalPoint>,
    styling: &CaptureEditorStylePresentation,
) -> AnnotationCommand {
    let style = &styling.style;
    let rect = LogicalRect {
        x: start.x.min(end.x),
        y: start.y.min(end.y),
        width: (end.x - start.x).abs(),
        height: (end.y - start.y).abs(),
    };
    let color = style.color;
    let stroke_width = style.stroke_width;
    match tool {
        AnnotationTool::Rectangle => AnnotationCommand::Rectangle {
            rect,
            color,
            stroke_width,
            filled: style.filled,
            corner_radius: style.corner_radius,
        },
        AnnotationTool::Ellipse => AnnotationCommand::Ellipse {
            rect,
            color,
            stroke_width,
            filled: style.filled,
        },
        AnnotationTool::RegionHighlight => AnnotationCommand::RegionHighlight {
            rect,
            color,
            corner_radius: style.corner_radius,
        },
        AnnotationTool::Blur => AnnotationCommand::Blur {
            rect,
            radius: style.blur_radius,
            corner_radius: style.corner_radius,
        },
        AnnotationTool::Spotlight => AnnotationCommand::Spotlight {
            rect,
            corner_radius: style.corner_radius,
        },
        AnnotationTool::Pen => AnnotationCommand::Freehand {
            points,
            color,
            stroke_width,
        },
        AnnotationTool::Highlight => AnnotationCommand::Highlight {
            points,
            color: [color[0], color[1], color[2], 96],
            stroke_width,
        },
        AnnotationTool::Mosaic => AnnotationCommand::Mosaic {
            points,
            stroke_width: (stroke_width * 5).max(18),
            block_size: (stroke_width * 3).max(6),
        },
        AnnotationTool::Eraser => AnnotationCommand::Eraser {
            points,
            stroke_width: (stroke_width * 4).max(14),
        },
        AnnotationTool::Line => AnnotationCommand::Line {
            start,
            end,
            color,
            stroke_width,
        },
        AnnotationTool::Arrow => AnnotationCommand::Arrow {
            start,
            end,
            color,
            stroke_width,
            control1: None,
            control2: None,
        },
        AnnotationTool::Text => unreachable!("Text input starts a text draft"),
    }
}

fn valid_annotation(annotation: &AnnotationCommand) -> bool {
    match annotation {
        AnnotationCommand::Rectangle { rect, .. }
        | AnnotationCommand::Ellipse { rect, .. }
        | AnnotationCommand::RegionHighlight { rect, .. }
        | AnnotationCommand::Blur { rect, .. }
        | AnnotationCommand::Spotlight { rect, .. } => rect.width >= 4.0 && rect.height >= 4.0,
        AnnotationCommand::Line { start, end, .. }
        | AnnotationCommand::Arrow { start, end, .. } => {
            (end.x - start.x).hypot(end.y - start.y) >= 4.0
        }
        AnnotationCommand::Freehand { points, .. }
        | AnnotationCommand::Highlight { points, .. } => {
            points
                .windows(2)
                .map(|pair| (pair[1].x - pair[0].x).hypot(pair[1].y - pair[0].y))
                .sum::<f64>()
                >= 4.0
        }
        AnnotationCommand::Mosaic { points, .. } | AnnotationCommand::Eraser { points, .. } => {
            !points.is_empty()
        }
        AnnotationCommand::Text {
            text, font_size, ..
        } => !text.trim().is_empty() && *font_size > 0,
    }
}

fn apply_style(annotation: &mut AnnotationCommand, styling: &CaptureEditorStylePresentation) {
    let style = &styling.style;
    match annotation {
        AnnotationCommand::Rectangle {
            color,
            stroke_width,
            filled,
            corner_radius,
            ..
        } => {
            *color = style.color;
            *stroke_width = style.stroke_width;
            *filled = style.filled;
            *corner_radius = style.corner_radius;
        }
        AnnotationCommand::Ellipse {
            color,
            stroke_width,
            filled,
            ..
        } => {
            *color = style.color;
            *stroke_width = style.stroke_width;
            *filled = style.filled;
        }
        AnnotationCommand::Arrow {
            color,
            stroke_width,
            ..
        }
        | AnnotationCommand::Line {
            color,
            stroke_width,
            ..
        }
        | AnnotationCommand::Freehand {
            color,
            stroke_width,
            ..
        } => {
            *color = style.color;
            *stroke_width = style.stroke_width;
        }
        AnnotationCommand::Highlight {
            color,
            stroke_width,
            ..
        } => {
            *color = [style.color[0], style.color[1], style.color[2], 96];
            *stroke_width = style.stroke_width;
        }
        AnnotationCommand::Mosaic {
            stroke_width,
            block_size,
            ..
        } => {
            *stroke_width = (style.stroke_width * 5).max(18);
            *block_size = (style.stroke_width * 3).max(6);
        }
        AnnotationCommand::Eraser { stroke_width, .. } => {
            *stroke_width = (style.stroke_width * 4).max(14)
        }
        AnnotationCommand::Text {
            color,
            font_size,
            font_family,
            bold,
            italic,
            ..
        } => {
            *color = style.color;
            *font_size = styling.font_size;
            *font_family = style.font_family.clone();
            *bold = style.bold;
            *italic = style.italic;
        }
        AnnotationCommand::RegionHighlight {
            color,
            corner_radius,
            ..
        } => {
            *color = style.color;
            *corner_radius = style.corner_radius;
        }
        AnnotationCommand::Blur {
            radius,
            corner_radius,
            ..
        } => {
            *radius = style.blur_radius;
            *corner_radius = style.corner_radius;
        }
        AnnotationCommand::Spotlight { corner_radius, .. } => *corner_radius = style.corner_radius,
    }
}

#[cfg(test)]
#[path = "editor_test.rs"]
mod tests;
