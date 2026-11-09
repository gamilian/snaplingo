use super::*;
use crate::{Key, Modifiers, Movement, Point, Rect, Size, TextEdit, TextLayout};

pub struct LibraryUi {
    pub(crate) snapshot: LibrarySnapshot,
    pub(crate) query: LibraryQuery,
    pub(crate) revision: u64,
    pub(crate) selected: Option<ItemKey>,
    pub(crate) focus: Option<LibraryTarget>,
    pub(crate) editing: Option<Editor>,
    pub(crate) list_scroll: f32,
    pub(crate) detail_scroll: f32,
    pub(crate) list_limit: f32,
    pub(crate) detail_limit: f32,
    pub(crate) list_view: Rect,
    pub(crate) detail_view: Rect,
    pub(crate) frame: Option<LibraryFrame>,
    pub(crate) scale: f32,
    selecting_text: bool,
    pub(crate) keyboard_focus: bool,
}
pub(crate) struct Editor {
    pub field: LibraryField,
    pub buffer: TextEdit,
    pub rect: Rect,
    pub layout: TextLayout,
    pub scroll: Point,
}
impl LibraryUi {
    pub fn new(snapshot: LibrarySnapshot) -> Self {
        Self {
            query: snapshot.query.clone(),
            revision: snapshot.revision,
            selected: snapshot.items.first().map(|i| i.key),
            snapshot,
            focus: None,
            editing: None,
            list_scroll: 0.,
            detail_scroll: 0.,
            list_limit: 0.,
            detail_limit: 0.,
            list_view: Rect::default(),
            detail_view: Rect::default(),
            frame: None,
            scale: 1.,
            selecting_text: false,
            keyboard_focus: false,
        }
    }
    pub fn snapshot(&self) -> &LibrarySnapshot {
        &self.snapshot
    }
    pub fn query(&self) -> &LibraryQuery {
        &self.query
    }
    pub fn selected(&self) -> Option<ItemKey> {
        self.selected
    }
    pub fn focused(&self) -> Option<LibraryTarget> {
        self.focus
    }
    pub fn text_edit(&self) -> Option<&TextEdit> {
        self.editing.as_ref().map(|e| &e.buffer)
    }
    pub(crate) fn frame_shell_hit(&self, point: Point) -> Option<crate::Target> {
        self.frame
            .as_ref()?
            .scene
            .hit_test(point)
            .filter(|target| matches!(target, crate::Target::Navigate(_) | crate::Target::Capacity))
    }
    pub fn update(&mut self, input: LibraryInput) -> Vec<LibraryIntent> {
        let mut intents = vec![];
        match input {
            LibraryInput::Refresh => self.request(&mut intents),
            LibraryInput::Snapshot(snapshot) => {
                if snapshot.kind != self.snapshot.kind || snapshot.revision != self.revision {
                    return intents;
                }
                self.query = snapshot.query.clone();
                if !snapshot.items.iter().any(|i| Some(i.key) == self.selected) {
                    self.selected = snapshot.items.first().map(|i| i.key);
                    if self
                        .editing
                        .as_ref()
                        .is_some_and(|e| e.field != LibraryField::Search)
                    {
                        self.editing = None;
                        self.focus = None;
                    }
                    self.detail_scroll = 0.;
                }
                self.snapshot = snapshot;
            }
            LibraryInput::Activate(target) => {
                if self.valid_target(target) {
                    self.blur_unless(target, &mut intents);
                    self.focus = Some(target);
                    self.ensure_visible(target);
                    self.activate(target, &mut intents);
                }
            }
            LibraryInput::Focus(target) => {
                self.keyboard_focus = true;
                if self.valid_target(target) {
                    self.blur_unless(target, &mut intents);
                    self.focus = Some(target);
                    self.ensure_visible(target);
                    if let Some(field) = target.field() {
                        self.begin_edit(field);
                    }
                }
            }
            LibraryInput::PointerDown(point) => {
                self.keyboard_focus = false;
                let target = self.frame.as_ref().and_then(|f| f.hit_test(point));
                if let Some(target) = target.filter(|target| self.valid_target(*target)) {
                    self.blur_unless(target, &mut intents);
                    self.focus = Some(target);
                    self.activate(target, &mut intents);
                    self.text_pointer(point, false);
                    self.selecting_text = target.field().is_some();
                } else {
                    self.finish_edit(&mut intents);
                    self.focus = None;
                    self.selecting_text = false;
                }
            }
            LibraryInput::PointerMove(point) => {
                if self.selecting_text {
                    self.text_pointer(point, true);
                }
            }
            LibraryInput::PointerUp => self.selecting_text = false,
            LibraryInput::Scroll { position, delta_y } => {
                if delta_y.is_finite() {
                    let point = Point {
                        x: position.x / self.scale,
                        y: position.y / self.scale,
                    };
                    if self.list_view.contains(point) {
                        self.list_scroll =
                            (self.list_scroll + delta_y / self.scale).clamp(0., self.list_limit);
                    } else if self.detail_view.contains(point) {
                        self.detail_scroll = (self.detail_scroll + delta_y / self.scale)
                            .clamp(0., self.detail_limit);
                    }
                }
            }
            LibraryInput::Text(text) | LibraryInput::ClipboardText(text) => {
                if let Some(edit) = self.editing.as_mut() {
                    edit.buffer.insert(&text);
                    self.text_changed(&mut intents);
                }
            }
            LibraryInput::Selection(range) => {
                if let Some(edit) = self.editing.as_mut() {
                    edit.buffer.set_selection(range);
                }
            }
            LibraryInput::ImePreedit { text, selection } => {
                if let Some(edit) = self.editing.as_mut() {
                    edit.buffer.preedit(&text, selection);
                }
            }
            LibraryInput::ImeCommit(text) => {
                if let Some(edit) = self.editing.as_mut() {
                    edit.buffer.commit(&text);
                    self.text_changed(&mut intents);
                }
            }
            LibraryInput::ImeDisabled => {
                if let Some(edit) = self.editing.as_mut() {
                    edit.buffer.clear_preedit();
                }
            }
            LibraryInput::FocusLost => {
                self.finish_edit(&mut intents);
                self.focus = None;
                self.selecting_text = false;
            }
            LibraryInput::KeyDown { key, modifiers } => {
                self.keyboard_focus = true;
                self.key(key, modifiers, &mut intents)
            }
        }
        intents
    }
    fn request(&mut self, intents: &mut Vec<LibraryIntent>) {
        self.revision = self.revision.wrapping_add(1);
        self.snapshot.loading = true;
        self.snapshot.error = None;
        intents.push(LibraryIntent::Query {
            revision: self.revision,
            kind: self.snapshot.kind,
            query: self.query.clone(),
        });
    }
    fn valid_target(&self, target: LibraryTarget) -> bool {
        let exists = self
            .frame
            .as_ref()
            .is_some_and(|f| f.controls.iter().any(|c| c.target == target && c.enabled));
        if !exists {
            return false;
        }
        match target {
            LibraryTarget::Item(key) => self.item(key).is_some(),
            LibraryTarget::Copy(key, _)
            | LibraryTarget::Delete(key)
            | LibraryTarget::Unfavorite(key)
            | LibraryTarget::RerunOcr(key)
            | LibraryTarget::CopyScreenshot(key)
            | LibraryTarget::RevealScreenshot(key)
            | LibraryTarget::Note(key)
            | LibraryTarget::Tags(key) => self.selected == Some(key) && self.item(key).is_some(),
            LibraryTarget::Favorite(key, result) => {
                self.selected == Some(key)
                    && self.item(key).is_some_and(|item| match &item.content {
                        LibraryContent::Translation { results, .. } => {
                            results.get(result).is_some_and(|r| !r.favorite)
                        }
                        LibraryContent::Ocr { favorite, .. } => !*favorite,
                        _ => false,
                    })
            }
            _ => true,
        }
    }
    fn item(&self, key: ItemKey) -> Option<&LibraryItem> {
        self.snapshot.items.iter().find(|i| i.key == key)
    }
    fn activate(&mut self, target: LibraryTarget, intents: &mut Vec<LibraryIntent>) {
        if let Some(field) = target.field() {
            self.begin_edit(field);
            return;
        }
        match target {
            LibraryTarget::Filter(filter) => {
                if self.query.filter != filter {
                    self.query.filter = filter;
                    self.query.page = 0;
                    self.list_scroll = 0.;
                    self.request(intents);
                }
            }
            LibraryTarget::PreviousPage => {
                self.query.page = self.query.page.saturating_sub(1);
                self.list_scroll = 0.;
                self.request(intents);
            }
            LibraryTarget::NextPage => {
                self.query.page = self.query.page.saturating_add(1);
                self.list_scroll = 0.;
                self.request(intents);
            }
            LibraryTarget::Item(key) => {
                if self.item(key).is_some() && self.selected != Some(key) {
                    self.selected = Some(key);
                    self.detail_scroll = 0.;
                }
            }
            LibraryTarget::ClearHistory => {
                intents.push(LibraryIntent::ConfirmClearHistory(self.query.filter))
            }
            LibraryTarget::Copy(key, part) => {
                let text = self.item(key).and_then(|item| match (&item.content, part) {
                    (LibraryContent::Translation { source, .. }, CopyPart::Source) => {
                        Some(source.clone())
                    }
                    (LibraryContent::Translation { results, .. }, CopyPart::Result(index)) => {
                        results.get(index).map(|r| r.text.clone())
                    }
                    (LibraryContent::Ocr { text, .. }, CopyPart::Ocr) => Some(text.clone()),
                    _ => None,
                });
                if let Some(text) = text {
                    intents.push(LibraryIntent::CopyText(text));
                }
            }
            LibraryTarget::Favorite(key, result) => {
                intents.push(LibraryIntent::Favorite { key, result })
            }
            LibraryTarget::Delete(key) | LibraryTarget::Unfavorite(key) => {
                intents.push(LibraryIntent::Delete(key))
            }
            LibraryTarget::RerunOcr(key) => intents.push(LibraryIntent::RerunOcrAndCopy(key)),
            LibraryTarget::CopyScreenshot(key) => intents.push(LibraryIntent::CopyScreenshot(key)),
            LibraryTarget::RevealScreenshot(key) => {
                intents.push(LibraryIntent::RevealScreenshot(key))
            }
            _ => {}
        }
    }
    fn begin_edit(&mut self, field: LibraryField) {
        if self.editing.as_ref().is_some_and(|e| e.field == field) {
            return;
        }
        let text = match field {
            LibraryField::Search => self.query.search.clone(),
            LibraryField::Note(key) => self
                .item(key)
                .and_then(|i| i.note.clone())
                .unwrap_or_default(),
            LibraryField::Tags(key) => self
                .item(key)
                .map(|i| i.tags.join(", "))
                .unwrap_or_default(),
        };
        self.editing = Some(Editor {
            field,
            buffer: if matches!(field, LibraryField::Note(_)) {
                TextEdit::multiline(text)
            } else {
                TextEdit::new(text)
            },
            rect: Rect::default(),
            layout: TextLayout::default(),
            scroll: Point::default(),
        });
    }
    fn blur_unless(&mut self, target: LibraryTarget, intents: &mut Vec<LibraryIntent>) {
        if self
            .editing
            .as_ref()
            .is_some_and(|e| Some(e.field) != target.field())
        {
            self.finish_edit(intents);
        }
    }
    fn finish_edit(&mut self, intents: &mut Vec<LibraryIntent>) {
        let Some(mut edit) = self.editing.take() else {
            return;
        };
        edit.buffer.clear_preedit();
        match edit.field {
            LibraryField::Search => {}
            LibraryField::Note(key) => {
                if let Some(item) = self.snapshot.items.iter_mut().find(|i| i.key == key) {
                    let text = edit.buffer.text();
                    let note = (!text.trim().is_empty()).then(|| text.to_owned());
                    if item.note != note {
                        item.note = note.clone();
                        intents.push(LibraryIntent::SaveMetadata {
                            key,
                            note,
                            tags: item.tags.clone(),
                        });
                    }
                }
            }
            LibraryField::Tags(key) => {
                if let Some(item) = self.snapshot.items.iter_mut().find(|i| i.key == key) {
                    let tags = edit
                        .buffer
                        .text()
                        .split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_owned)
                        .collect::<Vec<_>>();
                    if item.tags != tags {
                        item.tags = tags.clone();
                        intents.push(LibraryIntent::SaveMetadata {
                            key,
                            note: item.note.clone(),
                            tags,
                        });
                    }
                }
            }
        }
    }
    fn text_changed(&mut self, intents: &mut Vec<LibraryIntent>) {
        if let Some(edit) = self
            .editing
            .as_ref()
            .filter(|e| e.field == LibraryField::Search)
        {
            if self.query.search != edit.buffer.text() {
                self.query.search = edit.buffer.text().into();
                self.query.page = 0;
                self.list_scroll = 0.;
                self.request(intents);
            }
        }
    }
    fn key(&mut self, key: Key, m: Modifiers, intents: &mut Vec<LibraryIntent>) {
        let style = self.snapshot.shell.key_style;
        if self
            .editing
            .as_ref()
            .is_some_and(|e| e.buffer.is_composing())
        {
            return;
        }
        if key == Key::Tab {
            self.finish_edit(intents);
            self.move_focus(m.shift);
            return;
        }
        if let Some(edit) = self.editing.as_mut() {
            let word = if style == crate::KeyStyle::Mac {
                m.alt
            } else {
                m.control
            };
            if word && matches!(key, Key::ArrowLeft | Key::ArrowRight) {
                edit.buffer.move_caret(
                    if key == Key::ArrowLeft {
                        Movement::WordLeft
                    } else {
                        Movement::WordRight
                    },
                    m.shift,
                );
                return;
            }
            if primary(style, m) {
                match key {
                    Key::Character('a' | 'A') => edit.buffer.select_all(),
                    Key::Character('c' | 'C') => {
                        intents.push(LibraryIntent::CopyText(edit.buffer.selected_text().into()))
                    }
                    Key::Character('x' | 'X') => {
                        intents.push(LibraryIntent::CopyText(edit.buffer.selected_text().into()));
                        edit.buffer.delete(true);
                    }
                    Key::Character('v' | 'V') => intents.push(LibraryIntent::ReadClipboard),
                    _ => {}
                }
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
                    Key::ArrowUp | Key::ArrowDown
                        if matches!(edit.field, LibraryField::Note(_)) =>
                    {
                        let caret = edit.buffer.display_caret().and_then(|byte| {
                            edit.layout
                                .caret_stops
                                .iter()
                                .find(|(b, _)| *b == byte)
                                .map(|(_, p)| *p)
                        });
                        if let Some(caret) = caret {
                            let direction = if key == Key::ArrowUp { -1. } else { 1. };
                            if let Some((byte, _)) = edit
                                .layout
                                .caret_stops
                                .iter()
                                .filter(|(_, p)| (p.y - caret.y) * direction > 1.)
                                .min_by(|(_, a), (_, b)| {
                                    ((a.y - caret.y).abs() * 1000. + (a.x - caret.x).abs())
                                        .total_cmp(
                                            &((b.y - caret.y).abs() * 1000.
                                                + (b.x - caret.x).abs()),
                                        )
                                })
                            {
                                edit.buffer.move_to(*byte, m.shift);
                            }
                        }
                    }
                    Key::Backspace => {
                        edit.buffer.delete(true);
                    }
                    Key::Delete => {
                        edit.buffer.delete(false);
                    }
                    Key::Enter if matches!(edit.field, LibraryField::Note(_)) => {
                        edit.buffer.insert("\n");
                    }
                    Key::Enter => {
                        self.finish_edit(intents);
                        return;
                    }
                    Key::Escape => {
                        self.editing = None;
                        self.focus = None;
                        return;
                    }
                    _ => {}
                }
            }
            self.text_changed(intents);
            return;
        }
        match key {
            Key::Enter | Key::Space => {
                if let Some(target) = self.focus.filter(|t| self.valid_target(*t)) {
                    self.activate(target, intents);
                }
            }
            Key::ArrowDown | Key::ArrowUp => {
                let keys = self
                    .snapshot
                    .items
                    .iter()
                    .map(|i| i.key)
                    .collect::<Vec<_>>();
                if !keys.is_empty() {
                    let index = keys
                        .iter()
                        .position(|key| Some(*key) == self.selected)
                        .unwrap_or(0);
                    let next = if key == Key::ArrowDown {
                        (index + 1).min(keys.len() - 1)
                    } else {
                        index.saturating_sub(1)
                    };
                    self.selected = Some(keys[next]);
                    self.focus = Some(LibraryTarget::Item(keys[next]));
                    self.detail_scroll = 0.;
                    self.ensure_visible(LibraryTarget::Item(keys[next]));
                }
            }
            _ => {}
        }
    }
    fn move_focus(&mut self, backward: bool) {
        let controls = self
            .frame
            .as_ref()
            .map(|f| {
                f.controls
                    .iter()
                    .filter(|c| c.enabled)
                    .map(|c| c.target)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if controls.is_empty() {
            return;
        }
        let index = self
            .focus
            .and_then(|t| controls.iter().position(|c| *c == t));
        let next = if backward {
            index
                .unwrap_or(0)
                .checked_sub(1)
                .unwrap_or(controls.len() - 1)
        } else {
            index.map_or(0, |i| (i + 1) % controls.len())
        };
        let target = controls[next];
        self.focus = Some(target);
        self.ensure_visible(target);
        if let Some(field) = target.field() {
            self.begin_edit(field);
        }
    }
    fn ensure_visible(&mut self, target: LibraryTarget) {
        let Some(rect) = self
            .frame
            .as_ref()
            .and_then(|f| f.controls.iter().find(|c| c.target == target))
            .map(|c| Rect {
                x: c.rect.x / self.scale,
                y: c.rect.y / self.scale,
                width: c.rect.width / self.scale,
                height: c.rect.height / self.scale,
            })
        else {
            return;
        };
        if matches!(target, LibraryTarget::Item(_)) {
            if rect.y < self.list_view.y {
                self.list_scroll = (self.list_scroll + rect.y - self.list_view.y).max(0.);
            } else if rect.y + rect.height > self.list_view.y + self.list_view.height {
                self.list_scroll = (self.list_scroll + rect.y + rect.height
                    - self.list_view.y
                    - self.list_view.height)
                    .min(self.list_limit);
            }
        } else if !matches!(
            target,
            LibraryTarget::Search
                | LibraryTarget::Filter(_)
                | LibraryTarget::PreviousPage
                | LibraryTarget::NextPage
                | LibraryTarget::ClearHistory
        ) {
            if rect.y < self.detail_view.y {
                self.detail_scroll =
                    (self.detail_scroll + rect.y - self.detail_view.y - 8.).max(0.);
            } else if rect.y + rect.height > self.detail_view.y + self.detail_view.height {
                self.detail_scroll = (self.detail_scroll + rect.y + rect.height
                    - self.detail_view.y
                    - self.detail_view.height
                    + 8.)
                    .min(self.detail_limit);
            }
        }
    }
    fn text_pointer(&mut self, point: Point, extend: bool) {
        let Some(edit) = self.editing.as_mut() else {
            return;
        };
        let point = Point {
            x: point.x / self.scale - edit.rect.x + edit.scroll.x,
            y: point.y / self.scale - edit.rect.y + edit.scroll.y,
        };
        if let Some((byte, _)) = edit.layout.caret_stops.iter().min_by(|(_, a), (_, b)| {
            ((a.y - point.y).abs() * 1000. + (a.x - point.x).abs())
                .total_cmp(&((b.y - point.y).abs() * 1000. + (b.x - point.x).abs()))
        }) {
            edit.buffer.move_to(*byte, extend);
        }
    }
    pub(crate) fn logical_size(&mut self, size: Size, scale: f32) -> Size {
        self.scale = if scale.is_finite() {
            scale.clamp(0.5, 3.)
        } else {
            1.
        };
        Size {
            width: (size.width / self.scale).max(1.),
            height: (size.height / self.scale).max(1.),
        }
    }
}
