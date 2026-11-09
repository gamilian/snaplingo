//! The Settings Window's first-use Screen Recording guide.
use crate::{
    palette, DrawCommand, FontFamily, Key, Modifiers, Point, Rect, Scene, Size, Snapshot,
    TextAlign, TextMeasurer,
};
pub const GRANT_EXPLANATION:&str="如果你此前已授权，重装或更新时的应用签名变化可能使旧版本的授权未对当前应用生效。仅凭系统检测无法判断是尚未授权，还是旧授权未生效。若系统中已开启，请先重新检测；仅在 macOS 要求时重启应用，仍不可用再重新授权。";
#[derive(Clone, Debug)]
pub struct PermissionPromptSnapshot {
    pub shell: Snapshot,
    pub needs_installation: bool,
    pub busy: bool,
    pub attempted: bool,
    pub error: Option<String>,
}
impl PermissionPromptSnapshot {
    pub fn title(&self) -> &'static str {
        if self.needs_installation {
            "先安装 SnapLingo"
        } else {
            "开启屏幕录制"
        }
    }
    pub fn description(&self) -> &'static str {
        if self.needs_installation {
            "当前运行的是临时副本。请将应用拖入“应用程序”后打开，再检测是否需要授权。"
        } else {
            "当前运行的 SnapLingo 尚未获得屏幕录制权限，截图、识别和翻译屏幕内容需要此权限。"
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PermissionPromptTarget {
    Later,
    Authorize,
    Help,
}
#[derive(Clone, Debug)]
pub enum PermissionPromptInput {
    Pointer(Point),
    Key(Key, Modifiers),
    Activate(PermissionPromptTarget),
    Focus(PermissionPromptTarget),
    Snapshot(PermissionPromptSnapshot),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PermissionPromptIntent {
    Dismiss,
    Authorize,
    OpenSettings,
}
#[derive(Clone, Debug)]
pub struct PermissionPromptControl {
    pub target: PermissionPromptTarget,
    pub rect: Rect,
    pub label: String,
    pub enabled: bool,
}
pub struct PermissionPromptFrame {
    pub scene: Scene,
    pub controls: Vec<PermissionPromptControl>,
}
pub struct PermissionPromptUi {
    snapshot: PermissionPromptSnapshot,
    focused: Option<PermissionPromptTarget>,
    controls: Vec<PermissionPromptControl>,
}
impl PermissionPromptUi {
    pub fn new(snapshot: PermissionPromptSnapshot) -> Self {
        Self {
            snapshot,
            focused: None,
            controls: vec![],
        }
    }
    pub fn snapshot(&self) -> &PermissionPromptSnapshot {
        &self.snapshot
    }
    pub fn focused(&self) -> Option<PermissionPromptTarget> {
        self.focused
    }
    fn available(&self, target: PermissionPromptTarget) -> bool {
        match target {
            PermissionPromptTarget::Later => true,
            PermissionPromptTarget::Authorize => {
                !self.snapshot.needs_installation && !self.snapshot.busy
            }
            PermissionPromptTarget::Help => {
                !self.snapshot.needs_installation
                    && (self.snapshot.attempted || self.snapshot.error.is_some())
            }
        }
    }
    pub fn update(&mut self, input: PermissionPromptInput) -> Option<PermissionPromptIntent> {
        let target = match input {
            PermissionPromptInput::Snapshot(snapshot) => {
                self.snapshot = snapshot;
                if self.focused.is_some_and(|target| !self.available(target)) {
                    self.focused = None;
                }
                return None;
            }
            PermissionPromptInput::Focus(target) => {
                if self.available(target) {
                    self.focused = Some(target);
                }
                return None;
            }
            PermissionPromptInput::Pointer(point) => self
                .controls
                .iter()
                .find(|c| c.enabled && c.rect.contains(point))
                .map(|c| c.target)?,
            PermissionPromptInput::Activate(target) => target,
            PermissionPromptInput::Key(Key::Escape, _) => {
                return Some(PermissionPromptIntent::Dismiss)
            }
            PermissionPromptInput::Key(Key::Tab, modifiers) => {
                let targets = [
                    PermissionPromptTarget::Later,
                    PermissionPromptTarget::Authorize,
                    PermissionPromptTarget::Help,
                ]
                .into_iter()
                .filter(|t| self.available(*t))
                .collect::<Vec<_>>();
                let at = self
                    .focused
                    .and_then(|target| targets.iter().position(|t| *t == target));
                let next = at
                    .map(|i| {
                        if modifiers.shift {
                            (i + targets.len() - 1) % targets.len()
                        } else {
                            (i + 1) % targets.len()
                        }
                    })
                    .unwrap_or(if modifiers.shift {
                        targets.len() - 1
                    } else {
                        0
                    });
                self.focused = Some(targets[next]);
                return None;
            }
            PermissionPromptInput::Key(Key::Enter | Key::Space, _) => self.focused?,
            _ => return None,
        };
        if !self.available(target) {
            return None;
        }
        self.focused = Some(target);
        Some(match target {
            PermissionPromptTarget::Later => PermissionPromptIntent::Dismiss,
            PermissionPromptTarget::Authorize => PermissionPromptIntent::Authorize,
            PermissionPromptTarget::Help => PermissionPromptIntent::OpenSettings,
        })
    }
    pub fn frame(&mut self, scale: f32, metrics: &mut impl TextMeasurer) -> PermissionPromptFrame {
        let s = &self.snapshot;
        let p = palette(s.shell.theme, s.shell.dark);
        let scale = scale.clamp(0.5, 3.);
        let width = 380.;
        let inner = 330.;
        let mut commands = vec![];
        let mut controls = vec![];
        let rect = |x: f32, y: f32, width: f32, height: f32| Rect {
            x,
            y,
            width,
            height,
        };
        let title_font = if s.shell.theme == crate::Theme::Paper {
            FontFamily::Serif
        } else {
            FontFamily::Sans
        };
        let mut text = |area: Rect, value: &str, size: f32, weight: u16, color, align| {
            commands.push(DrawCommand::Text {
                rect: area,
                text: value.into(),
                size,
                weight,
                color,
                font: title_font,
                align,
            });
        };
        let icon = rect(166., 25., 48., 48.);
        text(
            rect(25., 89., inner, 25.),
            s.title(),
            17.,
            if s.shell.theme == crate::Theme::Paper {
                400
            } else {
                600
            },
            p.ink,
            TextAlign::Center,
        );
        let mut y = 122.;
        let layout = metrics.line_layout(s.description(), inner, 13., 400, FontFamily::Sans, 20.);
        let height = layout.height;
        let mut start = 0;
        let mut line_y = 0.;
        for (byte, point) in &layout.caret_stops {
            if point.y > line_y && *byte > start {
                commands.push(DrawCommand::Text {
                    rect: rect(25., y + line_y, inner, 20.),
                    text: s.description()[start..*byte].trim().into(),
                    size: 13.,
                    weight: 400,
                    color: p.muted,
                    font: FontFamily::Sans,
                    align: TextAlign::Center,
                });
                start = *byte;
                line_y = point.y;
            }
        }
        commands.push(DrawCommand::Text {
            rect: rect(25., y + line_y, inner, 20.),
            text: s.description()[start..].trim().into(),
            size: 13.,
            weight: 400,
            color: p.muted,
            font: FontFamily::Sans,
            align: TextAlign::Center,
        });
        y += height + 24.;
        if !s.needs_installation {
            let height = metrics
                .line_layout(GRANT_EXPLANATION, inner, 10.5, 400, FontFamily::Sans, 17.)
                .height;
            commands.push(DrawCommand::TextBlock {
                rect: rect(25., y, inner, height),
                text: GRANT_EXPLANATION.into(),
                size: 10.5,
                weight: 400,
                color: p.muted,
                font: FontFamily::Sans,
                line_height: 17.,
            });
            y += height + 20.;
        }
        if s.error.is_some() {
            commands.push(DrawCommand::TextBlock {
                rect: rect(25., y, inner, 15.),
                text: "暂时无法完成操作，请重试。".into(),
                size: 10.5,
                weight: 400,
                color: p.danger,
                font: FontFamily::Sans,
                line_height: 15.,
            });
            y += 31.;
        }
        let height = 39.;
        let later = if s.needs_installation {
            inner
        } else {
            (inner - 8.) / 2.
        };
        for (target, area, label, enabled, fill, color) in [
            (
                PermissionPromptTarget::Later,
                rect(
                    25.,
                    y,
                    if s.needs_installation {
                        later
                    } else {
                        later + 1.
                    },
                    height,
                ),
                if s.needs_installation {
                    "知道了"
                } else {
                    "稍后"
                },
                true,
                p.surface,
                p.ink,
            ),
            (
                PermissionPromptTarget::Authorize,
                rect(25. + later + 9., y, later - 1., height),
                if s.busy {
                    "请稍候…"
                } else if s.error.is_some() {
                    "重试"
                } else {
                    "去授权"
                },
                !s.busy,
                p.accent,
                p.accent_on,
            ),
        ] {
            if target == PermissionPromptTarget::Authorize && s.needs_installation {
                continue;
            }
            commands.push(DrawCommand::FillRect {
                rect: area,
                radius: 8.,
                color: if enabled {
                    fill
                } else {
                    [fill[0], fill[1], fill[2], 128]
                },
            });
            if target == PermissionPromptTarget::Later {
                commands.push(DrawCommand::StrokeRect {
                    rect: area,
                    radius: 8.,
                    width: 1.,
                    color: p.line,
                });
            }
            commands.push(DrawCommand::Text {
                rect: rect(area.x, area.y + 9., area.width, 20.),
                text: label.into(),
                size: 13.,
                weight: 500,
                color: if enabled {
                    color
                } else {
                    [color[0], color[1], color[2], 128]
                },
                font: FontFamily::Sans,
                align: TextAlign::Center,
            });
            controls.push(PermissionPromptControl {
                target,
                rect: area,
                label: label.into(),
                enabled,
            });
        }
        y += height;
        if !s.needs_installation && (s.attempted || s.error.is_some()) {
            let width = metrics
                .measure("授权遇到问题？", 10.5, 400, FontFamily::Sans)
                .caret_stops
                .iter()
                .map(|(_, x)| *x)
                .fold(0., f32::max)
                + 2.;
            let area = rect((380. - width) / 2., y + 16., width, 15.);
            commands.push(DrawCommand::Text {
                rect: area,
                text: "授权遇到问题？".into(),
                size: 10.5,
                weight: 400,
                color: p.muted,
                font: FontFamily::Sans,
                align: TextAlign::Center,
            });
            controls.push(PermissionPromptControl {
                target: PermissionPromptTarget::Help,
                rect: area,
                label: "授权遇到问题？".into(),
                enabled: true,
            });
            y += 33.;
        }
        let height = y + 25.;
        let full = rect(0., 0., width, height);
        commands.insert(
            0,
            DrawCommand::FillRect {
                rect: full,
                radius: 16.,
                color: p.surface,
            },
        );
        commands.insert(
            1,
            DrawCommand::StrokeRect {
                rect: rect(0.5, 0.5, width - 1., height - 1.),
                radius: 15.5,
                width: 1.,
                color: p.line,
            },
        );
        commands.insert(
            1,
            DrawCommand::FillRect {
                rect: icon,
                radius: 12.,
                color: p.accent_soft,
            },
        );
        commands.insert(
            2,
            DrawCommand::StrokeRect {
                rect: rect(179., 43., 22., 15.),
                radius: 2.,
                width: 2.,
                color: p.accent,
            },
        );
        commands.push(DrawCommand::StrokeRect {
            rect: rect(186., 46., 8., 8.),
            radius: 4.,
            width: 2.,
            color: p.accent,
        });
        for (from, to) in [
            (Point { x: 185., y: 43. }, Point { x: 187., y: 40. }),
            (Point { x: 187., y: 40. }, Point { x: 193., y: 40. }),
            (Point { x: 193., y: 40. }, Point { x: 195., y: 43. }),
        ] {
            commands.push(DrawCommand::Line {
                from,
                to,
                width: 2.,
                color: p.accent,
            });
        }
        for control in &controls {
            if self.focused == Some(control.target) {
                commands.push(DrawCommand::StrokeRect {
                    rect: Rect {
                        x: control.rect.x - 2.,
                        y: control.rect.y - 2.,
                        width: control.rect.width + 4.,
                        height: control.rect.height + 4.,
                    },
                    radius: 10.,
                    width: 1.5,
                    color: p.accent,
                });
            }
        }
        self.controls = controls.clone();
        let scene = Scene {
            size: Size { width, height },
            commands,
            hits: vec![],
            ime_cursor: None,
            content: full,
            viewport: full,
            geometry: vec![],
            content_height: height,
        }
        .scaled(scale);
        for c in &mut controls {
            c.rect = Rect {
                x: c.rect.x * scale,
                y: c.rect.y * scale,
                width: c.rect.width * scale,
                height: c.rect.height * scale,
            };
        }
        self.controls = controls.clone();
        PermissionPromptFrame { scene, controls }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Metrics;
    impl TextMeasurer for Metrics {
        fn measure(
            &mut self,
            text: &str,
            size: f32,
            _: u16,
            _: FontFamily,
        ) -> crate::TextMeasurement {
            crate::TextMeasurement {
                caret_stops: (0..=text.len())
                    .filter(|i| text.is_char_boundary(*i))
                    .map(|i| (i, i as f32 * size / 2.))
                    .collect(),
            }
        }
    }
    fn model() -> PermissionPromptUi {
        PermissionPromptUi::new(PermissionPromptSnapshot {
            shell: Snapshot::default(),
            needs_installation: false,
            busy: false,
            attempted: false,
            error: None,
        })
    }
    #[test]
    fn guide_traps_tab_and_preserves_later_escape_while_authorizing() {
        let mut ui = model();
        ui.frame(1., &mut Metrics);
        assert!(ui
            .update(PermissionPromptInput::Key(Key::Enter, Modifiers::default()))
            .is_none());
        ui.update(PermissionPromptInput::Key(Key::Tab, Modifiers::default()));
        assert_eq!(ui.focused(), Some(PermissionPromptTarget::Later));
        ui.update(PermissionPromptInput::Key(
            Key::Tab,
            Modifiers {
                shift: true,
                ..Default::default()
            },
        ));
        assert_eq!(ui.focused(), Some(PermissionPromptTarget::Authorize));
        assert_eq!(
            ui.update(PermissionPromptInput::Key(Key::Enter, Modifiers::default())),
            Some(PermissionPromptIntent::Authorize)
        );
        let mut s = ui.snapshot().clone();
        s.busy = true;
        ui.update(PermissionPromptInput::Snapshot(s));
        assert!(ui
            .update(PermissionPromptInput::Activate(
                PermissionPromptTarget::Authorize
            ))
            .is_none());
        assert_eq!(
            ui.update(PermissionPromptInput::Key(
                Key::Escape,
                Modifiers::default()
            )),
            Some(PermissionPromptIntent::Dismiss)
        );
    }
    #[test]
    fn temporary_copy_only_offers_acknowledgement_and_help_appears_after_attempt() {
        let mut ui = model();
        let mut s = ui.snapshot().clone();
        s.needs_installation = true;
        ui.update(PermissionPromptInput::Snapshot(s));
        let frame = ui.frame(1., &mut Metrics);
        assert_eq!(frame.controls.len(), 1);
        assert_eq!(frame.controls[0].label, "知道了");
        assert!(ui
            .update(PermissionPromptInput::Activate(
                PermissionPromptTarget::Authorize
            ))
            .is_none());
        assert!(ui
            .update(PermissionPromptInput::Activate(
                PermissionPromptTarget::Help
            ))
            .is_none());
        let mut s = ui.snapshot().clone();
        s.needs_installation = false;
        s.attempted = true;
        ui.update(PermissionPromptInput::Snapshot(s));
        assert_eq!(
            ui.update(PermissionPromptInput::Activate(
                PermissionPromptTarget::Help
            )),
            Some(PermissionPromptIntent::OpenSettings)
        );
    }
}
