//! Compact scrolling-capture controls shared by native desktop hosts.
use crate::{
    theme::palette, DrawCommand, FontFamily, Key, Modifiers, Point, Rect, Scene, Size, Snapshot,
    TextAlign,
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
    pub shell: Snapshot,
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
}
pub struct ScrollingUi {
    snapshot: ScrollingSnapshot,
    controls: Vec<ScrollingControl>,
    focus: Option<ScrollingAction>,
}
impl ScrollingUi {
    pub fn new(snapshot: ScrollingSnapshot) -> Self {
        Self {
            snapshot,
            controls: vec![],
            focus: None,
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
        let p = palette(self.snapshot.shell.theme, self.snapshot.shell.dark);
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
            ime_cursor: None,
            content: full,
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
            hits: vec![],
            geometry: vec![],
            content_height: 132.,
            viewport: full,
        };
        scene.commands.push(text(
            Rect {
                x: 13.,
                y: 12.,
                width: 200.,
                height: 20.,
            },
            "滚动截图",
            12.,
            p.ink,
        ));
        scene.commands.push(text(
            Rect {
                x: 210.,
                y: 12.,
                width: 185.,
                height: 20.,
            },
            format!("{} × {}", self.snapshot.width, self.snapshot.height),
            11.,
            p.muted,
        ));
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
                y: 40.,
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
            scene.commands.push(DrawCommand::FillRect {
                rect,
                radius: 4.,
                color: if action == ScrollingAction::Complete {
                    p.accent
                } else {
                    p.sunken
                },
            });
            scene.commands.push(text(
                Rect {
                    x: x + 3.,
                    y: 49.,
                    width: width - 6.,
                    height: 18.,
                },
                label,
                10.5,
                if !enabled {
                    p.faint
                } else if action == ScrollingAction::Complete {
                    p.accent_on
                } else {
                    p.ink
                },
            ));
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
        let hint = if self.snapshot.working {
            "正在处理…"
        } else {
            self.snapshot
                .error
                .as_deref()
                .or(self.snapshot.hint.as_deref())
                .unwrap_or("手动滚动页面，完成后复制并保存 PNG。")
        };
        scene.commands.push(text(
            Rect {
                x: 13.,
                y: 82.,
                width: if self.snapshot.error.is_some() {
                    328.
                } else {
                    382.
                },
                height: 39.,
            },
            hint,
            10.5,
            if self.snapshot.error.is_some() {
                p.danger
            } else {
                p.muted
            },
        ));
        if self.snapshot.error.is_some() {
            let rect = Rect {
                x: 349.,
                y: 80.,
                width: 46.,
                height: 25.,
            };
            scene.commands.push(text(rect, "重试", 10.5, p.danger));
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
        ScrollingFrame {
            scene: scene.scaled(scale),
            controls: self.controls.clone(),
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
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scaled_controls_reject_busy_or_unavailable_actions_and_expose_retry() {
        let mut ui = ScrollingUi::new(ScrollingSnapshot {
            shell: Snapshot::default(),
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
    }
}
