//! Compact scrolling-capture controls shared by native desktop hosts.
use crate::{
    theme::palette, Appearance, DrawCommand, FontFamily, Key, Modifiers, Point, RasterImage, Rect,
    Scene, Size, TextAlign,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollingAction {
    Undo,
    Preview,
    Copy,
    Save,
    Pin,
    Complete,
    Cancel,
    Resume,
}
impl ScrollingAction {
    pub fn label(self) -> &'static str {
        match self {
            Self::Undo => "撤销",
            Self::Preview => "预览",
            Self::Copy => "复制",
            Self::Save => "另存为",
            Self::Pin => "贴图",
            Self::Complete => "完成",
            Self::Cancel => "取消",
            Self::Resume => "重试",
        }
    }
}
#[derive(Clone, Debug)]
pub struct ScrollingSnapshot {
    pub appearance: Appearance,
    pub width: u32,
    pub height: u32,
    pub can_undo: bool,
    pub preview_available: bool,
    pub preview_expanded: bool,
    pub working: bool,
    pub hint: Option<String>,
    pub error: Option<String>,
}
#[derive(Clone, Debug)]
pub struct ScrollingControl {
    pub action: ScrollingAction,
    pub rect: Rect,
    pub label: String,
    pub enabled: bool,
}
pub struct ScrollingFrame {
    pub scene: Scene,
    pub controls: Vec<ScrollingControl>,
    pub title: Rect,
    pub dimensions: Rect,
    pub status: Rect,
    pub retry: Rect,
}
pub struct ScrollingUi {
    snapshot: ScrollingSnapshot,
    controls: Vec<ScrollingControl>,
    focus: Option<ScrollingAction>,
    hover: Option<ScrollingAction>,
}
impl ScrollingUi {
    pub fn new(snapshot: ScrollingSnapshot) -> Self {
        Self {
            snapshot,
            controls: vec![],
            focus: None,
            hover: None,
        }
    }
    pub fn snapshot(&self) -> &ScrollingSnapshot {
        &self.snapshot
    }
    pub fn replace(&mut self, snapshot: ScrollingSnapshot) {
        self.snapshot = snapshot;
    }
    pub fn focused(&self) -> Option<ScrollingAction> {
        self.focus
    }
    pub fn focus(&mut self, action: ScrollingAction) {
        if self
            .controls
            .iter()
            .any(|c| c.action == action && c.enabled)
        {
            self.focus = Some(action);
        }
    }
    pub fn activate(&self, action: ScrollingAction) -> Option<ScrollingAction> {
        self.controls
            .iter()
            .any(|c| c.action == action && c.enabled)
            .then_some(action)
    }
    pub fn pointer(&self, point: Point) -> Option<ScrollingAction> {
        self.controls
            .iter()
            .find(|c| c.enabled && c.rect.contains(point))
            .map(|c| c.action)
    }
    pub fn hovered(&self) -> Option<ScrollingAction> {
        self.hover
    }
    pub fn hover(&mut self, point: Option<Point>) -> bool {
        let next = point.and_then(|point| self.pointer(point));
        let changed = self.hover != next;
        self.hover = next;
        changed
    }
    pub fn status_text(&self) -> &str {
        if self.snapshot.working {
            "正在处理…"
        } else {
            self.snapshot
                .error
                .as_deref()
                .or(self.snapshot.hint.as_deref())
                .unwrap_or("手动滚动页面，完成后复制并保存 PNG。")
        }
    }
    pub fn key(&mut self, key: Key, m: Modifiers) -> Option<ScrollingAction> {
        if key == Key::Tab {
            let order: Vec<_> = self
                .controls
                .iter()
                .filter(|c| c.enabled)
                .map(|c| c.action)
                .collect();
            if !order.is_empty() {
                let index = self.focus.and_then(|f| order.iter().position(|a| *a == f));
                self.focus = Some(
                    order[if m.shift {
                        index.map_or(order.len() - 1, |i| (i + order.len() - 1) % order.len())
                    } else {
                        index.map_or(0, |i| (i + 1) % order.len())
                    }],
                );
            }
            return None;
        }
        let action = match key {
            Key::Escape => ScrollingAction::Cancel,
            Key::Enter => self.focus.unwrap_or(ScrollingAction::Complete),
            Key::Character('z' | 'Z') if m.control || m.meta => ScrollingAction::Undo,
            Key::Space => self.focus?,
            _ => return None,
        };
        self.activate(action)
    }
    pub fn frame(&mut self, scale: f32) -> ScrollingFrame {
        let scale = if scale.is_finite() {
            scale.clamp(0.5, 3.)
        } else {
            1.
        };
        let p = palette(
            self.snapshot.appearance.theme,
            self.snapshot.appearance.dark,
        );
        let full = Rect {
            x: 0.,
            y: 0.,
            width: 408.,
            height: 132.,
        };
        let mut scene = Scene {
            size: Size {
                width: 408.,
                height: 132.,
            },
            commands: vec![
                DrawCommand::FillRect {
                    rect: full,
                    radius: 13.,
                    color: p.surface,
                },
                DrawCommand::StrokeRect {
                    rect: Rect {
                        x: 0.5,
                        y: 0.5,
                        width: 407.,
                        height: 131.,
                    },
                    radius: 13.,
                    width: 1.,
                    color: p.line,
                },
            ],
        };
        let title = Rect {
            x: 13.,
            y: 13.,
            width: 80.,
            height: 18.75,
        };
        let dimensions = Rect {
            x: 130.,
            y: 13.,
            width: 265.,
            height: 18.75,
        };
        let status = Rect {
            x: 13.,
            y: 79.75,
            width: if self.snapshot.error.is_some() {
                328.
            } else {
                382.
            },
            height: 39.25,
        };
        let retry = Rect {
            x: 349.,
            y: 79.75,
            width: 46.,
            height: 16.,
        };
        let mut heading = text(title, "长截图", 12.5, p.ink);
        if let DrawCommand::Text { weight, .. } = &mut heading {
            *weight = 600;
        }
        scene.commands.push(heading);
        let mut counts = text(
            dimensions,
            format!("{} × {}", self.snapshot.width, self.snapshot.height),
            10.5,
            p.muted,
        );
        if let DrawCommand::Text { align, .. } = &mut counts {
            *align = TextAlign::Right;
        }
        scene.commands.push(counts);
        let mut controls = vec![];
        let mut x = 13.;
        for action in [
            ScrollingAction::Undo,
            ScrollingAction::Preview,
            ScrollingAction::Copy,
            ScrollingAction::Save,
            ScrollingAction::Pin,
            ScrollingAction::Complete,
            ScrollingAction::Cancel,
        ] {
            let width = match action {
                ScrollingAction::Preview if self.snapshot.preview_expanded => 60.,
                ScrollingAction::Save => 49.5,
                ScrollingAction::Complete => 47.,
                _ => 39.,
            };
            let rect = Rect {
                x,
                y: 39.75,
                width,
                height: 32.,
            };
            let enabled = !self.snapshot.working
                && match action {
                    ScrollingAction::Undo => self.snapshot.can_undo,
                    ScrollingAction::Preview => self.snapshot.preview_available,
                    _ => true,
                };
            let label = if action == ScrollingAction::Preview && self.snapshot.preview_expanded {
                "隐藏预览"
            } else {
                action.label()
            };
            let primary = action == ScrollingAction::Complete;
            let hovered = enabled && self.hover == Some(action);
            let opacity = |mut color: crate::Color| {
                if !enabled {
                    color[3] = 102;
                }
                color
            };
            let fill = match (primary, hovered) {
                (true, true) => p.accent_hover,
                (true, false) => p.accent,
                (false, true) => p.raised,
                (false, false) => p.surface,
            };
            scene.commands.push(DrawCommand::FillRect {
                rect,
                radius: 4.,
                color: opacity(fill),
            });
            scene.commands.push(DrawCommand::StrokeRect {
                rect,
                radius: 4.,
                width: 1.,
                color: opacity(if primary { p.accent } else { p.line }),
            });
            let mut label_command = text(
                Rect {
                    x: x + 3.,
                    y: 48.75,
                    width: width - 6.,
                    height: 18.,
                },
                label,
                10.5,
                if !enabled {
                    p.faint
                } else if primary {
                    p.accent_on
                } else {
                    p.ink
                },
            );
            if let DrawCommand::Text { weight, align, .. } = &mut label_command {
                *weight = if primary { 600 } else { 500 };
                *align = TextAlign::Center;
            }
            scene.commands.push(label_command);
            if self.focus == Some(action) {
                scene.commands.push(DrawCommand::StrokeRect {
                    rect,
                    radius: 4.,
                    width: 1.,
                    color: p.accent,
                });
            }
            controls.push(ScrollingControl {
                action,
                rect,
                label: label.into(),
                enabled,
            });
            x += width + 6.;
        }
        scene.commands.push(text(
            status,
            self.status_text(),
            10.5,
            if self.snapshot.error.is_some() {
                p.danger
            } else {
                p.muted
            },
        ));
        if self.snapshot.error.is_some() {
            let rect = retry;
            scene.commands.push(text(rect, "重试", 10.5, p.danger));
            scene.commands.push(DrawCommand::Line {
                from: Point {
                    x: retry.x + 12.5,
                    y: retry.y + 14.,
                },
                to: Point {
                    x: retry.x + 33.5,
                    y: retry.y + 14.,
                },
                width: 0.5,
                color: p.danger,
            });
            controls.push(ScrollingControl {
                action: ScrollingAction::Resume,
                rect,
                label: "重试".into(),
                enabled: !self.snapshot.working,
            });
        }
        self.controls = controls
            .into_iter()
            .map(|mut c| {
                c.rect = Rect {
                    x: c.rect.x * scale,
                    y: c.rect.y * scale,
                    width: c.rect.width * scale,
                    height: c.rect.height * scale,
                };
                c
            })
            .collect();
        let scaled = |rect: Rect| Rect {
            x: rect.x * scale,
            y: rect.y * scale,
            width: rect.width * scale,
            height: rect.height * scale,
        };
        ScrollingFrame {
            scene: scene.scaled(scale),
            controls: self.controls.clone(),
            title: scaled(title),
            dimensions: scaled(dimensions),
            status: scaled(status),
            retry: scaled(retry),
        }
    }
}
fn text(rect: Rect, text: impl Into<String>, size: f32, color: crate::Color) -> DrawCommand {
    DrawCommand::Text {
        rect,
        text: text.into(),
        size,
        weight: 400,
        color,
        font: FontFamily::Sans,
        align: TextAlign::Left,
    }
}

