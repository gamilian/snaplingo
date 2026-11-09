use super::super::super::shared_presentation::local_point;
use super::*;
use winit::event::{Ime, MouseScrollDelta};

impl View {
    pub(super) fn event(&mut self, event: &WindowEvent) -> Result<(), String> {
        match event {
            WindowEvent::RedrawRequested => self.draw()?,
            WindowEvent::CloseRequested => self.command(CaptureCommand::Cancel),
            WindowEvent::Focused(focused) => {
                self.focused = *focused;
                if !focused && !self.suspended && !self.busy {
                    self.dispatch(Action::FocusLost);
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                let m = modifiers.state();
                if m.shift_key()
                    && !self.modifiers.shift
                    && self.magnifier
                    && self.selection.anchor.is_none()
                {
                    self.rgb = !self.rgb;
                }
                self.magnifier = m.alt_key() || self.preferences.screenshot.show_magnifier;
                self.modifiers = Modifiers {
                    control: m.control_key(),
                    shift: m.shift_key(),
                    alt: m.alt_key(),
                    meta: m.super_key(),
                };
                self.refresh();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = local_point(position.x, position.y, self.scale());
                if self.busy {
                    return Ok(());
                }
                if self.text_dragging {
                    self.text_hit(true);
                } else if self.pressed_ui {
                    let intents = self.ui.pointer_move(self.cursor, &self.snapshot);
                    self.intents(intents);
                } else if self.selection.editing() {
                    self.selection
                        .update_edit(self.desktop_point(), self.modifiers.shift);
                    self.sync_selection();
                } else if self.editor.is_some() {
                    let point = self.editor_point();
                    self.editor
                        .as_mut()
                        .unwrap()
                        .pointer_move(point, self.modifiers.shift);
                } else {
                    self.selection
                        .move_to(self.desktop_point(), self.modifiers.shift);
                    self.inspect();
                }
                self.refresh();
            }
            WindowEvent::MouseInput { state, button, .. } if !self.busy => match (state, button) {
                (ElementState::Pressed, MouseButton::Left) => self.pointer_down(),
                (ElementState::Released, MouseButton::Left) => self.pointer_up(),
                (ElementState::Pressed, MouseButton::Right) => {
                    if self.selection.cancel_edit() {
                        self.sync_selection();
                    } else if let Some(editor) = &mut self.editor {
                        if !editor.escape() {
                            self.editor = None;
                            self.preview = None;
                            self.selection.reset();
                        }
                    } else if self.selection.rect.is_some() {
                        self.selection.reset();
                    } else {
                        self.command(CaptureCommand::Cancel);
                    }
                    self.refresh();
                }
                _ => {}
            },
            WindowEvent::KeyboardInput {
                event,
                is_synthetic: false,
                ..
            } if event.state == ElementState::Pressed => self.key(event),
            WindowEvent::Ime(ime) => self.ime(ime),
            WindowEvent::MouseWheel { delta, .. } if !self.busy => {
                let delta = match delta {
                    MouseScrollDelta::LineDelta(_, dy) => -*dy * 28.,
                    MouseScrollDelta::PixelDelta(p) => -p.y as f32 / self.scale() as f32,
                };
                if self.ui.scroll(self.cursor, delta, &self.snapshot) {
                    self.refresh();
                } else if self.editor.is_some()
                    && self.text.is_none()
                    && !self.modifiers.alt
                    && delta != 0.
                {
                    let step = if self.modifiers.control { 3 } else { 1 };
                    self.adjust_size(if delta < 0. { step } else { -step });
                }
            }
            WindowEvent::ScaleFactorChanged {
                inner_size_writer, ..
            } => {
                let scale = self.scale();
                let bounds = &self.geometry.bounds;
                let mut writer = inner_size_writer.clone();
                let _ = writer.request_inner_size(PhysicalSize::new(
                    (bounds.width * scale).round().max(1.) as u32,
                    (bounds.height * scale).round().max(1.) as u32,
                ));
                self.refresh();
            }
            WindowEvent::ThemeChanged(_) | WindowEvent::Resized(_) => self.refresh(),
            _ => {}
        }
        Ok(())
    }
    fn sync_selection(&mut self) {
        if let (Some(editor), Some(rect)) = (&mut self.editor, &self.selection.rect) {
            editor.selection_changed(rect.clone());
            self.preview = None;
        }
    }
    fn pointer_down(&mut self) {
        self.toolbar_focus = None;
        if let Some(intents) = self.ui.pointer_down(self.cursor, &self.snapshot) {
            self.pressed_ui = true;
            self.intents(intents);
            return;
        }
        let point = self.desktop_point();
        let now = std::time::Instant::now();
        let detail = if self.last_click.is_some_and(|(time, p)| {
            now.duration_since(time).as_millis() < 500
                && (p.x - self.cursor.x).abs() < 4.
                && (p.y - self.cursor.y).abs() < 4.
        }) {
            2
        } else {
            1
        };
        self.last_click = Some((now, self.cursor));
        if self
            .text_layout()
            .is_some_and(|(rect, _)| rect.contains(self.cursor))
        {
            self.text_hit(self.modifiers.shift);
            if detail >= 2 {
                let text = self.text.as_mut().unwrap();
                text.move_caret(snaplingo_ui::Movement::WordLeft, false);
                text.move_caret(snaplingo_ui::Movement::WordRight, true);
            }
            self.text_dragging = true;
            self.refresh();
            return;
        }
        if let Some(editor) = &self.editor {
            let local = self.editor_point();
            let annotation_handle = editor
                .selected_handles()
                .iter()
                .any(|(_, h)| (h.x - local.x).hypot(h.y - local.y) <= 6.);
            let edge = !annotation_handle
                && local
                    .x
                    .min(local.y)
                    .min(editor.selection.width - local.x)
                    .min(editor.selection.height - local.y)
                    <= 6.;
            if edge && self.selection.begin_edit(point.clone(), false) {
                self.refresh();
                return;
            }
            self.commit_composition();
            let point = self.editor_point();
            let editor = self.editor.as_mut().unwrap();
            editor.pointer_down(point, detail, self.modifiers.control);
            let moving = editor.tool.is_none() && editor.selected.is_none();
            self.text = editor
                .text
                .as_ref()
                .map(|text| snaplingo_ui::TextEdit::multiline(text.text.clone()));
            self.text_undo.clear();
            self.text_redo.clear();
            if moving {
                self.selection.begin_edit(self.desktop_point(), true);
            }
            self.window.set_ime_allowed(self.text.is_some());
        } else {
            self.selection.press(
                point,
                !self.modifiers.control
                    && !self.modifiers.alt
                    && !self.modifiers.shift
                    && !self.modifiers.meta,
            );
        }
        self.refresh();
    }
    fn pointer_up(&mut self) {
        if self.text_dragging {
            self.text_dragging = false;
            self.text_hit(true);
            return;
        }
        if self.pressed_ui {
            self.pressed_ui = false;
            let intents = self.ui.pointer_up(self.cursor, &self.snapshot);
            self.intents(intents);
            return;
        }
        if self.selection.finish_edit() {
            self.sync_selection();
        } else if self.editor.is_some() {
            let point = self.editor_point();
            self.editor
                .as_mut()
                .unwrap()
                .pointer_up(point, self.modifiers.shift);
        } else if let Some(rect) = self
            .selection
            .release(self.desktop_point(), self.modifiers.shift)
        {
            self.complete_selection(rect);
        }
        self.refresh();
    }
    fn key(&mut self, event: &winit::event::KeyEvent) {
        let named = match &event.logical_key {
            Key::Named(key) => Some(*key),
            _ => None,
        };
        let key = match &event.logical_key {
            Key::Character(text) => text.to_lowercase(),
            _ => String::new(),
        };
        if named == Some(NamedKey::Escape) && self.ui.dismiss_menus() {
            self.refresh();
            return;
        }
        if self.busy {
            return;
        }
        if self.text.is_some() {
            self.text_key(event);
            return;
        }
        if named == Some(NamedKey::Escape) {
            if self.selection.cancel_edit() {
                self.sync_selection();
                self.refresh();
                return;
            }
            self.command(CaptureCommand::Cancel);
            return;
        }
        if self.busy {
            return;
        }
        let primary = self.modifiers.control;
        let shift = self.modifiers.shift;
        if let Some(target) = self.toolbar_focus {
            if matches!(named, Some(NamedKey::Enter | NamedKey::Space)) {
                let intents = self.ui.activate(target, &self.snapshot);
                self.intents(intents);
                return;
            }
        }
        if named == Some(NamedKey::Tab)
            && self.editor.is_some()
            && !self.modifiers.alt
            && !primary
            && (shift
                || self.toolbar_focus.is_some()
                || !matches!(
                    self.editor.as_ref().unwrap().tool,
                    Some(AnnotationTool::Arrow | AnnotationTool::Line)
                ))
        {
            let controls = self
                .frame
                .controls
                .iter()
                .filter(|c| c.enabled)
                .map(|c| c.target)
                .collect::<Vec<_>>();
            if !controls.is_empty() {
                let index = self
                    .toolbar_focus
                    .and_then(|t| controls.iter().position(|c| *c == t));
                let next = index
                    .map(|i| {
                        if shift {
                            (i + controls.len() - 1) % controls.len()
                        } else {
                            (i + 1) % controls.len()
                        }
                    })
                    .unwrap_or(if shift { controls.len() - 1 } else { 0 });
                self.toolbar_focus = Some(controls[next]);
                self.refresh();
            }
            return;
        }
        if named == Some(NamedKey::F5) {
            self.command(CaptureCommand::Refresh);
            return;
        }
        if primary && !self.modifiers.alt {
            let command = match key.as_str() {
                "c" => Some(CaptureCommand::Copy),
                "s" => Some(if shift {
                    CaptureCommand::QuickSave
                } else {
                    CaptureCommand::Save
                }),
                "t" => Some(CaptureCommand::Pin),
                "p" => Some(CaptureCommand::Print),
                "z" => Some(if shift {
                    CaptureCommand::Redo
                } else {
                    CaptureCommand::Undo
                }),
                "y" => Some(CaptureCommand::Redo),
                _ => None,
            };
            if let Some(command) = command {
                self.command(command);
                return;
            }
            if key == "a" && self.editor.is_none() {
                let rect = if shift {
                    self.geometry.bounds.clone()
                } else {
                    let point = self.desktop_point();
                    self.images
                        .iter()
                        .find(|image| image.rect.contains(self.cursor))
                        .map(|image| LogicalRect {
                            x: image.rect.x as f64 + self.geometry.bounds.x,
                            y: image.rect.y as f64 + self.geometry.bounds.y,
                            width: image.rect.width as f64,
                            height: image.rect.height as f64,
                        })
                        .unwrap_or_else(|| {
                            let _ = point;
                            self.geometry.bounds.clone()
                        })
                };
                self.complete_selection(rect);
                return;
            }
        }
        if named == Some(NamedKey::Enter) {
            self.command(if self.mode == CaptureSelectionMode::Edit {
                CaptureCommand::Copy
            } else {
                CaptureCommand::Primary
            });
            return;
        }
        if named == Some(NamedKey::F3) {
            self.command(CaptureCommand::Pin);
            return;
        }
        if !primary && !self.modifiers.alt && (key == "`" || key == "!") {
            self.include_cursor = !self.include_cursor;
            self.rendering.invalidate();
            self.refresh();
            return;
        }
        if let Some(editor) = &mut self.editor {
            if primary && self.modifiers.alt && named == Some(NamedKey::Backspace) {
                self.command(CaptureCommand::Clear);
                return;
            }
            if !primary && !self.modifiers.alt {
                if key == "["
                    || key == "]"
                    || (key == "1" || key == "2")
                        && !self
                            .preferences
                            .screenshot
                            .annotation_tool_shortcuts
                            .values()
                            .any(|s| s == &key)
                {
                    self.adjust_size(if key == "]" || key == "2" { 1 } else { -1 });
                    return;
                }
                if key == "f" && editor.styling.fill_enabled {
                    let filled = !editor.styling.style.filled;
                    self.intents(vec![CaptureIntent::Style {
                        value: CaptureStyleValue::Filled(filled),
                        commit: false,
                    }]);
                    return;
                }
                if named == Some(NamedKey::Tab) {
                    let tool = match editor.tool {
                        Some(AnnotationTool::Arrow) => Some(CaptureTool::Line),
                        Some(AnnotationTool::Line) => Some(CaptureTool::Arrow),
                        _ => None,
                    };
                    if let Some(tool) = tool {
                        self.intents(vec![CaptureIntent::Tool(tool)]);
                        return;
                    }
                }
            }
            if matches!(named, Some(NamedKey::Delete | NamedKey::Backspace)) {
                editor.control(&CaptureEditorControlAction::Command("delete"));
                self.refresh();
                return;
            }
            let step = if shift { 10. } else { 1. };
            let delta = match named {
                Some(NamedKey::ArrowLeft) => Some((-step, 0.)),
                Some(NamedKey::ArrowRight) => Some((step, 0.)),
                Some(NamedKey::ArrowUp) => Some((0., -step)),
                Some(NamedKey::ArrowDown) => Some((0., step)),
                _ => None,
            };
            if let Some((x, y)) = delta {
                if !editor.nudge_selected(LogicalPoint { x, y }) {
                    self.selection.nudge_rect(x, y);
                    self.sync_selection();
                }
                self.refresh();
                return;
            }
            let shortcut_key = match named {
                Some(NamedKey::F1) => "f1",
                Some(NamedKey::F2) => "f2",
                Some(NamedKey::F4) => "f4",
                Some(NamedKey::F6) => "f6",
                Some(NamedKey::F7) => "f7",
                Some(NamedKey::F8) => "f8",
                Some(NamedKey::F9) => "f9",
                Some(NamedKey::F10) => "f10",
                Some(NamedKey::F11) => "f11",
                Some(NamedKey::F12) => "f12",
                _ => &key,
            };
            let shortcut = self
                .preferences
                .screenshot
                .annotation_tool_shortcuts
                .iter()
                .find(|(_, shortcut)| shortcut.eq_ignore_ascii_case(shortcut_key))
                .map(|(tool, _)| tool.clone());
            if !primary && !self.modifiers.alt {
                if let Some(tool) = shortcut {
                    if tool == "scrolling" {
                        self.command(CaptureCommand::Scrolling);
                    } else if AnnotationTool::parse(&tool).is_some() {
                        let editor = self.editor.as_mut().unwrap();
                        let tool = AnnotationTool::parse(&tool).unwrap();
                        let command = if editor.tool == Some(tool) {
                            "select"
                        } else {
                            tool.name()
                        };
                        editor.control(&CaptureEditorControlAction::Command(command));
                        self.dispatch(Action::RememberTool(tool.name().into()));
                        self.refresh();
                    }
                } else if let Ok(index) = key.parse::<usize>() {
                    let color = self
                        .editor
                        .as_ref()
                        .unwrap()
                        .styling
                        .color_presets
                        .get(index.wrapping_sub(1))
                        .copied();
                    if let Some(color) = color {
                        self.intents(vec![CaptureIntent::Style {
                            value: CaptureStyleValue::Color(color),
                            commit: false,
                        }]);
                    }
                }
            }
            return;
        }
        if !primary && !self.modifiers.alt {
            if named == Some(NamedKey::Tab) {
                self.selection.toggle_detection();
                self.inspect();
                self.refresh();
                return;
            }
            if key == "c" && self.magnifier {
                if let Some(color) = &self.snapshot.sampled_color {
                    self.dispatch(Action::CopyText(color.clone()));
                }
                return;
            }
            if key == "r" || key == "," || key == "." {
                let len = self.history.len();
                if len > 0 {
                    let current = self
                        .selection
                        .rect
                        .as_ref()
                        .and_then(|r| self.history.iter().position(|h| h == r));
                    let index = if key == "r" {
                        0
                    } else {
                        current
                            .map(|i| {
                                if key == "," {
                                    (i + 1) % len
                                } else {
                                    (i + len - 1) % len
                                }
                            })
                            .unwrap_or(0)
                    };
                    if let Some(rect) = snaplingo_core::application::capture::restore_selection(
                        &self.history[index],
                        &self.geometry.bounds,
                    ) {
                        self.complete_selection(rect);
                    }
                }
                return;
            }
            let step = if shift { 10. } else { 1. };
            let delta = match (key.as_str(), named) {
                ("a", _) | (_, Some(NamedKey::ArrowLeft)) => Some((-step, 0.)),
                ("d", _) | (_, Some(NamedKey::ArrowRight)) => Some((step, 0.)),
                ("w", _) | (_, Some(NamedKey::ArrowUp)) => Some((0., -step)),
                ("s", _) | (_, Some(NamedKey::ArrowDown)) => Some((0., step)),
                _ => None,
            };
            if let Some((x, y)) = delta {
                if let Some(delta) = self.selection.nudge_cursor(x, y) {
                    self.dispatch(Action::MoveCursor(
                        (delta.x * self.scale()).round() as i32,
                        (delta.y * self.scale()).round() as i32,
                    ));
                }
                self.inspect();
                self.refresh();
            }
        }
    }
    fn adjust_size(&mut self, delta: i32) {
        let Some(editor) = &self.editor else {
            return;
        };
        if matches!(
            editor.tool,
            Some(AnnotationTool::Spotlight | AnnotationTool::RegionHighlight)
        ) {
            return;
        }
        let style = &editor.styling;
        let value = if style.text_mode {
            CaptureStyleValue::FontSize((style.font_size as i32 + delta).clamp(10, 96) as u32)
        } else if style.blur_mode {
            CaptureStyleValue::BlurRadius((style.style.blur_radius + delta as f64).clamp(4., 64.))
        } else {
            CaptureStyleValue::StrokeWidth(
                (style.style.stroke_width as i32 + delta).clamp(1, 32) as u32
            )
        };
        self.intents(vec![CaptureIntent::Style {
            value,
            commit: true,
        }]);
    }
    fn text_key(&mut self, event: &winit::event::KeyEvent) {
        use snaplingo_ui::Movement;
        let key = match &event.logical_key {
            Key::Character(t) => t.to_lowercase(),
            _ => String::new(),
        };
        let named = match event.logical_key {
            Key::Named(key) => Some(key),
            _ => None,
        };
        if self.text.as_ref().unwrap().is_composing() {
            return;
        }
        if named == Some(NamedKey::Escape) {
            self.editor.as_mut().unwrap().escape();
            self.text = None;
            self.window.set_ime_allowed(false);
            self.refresh();
            return;
        }
        let primary = self.modifiers.control;
        let shift = self.modifiers.shift;
        if named == Some(NamedKey::Enter) && !shift && !primary {
            self.editor.as_mut().unwrap().commit_text();
            self.text = None;
            self.window.set_ime_allowed(false);
            self.refresh();
            return;
        }
        if primary && matches!(key.as_str(), "s" | "t" | "p") {
            let command = match key.as_str() {
                "s" if shift => CaptureCommand::QuickSave,
                "s" => CaptureCommand::Save,
                "t" => CaptureCommand::Pin,
                _ => CaptureCommand::Print,
            };
            self.command(command);
            return;
        }
        if primary && matches!(key.as_str(), "z" | "y") {
            let redo = key == "y" || shift;
            let previous = if redo {
                self.text_redo.pop()
            } else {
                self.text_undo.pop()
            };
            if let Some(previous) = previous {
                let current = self.text.replace(previous).unwrap();
                if redo {
                    self.text_undo.push(current);
                } else {
                    self.text_redo.push(current);
                }
                self.sync_text();
            }
            return;
        }
        if matches!(named, Some(NamedKey::ArrowUp | NamedKey::ArrowDown)) {
            self.text_vertical(named == Some(NamedKey::ArrowDown), shift);
            return;
        }
        let before = self.text.as_ref().unwrap().clone();
        let mut copy = None;
        let text = self.text.as_mut().unwrap();
        if primary && !self.modifiers.alt {
            match key.as_str() {
                "a" => text.select_all(),
                "c" => copy = Some(text.selected_text().into()),
                "x" => {
                    copy = Some(text.selected_text().into());
                    text.insert("");
                }
                "v" => {
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        if let Ok(value) = clipboard.get_text() {
                            text.insert(&value);
                        }
                    }
                }
                _ => match named {
                    Some(NamedKey::ArrowLeft) => {
                        text.move_caret(Movement::WordLeft, shift);
                    }
                    Some(NamedKey::ArrowRight) => {
                        text.move_caret(Movement::WordRight, shift);
                    }
                    Some(NamedKey::Home) => {
                        text.move_caret(Movement::Home, shift);
                    }
                    Some(NamedKey::End) => {
                        text.move_caret(Movement::End, shift);
                    }
                    Some(NamedKey::Enter) => {
                        text.insert("\n");
                    }
                    Some(NamedKey::Backspace | NamedKey::Delete) => {
                        let direction = if named == Some(NamedKey::Backspace) {
                            Movement::WordLeft
                        } else {
                            Movement::WordRight
                        };
                        text.move_caret(direction, true);
                        text.insert("");
                    }
                    _ => {}
                },
            }
        } else if !self.modifiers.alt {
            match named {
                Some(NamedKey::Enter) => {
                    text.insert("\n");
                }
                Some(NamedKey::Backspace) => {
                    text.delete(true);
                }
                Some(NamedKey::Delete) => {
                    text.delete(false);
                }
                Some(NamedKey::ArrowLeft) => {
                    text.move_caret(Movement::Left, shift);
                }
                Some(NamedKey::ArrowRight) => {
                    text.move_caret(Movement::Right, shift);
                }
                Some(NamedKey::Home | NamedKey::End) => {
                    let value = text.text();
                    let caret = text.caret();
                    let byte = if named == Some(NamedKey::Home) {
                        value[..caret].rfind('\n').map_or(0, |i| i + 1)
                    } else {
                        value[caret..].find('\n').map_or(value.len(), |i| caret + i)
                    };
                    text.move_to(byte, shift);
                }
                _ => {
                    if let Some(value) = &event.text {
                        text.insert(value);
                    }
                }
            }
        }
        if text.text() != before.text() {
            self.text_undo.push(before);
            self.text_redo.clear();
        }
        if let Some(copy) = copy {
            self.dispatch(Action::CopyText(copy));
        }
        self.sync_text();
    }
    fn ime(&mut self, ime: &Ime) {
        let Some(text) = &mut self.text else {
            return;
        };
        match ime {
            Ime::Preedit(value, selection) => {
                text.preedit(value, selection.map(|(start, end)| start..end));
            }
            Ime::Commit(value) => {
                self.text_undo.push(text.clone());
                self.text_redo.clear();
                text.commit(value);
            }
            Ime::Disabled => {
                text.clear_preedit();
            }
            _ => {}
        }
        self.sync_text();
    }
    pub(super) fn sync_text(&mut self) {
        if let (Some(editor), Some(text)) = (&mut self.editor, &self.text) {
            editor.update_text(text.display_text());
        }
        self.rendering.invalidate();
        self.refresh();
    }
    pub(super) fn commit_composition(&mut self) {
        if let (Some(editor), Some(text)) = (&mut self.editor, &mut self.text) {
            text.clear_preedit();
            editor.update_text(text.text().into());
        }
    }
}
