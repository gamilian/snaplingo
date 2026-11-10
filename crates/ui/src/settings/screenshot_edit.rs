use super::r;
use super::*;
use crate::settings::field::Field;
#[derive(Clone)]
pub(super) struct Editing {
    pub field: Field,
    pub buffer: TextEdit,
    pub scroll: f32,
    pub measured: Option<TextMeasurement>,
}
impl Editing {
    pub fn owns_target(&self, target: Target) -> bool {
        self.field.target() == target
            || (self.field == Field::Screenshot(ScreenshotField::LongshotMaxHeight)
                && matches!(target, Target::ScreenshotStep(_)))
    }
    pub fn new(field: Field, value: Value) -> Self {
        Self {
            field,
            buffer: TextEdit::new(match value {
                Value::Text(s) => s,
                Value::Number(n) => n.to_string(),
                _ => String::new(),
            }),
            scroll: 0.,
            measured: None,
        }
    }
    pub fn value(&self) -> Option<Value> {
        if self.field == Field::Screenshot(ScreenshotField::LongshotMaxHeight) {
            self.buffer
                .text()
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|n| n.is_finite() && *n >= 100.)
                .map(|n| Value::Number(n.round().clamp(100., 100_000.) as u32))
        } else if let Some((min, max, _, _)) = self.field.range() {
            self.buffer
                .text()
                .trim()
                .parse::<u32>()
                .ok()
                .filter(|n| (*n >= min) && (*n <= max))
                .map(Value::Number)
        } else {
            Some(Value::Text(self.buffer.text().into()))
        }
    }
}
impl SettingsUi {
    pub(super) fn begin_text_edit(&mut self, field: ScreenshotField) {
        self.begin_field_edit(Field::Screenshot(field));
    }
    pub(super) fn begin_field_edit(&mut self, field: Field) {
        if self.editing.as_ref().is_some_and(|e| e.field == field) {
            return;
        }
        self.editing = Some(Editing::new(field, field.value(&self.snapshot)));
    }
    pub(super) fn finish_text_edit(&mut self, actions: &mut Vec<Action>) {
        if let Some(mut edit) = self.editing.take() {
            edit.buffer.clear_preedit();
            if let Some(value) = edit.value() {
                if let Some(action) = edit.field.edit(&mut self.snapshot, value) {
                    actions.push(action);
                }
            }
        }
    }
    pub(super) fn text_changed(&mut self, actions: &mut Vec<Action>) {
        if let Some(edit) = self.editing.as_ref().filter(|e| {
            e.field == Field::Screenshot(ScreenshotField::CustomFileName)
                || matches!(e.field,Field::General(field)if field.range().is_some())
        }) {
            if let Some(value) = edit.value() {
                if let Some(action) = edit.field.edit(&mut self.snapshot, value) {
                    actions.push(action);
                }
            }
        }
    }
    pub(super) fn text_key(&mut self, key: Key, m: Modifiers, actions: &mut Vec<Action>) -> bool {
        let Some(edit) = self.editing.as_mut() else {
            return false;
        };
        if edit.buffer.is_composing() {
            return true;
        }
        if edit.field == Field::Screenshot(ScreenshotField::LongshotMaxHeight)
            && matches!(key, Key::ArrowUp | Key::ArrowDown)
        {
            self.step_longshot_height(key == Key::ArrowUp);
            return true;
        }
        if matches!(edit.field, Field::General(_)) && matches!(key, Key::ArrowUp | Key::ArrowDown) {
            if let (Some((min, max, step, _)), Some(Value::Number(value))) =
                (edit.field.range(), edit.value())
            {
                let value = if key == Key::ArrowUp {
                    value.saturating_add(step).min(max)
                } else {
                    value.saturating_sub(step).max(min)
                };
                edit.buffer = TextEdit::new(value.to_string());
                self.text_changed(actions);
            }
            return true;
        }
        let primary = if self.snapshot.key_style == KeyStyle::Mac {
            m.meta
        } else {
            m.control
        };
        let word_modifier = if self.snapshot.key_style == KeyStyle::Mac {
            m.alt
        } else {
            m.control
        };
        if word_modifier && matches!(key, Key::ArrowLeft | Key::ArrowRight) {
            edit.buffer.move_caret(
                if key == Key::ArrowLeft {
                    Movement::WordLeft
                } else {
                    Movement::WordRight
                },
                m.shift,
            );
            return true;
        }
        if primary {
            match key {
                Key::Character('a' | 'A') => edit.buffer.select_all(),
                Key::Character('c' | 'C') => {
                    actions.push(Action::CopyToClipboard(edit.buffer.selected_text().into()))
                }
                Key::Character('x' | 'X') => {
                    actions.push(Action::CopyToClipboard(edit.buffer.selected_text().into()));
                    if !edit.buffer.selection().is_empty() {
                        edit.buffer.delete(true);
                    }
                }
                Key::Character('v' | 'V') => actions.push(Action::ReadClipboard),
                _ => {}
            };
        } else {
            match key {
                Key::ArrowLeft => {
                    edit.buffer.move_caret(Movement::Left, m.shift);
                }
                Key::ArrowRight => {
                    edit.buffer.move_caret(Movement::Right, m.shift);
                }
                Key::Home => {
                    edit.buffer.move_caret(Movement::Home, m.shift);
                }
                Key::End => {
                    edit.buffer.move_caret(Movement::End, m.shift);
                }
                Key::Backspace => {
                    edit.buffer.delete(true);
                }
                Key::Delete => {
                    edit.buffer.delete(false);
                }
                Key::Enter => {
                    self.finish_text_edit(actions);
                    return true;
                }
                Key::Escape => {
                    self.editing = None;
                    return true;
                }
                Key::Tab => {
                    self.finish_text_edit(actions);
                    return false;
                }
                _ => {}
            }
        }
        self.text_changed(actions);
        true
    }
    pub(super) fn text_pointer(&mut self, point: Point) {
        let Some(edit) = self.editing.as_mut() else {
            return;
        };
        let target = edit.field.target();
        let Some(node) = self
            .frame
            .geometry
            .iter()
            .find(|n| n.kind == GeometryKind::Control(target))
        else {
            return;
        };
        if let Some(measured) = edit.measured.as_ref() {
            let x = point.x - node.rect.x - 12. + edit.scroll;
            if let Some((byte, _)) = measured
                .caret_stops
                .iter()
                .min_by(|(_, a), (_, b)| (a - x).abs().total_cmp(&(b - x).abs()))
            {
                edit.buffer.set_selection(*byte..*byte);
            }
        }
    }
    pub(super) fn paint_edit(&self, b: &mut super::Paint, field: ScreenshotField, rect: Rect) {
        self.paint_field_edit(b, Field::Screenshot(field), rect);
        if field == ScreenshotField::LongshotMaxHeight {
            let p = b.p;
            let x = rect.x + rect.width - 21.;
            b.fill(r(x - 2., rect.y + 8., 14., 20.), 3., p.surface);
            b.stroke(r(x - 2., rect.y + 8., 14., 20.), 3., 1., p.line);
            for (increase, y) in [(true, rect.y + 8.), (false, rect.y + 18.)] {
                let center_y = y + 5.;
                let dy = if increase { -2. } else { 2. };
                b.line(x + 1., center_y, x + 4., center_y + dy, p.muted);
                b.line(x + 4., center_y + dy, x + 7., center_y, p.muted);
                b.hit(
                    r(x - 2., y, 14., 10.),
                    Target::ScreenshotStep(increase),
                    if increase {
                        "增加长截图最大高度"
                    } else {
                        "减少长截图最大高度"
                    },
                );
            }
        }
    }