pub const VIEWPORT_BORDER: f32 = 2.;
pub const VIEWPORT_COLOR: crate::Color = [59, 130, 245, 255];

pub fn preview_rect(size: Size, image: Size) -> Rect {
    let scale = (size.width / image.width)
        .min(size.height / image.height)
        .min(1.);
    let width = image.width * scale;
    let height = image.height * scale;
    Rect {
        x: (size.width - width) / 2.,
        y: size.height - height,
        width,
        height,
    }
}

pub fn preview_scene(size: Size, image: Option<RasterImage>) -> Scene {
    let commands = image
        .into_iter()
        .map(|image| DrawCommand::Image {
            rect: preview_rect(
                size,
                Size {
                    width: image.width() as f32,
                    height: image.height() as f32,
                },
            ),
            image,
        })
        .collect();
    overlay_scene(size, commands)
}

pub fn outline_scene(size: Size, scale: f32) -> Scene {
    let stroke = VIEWPORT_BORDER * scale;
    overlay_scene(
        size,
        vec![DrawCommand::StrokeRect {
            rect: Rect {
                x: stroke / 2.,
                y: stroke / 2.,
                width: (size.width - stroke).max(0.),
                height: (size.height - stroke).max(0.),
            },
            radius: 0.,
            width: stroke,
            color: VIEWPORT_COLOR,
        }],
    )
}

