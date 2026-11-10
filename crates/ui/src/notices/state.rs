use super::*;
use crate::{Appearance, Movement, TextEdit, TextLayout};
pub struct NoticesUi {
    pub(super) appearance: Appearance,
    pub(super) components: Vec<NoticeComponent>,
    pub(super) filtered: Vec<usize>,
    pub(super) selected: Option<usize>,
    pub(super) focus: Option<NoticesTarget>,
    pub(super) editor: Option<TextEdit>,
    pub(super) search: TextEdit,
    pub(super) controls: Vec<NoticesControl>,
    pub(super) scroll: f32,
    pub(super) max_scroll: f32,
    pub(super) scale: f32,
    pub(super) layout: TextLayout,
    pub(super) text_rect: Rect,
    dragging: bool,
}
impl NoticesUi {
    pub fn new(appearance: Appearance, components: Vec<NoticeComponent>) -> Self {
        let filtered = (0..components.len()).collect();
        Self {
            appearance,
            components,
            filtered,
            selected: None,
            focus: Some(NoticesTarget::Search),
            editor: None,
            search: TextEdit::new(String::new()),
            controls: vec![],
            scroll: 0.,
            max_scroll: 0.,
            scale: 1.,
            layout: TextLayout::default(),
            text_rect: Rect::default(),
            dragging: false,
        }
    }
    pub fn focused(&self) -> Option<&NoticesTarget> {
        self.focus.as_ref()
    }
    pub fn query(&self) -> &str {
        self.search.text()
    }
    pub fn selected(&self) -> Option<&NoticeComponent> {
        self.selected.and_then(|index| self.components.get(index))
    }
    pub fn is_composing(&self) -> bool {
        self.focus == Some(NoticesTarget::Search) && self.search.is_composing()
    }
    fn filter(&mut self) {
        let query = self.search.text().to_lowercase();
        self.filtered = self
            .components
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                format!("{} {}", c.name, c.license)
                    .to_lowercase()
                    .contains(&query)
            })
            .map(|(index, _)| index)
            .collect();
        self.selected = None;
        self.scroll = 0.;
        self.editor = None;
    }
    fn activate(&mut self, target: NoticesTarget) -> Vec<NoticesIntent> {
        if !self.controls.iter().any(|c| c.target == target) {
            return vec![];
        }
        self.focus = Some(target.clone());
        match target {
            NoticesTarget::Close => return vec![NoticesIntent::Close],
            NoticesTarget::ClearSearch => {
                self.search = TextEdit::new(String::new());
                self.filter();
                self.focus = Some(NoticesTarget::Search);
            }
            NoticesTarget::Search => self.editor = None,
            NoticesTarget::Component(index) => {
                if self.filtered.contains(&index) {
                    self.selected = Some(index);
                    self.scroll = 0.;
                    self.editor = None;
                }
            }
            NoticesTarget::Back => {
                self.selected = None;
                self.scroll = 0.;
                self.editor = None;
            }
            NoticesTarget::Source => {
                if let Some(component) = self.selected() {
                    return vec![NoticesIntent::OpenSource(component.source.clone())];
                }
            }
            NoticesTarget::License => {
                if let Some(component) = self.selected() {
                    self.editor = Some(TextEdit::multiline(component.text.clone()));
                }
            }
        }
        vec![]
    }
    fn point(&mut self, point: Point, extend: bool) {
        if self.focus != Some(NoticesTarget::License) {
            return;
        }
        let local = Point {
            x: point.x / self.scale - self.text_rect.x,
            y: point.y / self.scale - self.text_rect.y,
        };
        if let Some((byte, _)) = self.layout.caret_stops.iter().min_by(|(_, a), (_, b)| {
            let distance = |p: &Point| (p.y - local.y).abs() * 10000. + (p.x - local.x).abs();
            distance(a).total_cmp(&distance(b))
        }) {
            if let Some(edit) = &mut self.editor {
                edit.move_to(*byte, extend);
            }
        }
    }
    pub fn update(&mut self, input: NoticesInput) -> Vec<NoticesIntent> {
        match input {
            NoticesInput::Activate(target) => return self.activate(target),
            NoticesInput::Focus(target) => {
                if self.controls.iter().any(|c| c.target == target) {
                    self.focus = Some(target);
                }
            }
            NoticesInput::SetSearch(text) => {
                self.search = TextEdit::new(text);
                self.filter();
                self.focus = Some(NoticesTarget::Search);
            }
            NoticesInput::PointerDown(point) => {
                let target = self
                    .controls
                    .iter()
                    .rev()
                    .find(|c| c.hit_rect.contains(point))
                    .map(|c| c.target.clone());
                if let Some(target) = target {
                    let selecting = target == NoticesTarget::License;
                    let intents = self.activate(target);
                    if selecting {
                        self.point(point, false);
                    }
                    self.dragging = selecting;
                    return intents;
                }
            }
            NoticesInput::PointerMove(point) => {
                if self.dragging {
                    self.point(point, true);
                }
            }
            NoticesInput::PointerUp => self.dragging = false,
            NoticesInput::Scroll(delta) => {
                self.scroll = (self.scroll + delta / self.scale).clamp(0., self.max_scroll)
            }
            NoticesInput::Text(text) | NoticesInput::Commit(text) => {
                if self.focus == Some(NoticesTarget::Search) {
                    self.search.commit(&text);
                    self.filter();
                }
            }
            NoticesInput::Preedit(text, selection) => {
                if self.focus == Some(NoticesTarget::Search) {
                    self.search.preedit(&text, selection);
                }
            }
            NoticesInput::ImeDisabled => {
                self.search.clear_preedit();
            }
            NoticesInput::Key(key, m) => return self.key(key, m),
        }
        vec![]
    }
    fn key(&mut self, key: Key, m: Modifiers) -> Vec<NoticesIntent> {
        if self.is_composing() {
            return vec![];
        }
        if key == Key::Escape {
            if self.selected.take().is_some() {
                self.scroll = 0.;
                self.editor = None;
                return vec![];
            }
            return vec![NoticesIntent::Close];
        }
        if key == Key::Tab {
            let targets: Vec<_> = self
                .controls
                .iter()
                .filter(|c| c.hit_rect.height > 0.)
                .map(|c| c.target.clone())
                .collect();
            if !targets.is_empty() {
                let current = self
                    .focus
                    .as_ref()
                    .and_then(|f| targets.iter().position(|t| t == f));
                let next = if m.shift {
                    current.map_or(targets.len() - 1, |i| {
                        (i + targets.len() - 1) % targets.len()
                    })
                } else {
                    current.map_or(0, |i| (i + 1) % targets.len())
                };
                self.focus = Some(targets[next].clone());
            }
            return vec![];
        }
        let search = self.focus == Some(NoticesTarget::Search);
        if search || self.focus == Some(NoticesTarget::License) {
            if !search && self.editor.is_none() {
                if let Some(c) = self.selected() {
                    self.editor = Some(TextEdit::multiline(c.text.clone()));
                }
            }
            let edit = if search {
                Some(&mut self.search)
            } else {
                self.editor.as_mut()
            };
            if let Some(edit) = edit {
                let command = m.control || m.meta;
                match key {
                    Key::Character(c) if command && c.eq_ignore_ascii_case(&'a') => {
                        edit.select_all()
                    }
                    Key::Character(c) if command && c.eq_ignore_ascii_case(&'c') => {
                        return vec![NoticesIntent::Copy(edit.selected_text().into())]
                    }
                    Key::Character(c) if command && c.eq_ignore_ascii_case(&'v') && search => {
                        return vec![NoticesIntent::ReadClipboard]
                    }
                    Key::Backspace | Key::Delete if search => {
                        edit.delete(key == Key::Backspace);
                        self.filter();
                    }
                    Key::ArrowLeft | Key::ArrowRight | Key::Home | Key::End => {
                        edit.move_caret(
                            match key {
                                Key::ArrowLeft => Movement::Left,
                                Key::ArrowRight => Movement::Right,
                                Key::Home => Movement::Home,
                                _ => Movement::End,
                            },
                            m.shift,
                        );
                    }
                    _ => {}
                }
            }
        } else if matches!(key, Key::Enter | Key::Space) {
            if let Some(target) = self.focus.clone() {
                return self.activate(target);
            }
        }
        if key == Key::PageDown || key == Key::PageUp {
            self.scroll = (self.scroll + if key == Key::PageDown { 300. } else { -300. })
                .clamp(0., self.max_scroll);
        }
        vec![]
    }
}