    pub(super) fn step_longshot_height(&mut self, increase: bool) {
        self.begin_text_edit(ScreenshotField::LongshotMaxHeight);
        let edit = self.editing.as_mut().unwrap();
        if edit.buffer.is_composing() {
            return;
        }
        // Preserve the retained WebKit number field's step behavior. This is
        // an input draft; its blur handler rounds and clamps before saving.
        let value = edit
            .buffer
            .text()
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .unwrap_or(0.);
        let next = if increase && value < 100_000. {
            (value + 1_000.).min(100_000.)
        } else if !increase && value > 100. {
            (value - 1_000.).max(100.)
        } else {
            return;
        };
        edit.buffer = TextEdit::new(next.to_string());
    }
    pub(super) fn paint_field_edit(&self, b: &mut super::Paint, field: Field, rect: Rect) {
        let p = b.p;
        let editing = self.editing.as_ref().filter(|edit| edit.field == field);
        let value = field.value(&self.snapshot);
        let display = editing
            .map(|edit| edit.buffer.display_text())
            .unwrap_or_else(|| match value {
                Value::Text(v) => v,
                Value::Number(v) => v.to_string(),
                _ => String::new(),
            });
        b.fill(rect, 8., p.surface);
        b.stroke(
            rect,
            8.,
            1.,
            if editing.is_some() { p.accent } else { p.line },
        );
        b.clip(r(
            rect.x + 1.,
            rect.y + 1.,
            rect.width - 2.,
            rect.height - 2.,
        ));
        let offset = editing.map_or(0., |e| e.scroll);
        let tx = rect.x + 12. - offset;
        let ty = rect.y + (rect.height - 18.) / 2.;
        let caret_x = |edit: &Editing, byte| {
            edit.measured
                .as_ref()
                .and_then(|m| m.caret_stops.iter().find(|(index, _)| *index == byte))
                .map(|(_, x)| *x)
        };
        if let Some(edit) = editing {
            if let Some(selection) = edit.buffer.display_selection().filter(|r| !r.is_empty()) {
                if let (Some(start), Some(end)) =
                    (caret_x(edit, selection.start), caret_x(edit, selection.end))
                {
                    b.fill(
                        r(tx + start.min(end), ty, (end - start).abs(), 19.),
                        0.,
                        p.accent_soft,
                    );
                }
            }
        }
        b.text(
            r(tx, ty, 100_000., 20.),
            if display.is_empty() && field == Field::Screenshot(ScreenshotField::SavePath) {
                "~/Pictures/SnapLingo"
            } else {
                &display
            },
            field.text_size(),
            400,
            if display.is_empty() { p.faint } else { p.ink },
            FontFamily::Sans,
        );
        if let Some(edit) = editing {
            if let Some(range) = edit.buffer.composition_range() {
                if let (Some(start), Some(end)) =
                    (caret_x(edit, range.start), caret_x(edit, range.end))
                {
                    b.line(tx + start, ty + 19., tx + end, ty + 19., p.accent);
                }
            }
            if let Some(byte) = edit.buffer.display_caret() {
                if let Some(cx) = caret_x(edit, byte) {
                    b.line(tx + cx, ty, tx + cx, ty + 18., p.ink);
                    b.frame.ime_cursor = Some(r(tx + cx, ty, 1., 18.));
                }
            }
        }
        b.unclip();
        b.hit(rect, field.target(), field.label());
    }
}