fn overlay_scene(size: Size, commands: Vec<DrawCommand>) -> Scene {
    Scene { size, commands }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scaled_controls_reject_busy_or_unavailable_actions_and_expose_retry() {
        let mut ui = ScrollingUi::new(ScrollingSnapshot {
            appearance: Appearance::default(),
            width: 640,
            height: 12000,
            can_undo: false,
            preview_available: false,
            preview_expanded: false,
            working: false,
            hint: Some("已达到最大尺寸".into()),
            error: None,
        });
        let frame = ui.frame(1.75);
        assert_eq!(frame.scene.size.width, 714.);
        assert!(ui.activate(ScrollingAction::Undo).is_none());
        assert!(ui.activate(ScrollingAction::Preview).is_none());
        assert_eq!(
            ui.key(Key::Enter, Modifiers::default()),
            Some(ScrollingAction::Complete)
        );
        ui.snapshot.working = true;
        ui.frame(1.);
        assert!(ui.key(Key::Escape, Modifiers::default()).is_none());
        ui.snapshot.working = false;
        ui.snapshot.error = Some("采样失败".into());
        let frame = ui.frame(1.);
        assert!(frame
            .controls
            .iter()
            .any(|c| c.action == ScrollingAction::Resume && c.enabled));
        assert_eq!(ui.status_text(), "采样失败");
        assert_eq!(
            frame.retry,
            Rect {
                x: 349.,
                y: 79.75,
                width: 46.,
                height: 16.
            }
        );
        ui.snapshot.preview_expanded = true;
        ui.snapshot.preview_available = true;
        let frame = ui.frame(1.25);
        let preview = &frame.controls[1];
        assert_eq!(preview.label, "隐藏预览");
        assert_eq!(
            preview.rect,
            Rect {
                x: 72.5,
                y: 49.6875,
                width: 75.,
                height: 40.
            }
        );
        assert!(ui.hover(Some(Point { x: 80., y: 55. })));
        assert_eq!(ui.hovered(), Some(ScrollingAction::Preview));
        assert_eq!(
            ui.pointer(Point { x: 80., y: 55. }),
            Some(ScrollingAction::Preview)
        );
        assert!(ui.hover(None));
    }

    #[test]
    fn previews_keep_mac_bottom_alignment_without_upscaling_or_a_background() {
        let size = Size {
            width: 110.,
            height: 140.,
        };
        assert_eq!(
            preview_rect(
                size,
                Size {
                    width: 2.,
                    height: 2.
                }
            ),
            Rect {
                x: 54.,
                y: 138.,
                width: 2.,
                height: 2.
            }
        );
        assert_eq!(
            preview_rect(
                size,
                Size {
                    width: 220.,
                    height: 560.
                }
            ),
            Rect {
                x: 27.5,
                y: 0.,
                width: 55.,
                height: 140.
            }
        );
        let image = RasterImage::from_rgba(2, 2, vec![255; 16]).unwrap();
        let preview = preview_scene(size, Some(image));
        assert!(
            matches!(preview.commands.as_slice(),[DrawCommand::Image{rect,..}] if *rect == Rect {x: 54.,y: 138.,width: 2.,height: 2.})
        );
        assert!(preview_scene(size, None).commands.is_empty());
        let outline = outline_scene(size, 1.25);
        assert!(
            matches!(outline.commands.as_slice(),[DrawCommand::StrokeRect{width,color,..}] if *width == 2.5 && *color == VIEWPORT_COLOR)
        );
    }
}
