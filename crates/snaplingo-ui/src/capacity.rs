//! History/favorite limits are edited as one draft and saved explicitly.
use crate::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapacityValue {
    pub automatic_cleanup: bool,
    pub retention_days: u32,
    pub history_limit: u32,
    pub favorites_limit: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapacityTarget {
    Close,
    Cleanup,
    Days,
    History,
    Favorites,
    Cancel,
    Save,
}
impl CapacityTarget {
    pub fn label(self) -> &'static str {
        match self {
            Self::Close => "关闭",
            Self::Cleanup => "自动清理历史记录",
            Self::Days => "历史保留天数",
            Self::History => "历史记录上限",
            Self::Favorites => "收藏容量",
            Self::Cancel => "取消",
            Self::Save => "保存设置",
        }
    }
    pub fn range(self) -> Option<(u32, u32, u32)> {
        match self {
            Self::Days => Some((1, 365, 1)),
            Self::History | Self::Favorites => Some((100, 100000, 100)),
            _ => None,
        }
    }
}
pub enum CapacityInput {
    Focus(CapacityTarget),
    Pointer(Point),
    Activate(CapacityTarget),
    SetValue(CapacityTarget, String),
    Key(Key, Modifiers),
    Text(String),
}
pub enum CapacityIntent {
    Cancel,
    Save(CapacityValue),
}
#[derive(Clone, Debug)]
pub struct CapacityControl {
    pub target: CapacityTarget,
    pub rect: Rect,
    pub value: Option<String>,
    pub checked: Option<bool>,
}
pub struct CapacityFrame {
    pub scene: Scene,
    pub controls: Vec<CapacityControl>,
}
pub struct CapacityUi {
    shell: Snapshot,
    draft: CapacityValue,
    focus: Option<CapacityTarget>,
    editor: Option<TextEdit>,
    controls: Vec<CapacityControl>,
    error: Option<String>,
}
impl CapacityUi {
    pub fn new(shell: Snapshot, draft: CapacityValue) -> Self {
        Self {
            shell,
            draft,
            focus: None,
            editor: None,
            controls: vec![],
            error: None,
        }
    }
    pub fn error(&mut self, error: String) {
        self.error = Some(error);
    }
    pub fn focused(&self) -> Option<CapacityTarget> {
        self.focus
    }
    pub fn value(&self) -> &CapacityValue {
        &self.draft
    }
    fn field_value(&self, target: CapacityTarget) -> String {
        match target {
            CapacityTarget::Days => self.draft.retention_days,
            CapacityTarget::History => self.draft.history_limit,
            CapacityTarget::Favorites => self.draft.favorites_limit,
            _ => 0,
        }
        .to_string()
    }
    fn commit(&mut self) -> bool {
        let Some(edit) = self.editor.take() else {
            return true;
        };
        let Some(target) = self.focus else {
            return true;
        };
        let Some((min, max, _)) = target.range() else {
            return true;
        };
        let value = edit
            .text()
            .parse::<u32>()
            .ok()
            .filter(|n| *n >= min && *n <= max);
        if let Some(value) = value {
            match target {
                CapacityTarget::Days => self.draft.retention_days = value,
                CapacityTarget::History => self.draft.history_limit = value,
                CapacityTarget::Favorites => self.draft.favorites_limit = value,
                _ => {}
            }
            self.error = None;
            true
        } else {
            self.editor = Some(edit);
            self.error = Some(format!("{}须为 {min}–{max} 的整数", target.label()));
            false
        }
    }
    fn focus(&mut self, target: CapacityTarget) -> bool {
        if self.focus == Some(target) {
            return true;
        }
        if !self.commit() {
            return false;
        }
        self.focus = Some(target);
        if target.range().is_some() {
            let mut edit = TextEdit::new(self.field_value(target));
            edit.select_all();
            self.editor = Some(edit);
        }
        true
    }
    pub fn update(&mut self, input: CapacityInput) -> Option<CapacityIntent> {
        match input {
            CapacityInput::Focus(target) => {
                self.focus(target);
            }
            CapacityInput::Pointer(point) => {
                let target = self
                    .controls
                    .iter()
                    .rev()
                    .find(|c| c.rect.contains(point))
                    .map(|c| c.target)?;
                return self.update(CapacityInput::Activate(target));
            }
            CapacityInput::Activate(CapacityTarget::Cancel | CapacityTarget::Close) => {
                return Some(CapacityIntent::Cancel)
            }
            CapacityInput::Activate(target) => {
                if !self.focus(target) {
                    return None;
                }
                if target == CapacityTarget::Cleanup {
                    self.draft.automatic_cleanup = !self.draft.automatic_cleanup;
                }
                if target == CapacityTarget::Save {
                    return Some(CapacityIntent::Save(self.draft.clone()));
                }
            }
            CapacityInput::SetValue(target, value) => {
                if self.focus(target) {
                    if let Some(edit) = &mut self.editor {
                        edit.select_all();
                        edit.insert(&value);
                    }
                }
            }
            CapacityInput::Text(text) => {
                if let Some(edit) = &mut self.editor {
                    if text.chars().all(|c| c.is_ascii_digit()) {
                        edit.insert(&text);
                    }
                }
            }
            CapacityInput::Key(Key::Escape, _) => return Some(CapacityIntent::Cancel),
            CapacityInput::Key(Key::Tab, m) => {
                const ORDER: [CapacityTarget; 6] = [
                    CapacityTarget::Cleanup,
                    CapacityTarget::Days,
                    CapacityTarget::History,
                    CapacityTarget::Favorites,
                    CapacityTarget::Cancel,
                    CapacityTarget::Save,
                ];
                let current = self
                    .focus
                    .and_then(|target| ORDER.iter().position(|t| *t == target));
                let next = if m.shift {
                    current.map_or(5, |i| (i + 5) % 6)
                } else {
                    current.map_or(0, |i| (i + 1) % 6)
                };
                self.focus(ORDER[next]);
            }
            CapacityInput::Key(Key::Enter, _) => {
                if self.commit() {
                    return Some(CapacityIntent::Save(self.draft.clone()));
                }
            }
            CapacityInput::Key(key, m) => {
                if self.editor.is_none() {
                    if matches!(key, Key::Space) {
                        if let Some(target) = self.focus {
                            return self.update(CapacityInput::Activate(target));
                        }
                    }
                    return None;
                }
                if let (Some(target), Some(edit)) = (self.focus, self.editor.as_mut()) {
                    let primary = if self.shell.key_style == KeyStyle::Mac {
                        m.meta
                    } else {
                        m.control
                    };
                    match key {
                        Key::Character('a' | 'A') if primary => edit.select_all(),
                        Key::ArrowLeft => {
                            edit.move_caret(Movement::Left, m.shift);
                        }
                        Key::ArrowRight => {
                            edit.move_caret(Movement::Right, m.shift);
                        }
                        Key::Home => {
                            edit.move_caret(Movement::Home, m.shift);
                        }
                        Key::End => {
                            edit.move_caret(Movement::End, m.shift);
                        }
                        Key::Backspace => {
                            edit.delete(true);
                        }
                        Key::Delete => {
                            edit.delete(false);
                        }
                        Key::ArrowUp | Key::ArrowDown => {
                            if let Some((min, max, step)) = target.range() {
                                if let Ok(value) = edit.text().parse::<u32>() {
                                    let next = if key == Key::ArrowUp {
                                        value.saturating_add(step).min(max)
                                    } else {
                                        value.saturating_sub(step).max(min)
                                    };
                                    *edit = TextEdit::new(next.to_string());
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        None
    }
    pub fn frame(&mut self, scale: f32, metrics: &mut impl TextMeasurer) -> CapacityFrame {
        let p = palette(self.shell.theme, self.shell.dark);
        let mut b = Paint::new(
            Size {
                width: 440.,
                height: 510.,
            },
            p,
            None,
            None,
        );
        b.fill(r(0., 0., 440., 510.), 16., p.surface);
        b.text(
            r(20., 16., 360., 25.),
            "历史与收藏",
            16.,
            700,
            p.ink,
            FontFamily::Sans,
        );
        b.text(
            r(20., 45., 380., 18.),
            "设置历史保留策略与全局收藏容量。",
            12.,
            400,
            p.muted,
            FontFamily::Sans,
        );
        b.line(0., 80., 440., 80., p.line);
        let mut controls = vec![];
        let mut control = |target, rect, value, checked| {
            controls.push(CapacityControl {
                target,
                rect,
                value,
                checked,
            })
        };
        b.center(r(388., 22., 32., 28.), "×", 18., p.faint);
        control(CapacityTarget::Close, r(388., 16., 32., 32.), None, None);
        b.text(
            r(20., 105., 280., 22.),
            "自动清理历史记录",
            14.,
            600,
            p.ink,
            FontFamily::Sans,
        );
        b.text(
            r(20., 132., 310., 18.),
            "仅影响历史记录，收藏会独立保留。",
            10.5,
            400,
            p.muted,
            FontFamily::Sans,
        );
        let toggle = r(380., 112., 40., 22.);
        b.fill(
            toggle,
            11.,
            if self.draft.automatic_cleanup {
                p.accent
            } else {
                p.line_strong
            },
        );
        b.fill(
            r(
                toggle.x
                    + if self.draft.automatic_cleanup {
                        20.
                    } else {
                        2.
                    },
                toggle.y + 2.,
                18.,
                18.,
            ),
            9.,
            p.surface,
        );
        control(
            CapacityTarget::Cleanup,
            toggle,
            None,
            Some(self.draft.automatic_cleanup),
        );
        for (index, (target, help)) in [
            (CapacityTarget::Days, "超过天数的历史记录会自动清理"),
            (CapacityTarget::History, "达到上限后自动清理较早的历史记录"),
            (CapacityTarget::Favorites, "翻译、OCR 与截图共享这个容量"),
        ]
        .into_iter()
        .enumerate()
        {
            let y = 177. + index as f32 * 77.;
            b.text(
                r(20., y, 255., 22.),
                target.label(),
                14.,
                600,
                p.ink,
                FontFamily::Sans,
            );
            b.text(
                r(20., y + 27., 265., 18.),
                help,
                10.5,
                400,
                p.muted,
                FontFamily::Sans,
            );
            let rect = r(308., y + 2., 112., 38.);
            b.fill(rect, 7., p.sunken);
            b.stroke(
                rect,
                7.,
                1.,
                if self.focus == Some(target) {
                    p.accent
                } else {
                    p.line
                },
            );
            let text = if self.focus == Some(target) {
                self.editor
                    .as_ref()
                    .map(|edit| edit.display_text())
                    .unwrap_or_else(|| self.field_value(target))
            } else {
                self.field_value(target)
            };
            b.text(
                r(rect.x + 10., rect.y + 10., 90., 22.),
                &text,
                12.5,
                500,
                p.ink,
                FontFamily::Sans,
            );
            if self.focus == Some(target) {
                if let Some(edit) = &self.editor {
                    let measured = metrics.measure(&text, 12.5, 500, FontFamily::Sans);
                    let x = |byte| {
                        measured
                            .caret_stops
                            .iter()
                            .find(|(index, _)| *index == byte)
                            .map(|(_, x)| *x)
                            .unwrap_or(0.)
                    };
                    b.clip(rect);
                    let selection = edit.selection();
                    if !selection.is_empty() {
                        b.fill(
                            r(
                                rect.x + 10. + x(selection.start),
                                rect.y + 8.,
                                (x(selection.end) - x(selection.start)).abs(),
                                22.,
                            ),
                            0.,
                            p.accent_soft,
                        );
                        b.text(
                            r(rect.x + 10., rect.y + 10., 90., 22.),
                            &text,
                            12.5,
                            500,
                            p.ink,
                            FontFamily::Sans,
                        );
                    }
                    let caret = rect.x + 10. + x(edit.caret());
                    b.line(caret, rect.y + 8., caret, rect.y + 28., p.ink);
                    b.unclip();
                }
            }
            control(target, rect, Some(text), None);
        }
        if let Some(error) = &self.error {
            b.text(
                r(20., 409., 400., 23.),
                error,
                11.,
                400,
                p.danger,
                FontFamily::Sans,
            );
        }
        b.line(0., 446., 440., 446., p.line);
        for (target, rect) in [
            (CapacityTarget::Cancel, r(222., 462., 80., 34.)),
            (CapacityTarget::Save, r(314., 462., 106., 34.)),
        ] {
            b.fill(
                rect,
                7.,
                if target == CapacityTarget::Save {
                    p.accent
                } else {
                    p.surface
                },
            );
            b.stroke(rect, 7., 1., p.line);
            b.center(
                r(rect.x, rect.y + 9., rect.width, 18.),
                target.label(),
                12.,
                if target == CapacityTarget::Save {
                    p.accent_on
                } else {
                    p.muted
                },
            );
            control(target, rect, None, None);
        }
        b.scene.content = r(0., 0., 440., 510.);
        b.scene.viewport = b.scene.content;
        self.controls = controls.clone();
        for control in &mut controls {
            control.rect = control.rect.scaled(scale);
        }
        CapacityFrame {
            scene: b.scene.scaled(scale),
            controls,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn drafts_validate_and_cancel_without_changing_the_saved_value() {
        let saved = CapacityValue {
            automatic_cleanup: true,
            retention_days: 30,
            history_limit: 5000,
            favorites_limit: 1000,
        };
        let mut ui = CapacityUi::new(Snapshot::default(), saved.clone());
        struct Metrics;
        impl TextMeasurer for Metrics {
            fn measure(&mut self, _: &str, _: f32, _: u16, _: FontFamily) -> TextMeasurement {
                Default::default()
            }
        }
        ui.frame(1., &mut Metrics);
        ui.update(CapacityInput::SetValue(CapacityTarget::Days, "999".into()));
        assert!(ui
            .update(CapacityInput::Activate(CapacityTarget::Save))
            .is_none());
        ui.update(CapacityInput::SetValue(CapacityTarget::Days, "365".into()));
        let Some(CapacityIntent::Save(value)) =
            ui.update(CapacityInput::Activate(CapacityTarget::Save))
        else {
            panic!()
        };
        assert_eq!(value.retention_days, 365);
        assert_eq!(saved.retention_days, 30);
        let mut ui = CapacityUi::new(Snapshot::default(), saved.clone());
        ui.update(CapacityInput::Activate(CapacityTarget::Cleanup));
        assert!(matches!(
            ui.update(CapacityInput::Key(Key::Escape, Modifiers::default())),
            Some(CapacityIntent::Cancel)
        ));
        assert!(saved.automatic_cleanup);
    }
}