impl SettingsUi {
    pub(super) fn save_local_shortcut(
        &mut self,
        field: ScreenshotField,
        key: Key,
        m: Modifiers,
        registered: bool,
        actions: &mut Vec<Action>,
    ) {
        let value = if matches!(key, Key::Delete | Key::Backspace) {
            String::new()
        } else {
            let Some(main) =
                crate::foundation::input::chord(key, Modifiers::default(), self.snapshot.key_style)
            else {
                self.snapshot.error = Some("请使用字母、数字或 F1–F24。".into());
                return;
            };
            if matches!(field, ScreenshotField::AnnotationShortcut(_))
                && (m.alt || m.control || m.meta)
            {
                self.snapshot.error = Some("工具切换仅使用主键，请松开修饰键。".into());
                return;
            }
            let mut parts = vec![];
            if field == ScreenshotField::PinBorderShortcut {
                for (on, name) in [
                    (m.control, "ctrl"),
                    (m.meta, "meta"),
                    (m.alt, "alt"),
                    (m.shift, "shift"),
                ] {
                    if on {
                        parts.push(name);
                    }
                }
            }
            parts.push(&main);
            parts.join("+").to_ascii_lowercase()
        };
        let display = crate::foundation::input::chord(key, m, self.snapshot.key_style);
        let conflict = display.as_ref().is_some_and(|v| {
            self.snapshot
                .hotkeys
                .iter()
                .chain(self.snapshot.translation_hotkeys.iter())
                .chain(self.snapshot.screenshot_hotkeys.iter())
                .any(|h| &h.value == v)
        });
        if registered || conflict {
            self.snapshot.error = Some("与全局快捷键重复，请先修改对应的全局快捷键。".into());
            return;
        }
        if let Some(action) = Field::Screenshot(field).edit(&mut self.snapshot, Value::Text(value))
        {
            self.snapshot.error = None;
            self.cancel_recording(actions);
            actions.push(action);
        } else {
            self.snapshot.error = Some("快捷键重复或用于固定编辑操作，请换一个按键。".into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mac_option_and_windows_control_move_and_select_unicode_words() {
        for key_style in [KeyStyle::Mac, KeyStyle::Windows] {
            let mut snapshot = Snapshot {
                key_style,
                ..Snapshot::default()
            };
            snapshot.screenshot.save_path = "alpha cafe\u{301} שלום".into();
            let mut ui = SettingsUi::new(snapshot);
            ui.update(Event::Activate(Target::Navigate(Page::Screenshot)));
            ui.update(Event::Activate(Target::Screenshot(
                ScreenshotField::SavePath,
            )));
            let mut modifiers = Modifiers {
                alt: key_style == KeyStyle::Mac,
                control: key_style == KeyStyle::Windows,
                shift: true,
                ..Modifiers::default()
            };
            let press = |ui: &mut SettingsUi, key, modifiers| {
                assert!(ui
                    .update(Event::KeyDown {
                        key,
                        modifiers,
                        repeat: false
                    })
                    .is_empty());
            };
            press(&mut ui, Key::ArrowLeft, modifiers);
            assert_eq!(ui.text_edit().unwrap().selected_text(), "שלום");
            press(&mut ui, Key::ArrowLeft, modifiers);
            assert_eq!(ui.text_edit().unwrap().selected_text(), "cafe\u{301} שלום");
            modifiers.shift = false;
            press(&mut ui, Key::Home, Modifiers::default());
            press(&mut ui, Key::ArrowRight, modifiers);
            assert_eq!(ui.text_edit().unwrap().caret(), "alpha".len());
            modifiers.shift = true;
            press(&mut ui, Key::ArrowRight, modifiers);
            assert_eq!(ui.text_edit().unwrap().selected_text(), " cafe\u{301}");
            ui.update(Event::ImePreedit {
                text: "拼音".into(),
                selection: Some(6..6),
            });
            press(&mut ui, Key::ArrowRight, modifiers);
            assert!(ui.text_edit().unwrap().is_composing());
            assert_eq!(ui.text_edit().unwrap().selected_text(), " cafe\u{301}");
        }
    }
}
