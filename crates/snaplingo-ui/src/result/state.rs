use super::*;
use crate::{Movement, TextEdit, TextLayout};

pub struct ResultUi {
    pub(super) snapshot: ResultSnapshot,
    pub(super) controls: Vec<ResultControl>,
    pub(super) focus: Option<ResultTarget>,
    pub(super) editor: Option<TextEdit>,
    pub(super) text_regions: Vec<(ResultTarget, Rect, TextLayout)>,
    pub(super) menu: Option<bool>,
    pub(super) scroll: f32,
    pub(super) max_scroll: f32,
    pub(super) scale: f32,
    dragging: bool,
    pub(super) pinned: bool,
    pub(super) source_collapsed: bool,
    pub(super) collapsed_rows: std::collections::HashSet<String>,
}
impl ResultUi {
    pub fn new(snapshot: ResultSnapshot) -> Self {
        Self {
            snapshot,
            controls: vec![],
            focus: None,
            editor: None,
            text_regions: vec![],
            menu: None,
            scroll: 0.,
            max_scroll: 0.,
            scale: 1.,
            dragging: false,
            pinned: false,
            source_collapsed: false,
            collapsed_rows: Default::default(),
        }
    }
    pub fn row_expanded(&self, id: &str) -> bool {
        !self.collapsed_rows.contains(id)
    }
    pub fn pinned(&self) -> bool {
        self.pinned
    }
    pub fn snapshot(&self) -> &ResultSnapshot {
        &self.snapshot
    }
    pub fn focused(&self) -> Option<&ResultTarget> {
        self.focus.as_ref()
    }
    pub fn text_edit(&self) -> Option<&TextEdit> {
        self.editor.as_ref()
    }
    pub fn replace(&mut self, snapshot: ResultSnapshot) {
        if snapshot.revision < self.snapshot.revision {
            return;
        }
        let changed = self.snapshot.mode != snapshot.mode;
        self.snapshot = snapshot;
        if changed {
            self.focus = None;
            self.editor = None;
            self.menu = None;
            self.scroll = 0.;
        }
        if let Some(target) = &self.focus {
            let text = self.text(target);
            if self
                .editor
                .as_ref()
                .is_some_and(|e| Some(e.text()) != text.as_deref())
            {
                self.editor = text.map(TextEdit::multiline);
            }
            if !self.snapshot.composing {
                if let Some(edit) = &mut self.editor {
                    edit.clear_preedit();
                }
            }
        }
    }
    fn text(&self, target: &ResultTarget) -> Option<String> {
        match target {
            ResultTarget::Source => Some(self.snapshot.source.clone()),
            ResultTarget::OcrText => Some(self.snapshot.ocr_text.clone()),
            ResultTarget::ResultText(id) => self
                .snapshot
                .rows
                .iter()
                .find(|r| r.id == *id)
                .map(|r| r.text.clone()),
            _ => None,
        }
    }
    fn focus(&mut self, target: ResultTarget) {
        if self.focus.as_ref() != Some(&target) {
            self.editor = self.text(&target).map(TextEdit::multiline);
            self.focus = Some(target);
        }
    }
    fn edited(&mut self) -> Vec<ResultIntent> {
        let Some(edit) = self
            .editor
            .as_ref()
            .filter(|_| self.focus == Some(ResultTarget::Source))
        else {
            return vec![];
        };
        self.snapshot.source = edit.text().into();
        self.snapshot.composing = edit.is_composing();
        vec![ResultIntent::Edit(edit.text().into(), edit.is_composing())]
    }
    fn point(&mut self, point: Point, extend: bool) {
        let Some((_, rect, layout)) = self
            .text_regions
            .iter()
            .find(|(target, _, _)| Some(target) == self.focus.as_ref())
        else {
            return;
        };
        let local = Point {
            x: point.x / self.scale - rect.x,
            y: point.y / self.scale - rect.y,
        };
        let nearest = layout
            .caret_stops
            .iter()
            .min_by(|(_, a), (_, b)| {
                let distance = |p: &Point| (p.y - local.y).abs() * 10000. + (p.x - local.x).abs();
                distance(a).total_cmp(&distance(b))
            })
            .map(|(byte, _)| *byte);
        if let (Some(edit), Some(byte)) = (&mut self.editor, nearest) {
            edit.move_to(byte, extend);
        }
    }
    fn activate(&mut self, target: ResultTarget) -> Vec<ResultIntent> {
        if !self
            .controls
            .iter()
            .any(|c| c.target == target && c.enabled)
        {
            return vec![];
        }
        if self.text(&target).is_some() {
            self.focus(target);
            self.menu = None;
            return vec![];
        }
        self.focus(target.clone());
        let snapshot = &self.snapshot;
        let intent = match &target {
            ResultTarget::Close => ResultIntent::Close,
            ResultTarget::Header => ResultIntent::Drag,
            ResultTarget::Pin => {
                self.pinned = !self.pinned;
                return vec![];
            }
            ResultTarget::ToggleSource => {
                self.source_collapsed = !self.source_collapsed;
                self.editor = None;
                return vec![];
            }
            ResultTarget::ToggleRow(id) => {
                if !self.collapsed_rows.remove(id) {
                    self.collapsed_rows.insert(id.clone());
                }
                return vec![];
            }
            ResultTarget::ClearSource => ResultIntent::Edit(String::new(), false),
            ResultTarget::Translate => ResultIntent::Translate,
            ResultTarget::Swap => ResultIntent::Swap,
            ResultTarget::SourceLanguage | ResultTarget::TargetLanguage => {
                let source = target == ResultTarget::SourceLanguage;
                self.menu = if self.menu == Some(source) {
                    None
                } else {
                    Some(source)
                };
                self.focus = Some(target);
                self.editor = None;
                return vec![];
            }
            ResultTarget::Language(source, code) => {
                let intent = if *source {
                    ResultIntent::SourceLanguage(code.clone())
                } else {
                    ResultIntent::TargetLanguage(code.clone())
                };
                self.menu = None;
                return vec![intent];
            }
            ResultTarget::CopySource => ResultIntent::Copy(snapshot.source.clone()),
            ResultTarget::SpeakSource => ResultIntent::Speak(
                snapshot.source.clone(),
                snapshot.detected_language.clone().or_else(|| {
                    (snapshot.source_language != "auto").then(|| snapshot.source_language.clone())
                }),
            ),
            ResultTarget::FavoriteAll => ResultIntent::Favorite(
                snapshot
                    .rows
                    .iter()
                    .filter(|r| r.status == RowStatus::Success && !r.favorite && !r.text.is_empty())
                    .map(|r| r.id.clone())
                    .collect(),
            ),
            ResultTarget::FileOcr => ResultIntent::FileOcr,
            ResultTarget::ClearOcr => ResultIntent::ClearOcr,
            ResultTarget::CopyOcr => ResultIntent::Copy(snapshot.ocr_text.clone()),
            ResultTarget::CopyToken(value) => ResultIntent::Copy(value.clone()),
            ResultTarget::SpeakOcr => ResultIntent::Speak(snapshot.ocr_text.clone(), None),
            ResultTarget::FavoriteOcr => ResultIntent::Favorite(vec![]),
            ResultTarget::Retry(id) => ResultIntent::Retry(id.clone()),
            ResultTarget::FavoriteResult(id) => ResultIntent::Favorite(vec![id.clone()]),
            ResultTarget::CopyResult(id) | ResultTarget::SpeakResult(id) => {
                let Some(row) = snapshot.rows.iter().find(|r| r.id == *id) else {
                    return vec![];
                };
                if matches!(target, ResultTarget::CopyResult(_)) {
                    ResultIntent::Copy(row.text.clone())
                } else {
                    ResultIntent::Speak(row.text.clone(), Some(snapshot.target_language.clone()))
                }
            }
            _ => return vec![],
        };
        self.menu = None;
        if matches!(intent, ResultIntent::Edit(..)) {
            self.snapshot.source.clear();
            self.focus = Some(ResultTarget::Source);
            self.editor = Some(TextEdit::multiline(String::new()));
        }
        vec![intent]
    }
    pub fn update(&mut self, input: ResultInput) -> Vec<ResultIntent> {
        match input {
            ResultInput::Activate(target) => return self.activate(target),
            ResultInput::Focus(target) => {
                if self
                    .controls
                    .iter()
                    .any(|c| c.target == target && c.enabled)
                {
                    self.focus(target);
                }
            }
            ResultInput::SetSource(text) => {
                if self.snapshot.mode == ResultMode::Translation {
                    self.focus = Some(ResultTarget::Source);
                    self.editor = Some(TextEdit::multiline(text));
                    return self.edited();
                }
            }
            ResultInput::PointerDown(point) => {
                let target = self
                    .controls
                    .iter()
                    .rev()
                    .find(|c| c.enabled && c.hit_rect.contains(point))
                    .map(|c| c.target.clone());
                let Some(target) = target else {
                    self.menu = None;
                    return vec![];
                };
                let text = self.text(&target).is_some();
                let intents = self.activate(target);
                if text {
                    self.point(point, false);
                }
                self.dragging = text;
                return intents;
            }
            ResultInput::PointerMove(point) => {
                if self.dragging {
                    self.point(point, true);
                }
            }
            ResultInput::PointerUp => self.dragging = false,
            ResultInput::Scroll(delta) => {
                self.scroll = (self.scroll + delta / self.scale).clamp(0., self.max_scroll);
            }
            ResultInput::Text(text) | ResultInput::Commit(text) => {
                if self.focus == Some(ResultTarget::Source) {
                    if let Some(edit) = &mut self.editor {
                        edit.commit(&text);
                    }
                    return self.edited();
                }
            }
            ResultInput::Preedit(text, selection) => {
                if self.focus == Some(ResultTarget::Source) {
                    if let Some(edit) = &mut self.editor {
                        edit.preedit(&text, selection);
                    }
                    return self.edited();
                }
            }
            ResultInput::ImeDisabled => {
                if let Some(edit) = &mut self.editor {
                    edit.clear_preedit();
                }
                return self.edited();
            }
            ResultInput::Selection(selection) => {
                if let Some(edit) = &mut self.editor {
                    edit.set_selection(selection);
                }
            }
            ResultInput::FocusLost => {
                self.dragging = false;
                self.menu = None;
            }
            ResultInput::Key(key, modifiers) => return self.key(key, modifiers),
        }
        vec![]
    }
    fn key(&mut self, key: Key, modifiers: Modifiers) -> Vec<ResultIntent> {
        let command = modifiers.control || modifiers.meta;
        if key == Key::Escape {
            if self.menu.take().is_some() {
                return vec![];
            }
            return vec![ResultIntent::Close];
        }
        if let Some(source) = self.menu {
            if matches!(key, Key::ArrowDown | Key::ArrowUp) {
                let selected = self.focus.as_ref().and_then(|focus| {
                    LANGUAGES.iter().position(|(code, _)| {
                        *focus == ResultTarget::Language(source, (*code).into())
                    })
                });
                let index = if key == Key::ArrowDown {
                    selected.map_or(0, |i| (i + 1) % LANGUAGES.len())
                } else {
                    selected.map_or(LANGUAGES.len() - 1, |i| {
                        (i + LANGUAGES.len() - 1) % LANGUAGES.len()
                    })
                };
                self.focus(ResultTarget::Language(source, LANGUAGES[index].0.into()));
                return vec![];
            }
        }
        if key == Key::Tab {
            let targets: Vec<_> = self
                .controls
                .iter()
                .filter(|c| c.enabled && c.hit_rect.height > 0.)
                .map(|c| c.target.clone())
                .collect();
            if !targets.is_empty() {
                let index = self
                    .focus
                    .as_ref()
                    .and_then(|focus| targets.iter().position(|t| t == focus));
                let next = if modifiers.shift {
                    index.map_or(targets.len() - 1, |i| {
                        (i + targets.len() - 1) % targets.len()
                    })
                } else {
                    index.map_or(0, |i| (i + 1) % targets.len())
                };
                self.focus(targets[next].clone());
            }
            return vec![];
        }
        let editable = self.focus == Some(ResultTarget::Source);
        if let Some(edit) = &mut self.editor {
            if edit.is_composing() {
                return vec![];
            }
            match key {
                Key::Character(c) if command && c.eq_ignore_ascii_case(&'a') => edit.select_all(),
                Key::Character(c) if command && c.eq_ignore_ascii_case(&'c') => {
                    return (!edit.selected_text().is_empty())
                        .then(|| ResultIntent::Copy(edit.selected_text().into()))
                        .into_iter()
                        .collect();
                }
                Key::Character(c) if command && c.eq_ignore_ascii_case(&'v') && editable => {
                    return vec![ResultIntent::ReadClipboard]
                }
                Key::Character(c) if command && c.eq_ignore_ascii_case(&'x') && editable => {
                    let text = edit.selected_text().to_owned();
                    if text.is_empty() {
                        return vec![];
                    }
                    edit.insert("");
                    let mut intents = vec![ResultIntent::Copy(text)];
                    intents.extend(self.edited());
                    return intents;
                }
                Key::ArrowLeft | Key::ArrowRight | Key::Home | Key::End => {
                    let movement = match (key, command || modifiers.alt) {
                        (Key::ArrowLeft, true) => Movement::WordLeft,
                        (Key::ArrowLeft, false) => Movement::Left,
                        (Key::ArrowRight, true) => Movement::WordRight,
                        (Key::ArrowRight, false) => Movement::Right,
                        (Key::Home, _) => Movement::Home,
                        _ => Movement::End,
                    };
                    edit.move_caret(movement, modifiers.shift);
                }
                Key::Backspace | Key::Delete if editable => {
                    edit.delete(key == Key::Backspace);
                    return self.edited();
                }
                Key::Enter if editable && modifiers.shift => {
                    edit.insert("\n");
                    return self.edited();
                }
                Key::Enter if editable => return vec![ResultIntent::Translate],
                _ => {}
            }
            return vec![];
        }
        if matches!(key, Key::Enter | Key::Space) {
            if let Some(target) = self.focus.clone() {
                return self.activate(target);
            }
        }
        if matches!(key, Key::PageDown | Key::PageUp) {
            self.scroll = (self.scroll + if key == Key::PageDown { 250. } else { -250. })
                .clamp(0., self.max_scroll);
        }
        vec![]
    }
}
