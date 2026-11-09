use super::*;
use crate::{Key, KeyStyle, Modifiers, Movement, Point, Rect, Target, TextEdit, TextLayout};
use std::collections::BTreeMap;

pub struct ServicesUi {
    pub(crate) snapshot: ServicesSnapshot,
    pub(crate) epoch: u64,
    pub(crate) credentials: BTreeMap<String, String>,
    pub(crate) focus: Option<ServiceControl>,
    pub(crate) hover: Option<ServiceControl>,
    pub(crate) editing: Option<Editor>,
    pub(crate) drag: Option<(String, Option<String>)>,
    pub(crate) rate_drag: bool,
    pub(crate) voice_menu: bool,
    pub(crate) voice_scroll: f32,
    pub(crate) voice_view: Rect,
    pub(crate) voice_limit: f32,
    pub(crate) selecting: bool,
    pub(crate) scroll: f32,
    pub(crate) scroll_limit: f32,
    pub(crate) viewport: Rect,
    pub(crate) frame: Option<ServicesFrame>,
    pub(crate) provider_rows: Vec<(String, Rect)>,
    pub(crate) input_layouts: Vec<(ServiceField, Rect, TextLayout)>,
    pub(crate) scale: f32,
}
pub(crate) struct Editor {
    pub field: ServiceField,
    pub buffer: TextEdit,
    pub rect: Rect,
    pub layout: TextLayout,
    pub scroll: Point,
}
impl ServicesUi {
    pub fn new(snapshot: ServicesSnapshot) -> Self {
        Self {
            snapshot,
            epoch: 1,
            credentials: BTreeMap::new(),
            focus: None,
            hover: None,
            editing: None,
            drag: None,
            rate_drag: false,
            voice_menu: false,
            voice_scroll: 0.,
            voice_view: Rect::default(),
            voice_limit: 0.,
            selecting: false,
            scroll: 0.,
            scroll_limit: 0.,
            viewport: Rect::default(),
            frame: None,
            provider_rows: vec![],
            input_layouts: vec![],
            scale: 1.,
        }
    }
    pub fn snapshot(&self) -> &ServicesSnapshot {
        &self.snapshot
    }
    pub fn focused(&self) -> Option<ServicesTarget> {
        self.focus.clone().map(|control| ServicesTarget {
            revision: self.epoch,
            control,
        })
    }
    pub fn text_edit(&self) -> Option<&TextEdit> {
        self.editing.as_ref().map(|editor| &editor.buffer)
    }
    pub fn update(&mut self, input: ServicesInput) -> Vec<ServicesIntent> {
        let mut intents = vec![];
        match input {
            ServicesInput::Refresh => intents.push(ServicesIntent::Refresh),
            ServicesInput::Snapshot(snapshot) => {
                if snapshot.revision < self.snapshot.revision {
                    return intents;
                }
                let changed = snapshot.route != self.snapshot.route
                    || snapshot.form_revision != self.snapshot.form_revision
                    || std::mem::discriminant(&snapshot.form)
                        != std::mem::discriminant(&self.snapshot.form);
                let providers_changed = snapshot
                    .providers
                    .iter()
                    .map(|provider| (&provider.id, provider.active))
                    .collect::<Vec<_>>()
                    != self
                        .snapshot
                        .providers
                        .iter()
                        .map(|provider| (&provider.id, provider.active))
                        .collect::<Vec<_>>();
                if changed {
                    self.editing = None;
                    self.credentials.clear();
                    self.focus = None;
                    self.voice_menu = false;
                    self.scroll = 0.;
                }
                self.snapshot = snapshot;
                self.epoch = self.epoch.wrapping_add(1);
                if changed || providers_changed {
                    self.drag = None;
                }
                if changed || self.snapshot.busy {
                    self.rate_drag = false;
                    self.selecting = false;
                }
                self.frame = None;
            }
            ServicesInput::Activate(target) => {
                if self.valid(&target) {
                    self.blur_unless(&target.control);
                    self.focus = Some(target.control.clone());
                    self.ensure_visible(&target.control);
                    self.activate(target.control, &mut intents);
                }
            }
            ServicesInput::Focus(target) => {
                if self.valid(&target) {
                    self.blur_unless(&target.control);
                    self.focus = Some(target.control.clone());
                    self.ensure_visible(&target.control);
                    if let ServiceControl::Field(field) = target.control {
                        self.begin_edit(field);
                    }
                }
            }
            ServicesInput::SetValue(target, value) => {
                if self.valid(&target) {
                    match target.control {
                        ServiceControl::Field(field) => {
                            self.begin_edit(field);
                            if let Some(editor) = self.editing.as_mut() {
                                editor.buffer.select_all();
                                editor.buffer.insert(&value);
                            }
                            self.text_changed(&mut intents);
                        }
                        ServiceControl::Rate => {
                            if let Ok(rate) = value.parse::<u32>() {
                                if (80..=400).contains(&rate) {
                                    self.snapshot.rate = rate;
                                    intents.push(ServicesIntent::Rate(rate));
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            ServicesInput::PointerDown(point) => {
                let target = self.frame.as_ref().and_then(|frame| frame.hit_test(point));
                if let Some(target) = target {
                    self.blur_unless(&target.control);
                    self.focus = Some(target.control.clone());
                    match &target.control {
                        ServiceControl::Drag(id) => self.drag = Some((id.clone(), None)),
                        ServiceControl::Rate => {
                            self.rate_drag = true;
                            self.drag_rate(point, &mut intents);
                        }
                        _ => self.activate(target.control, &mut intents),
                    }
                    self.text_pointer(point, false);
                } else {
                    self.editing = None;
                    self.focus = None;
                    self.voice_menu = false;
                }
                self.selecting = self.editing.is_some();
            }
            ServicesInput::PointerMove(point) => {
                self.hover = self
                    .frame
                    .as_ref()
                    .and_then(|frame| frame.hit_test(point))
                    .map(|target| target.control)
                    .or_else(|| self.provider_at(point).map(ServiceControl::Configure));
                if self.selecting {
                    self.text_pointer(point, true);
                }
                if self.rate_drag {
                    self.drag_rate(point, &mut intents);
                }
                if let Some((source, _)) = self.drag.clone() {
                    let target = self
                        .provider_at(point)
                        .filter(|id| id != &source && self.same_group(&source, id));
                    self.drag = Some((source, target));
                }
            }
            ServicesInput::PointerUp(point) => {
                self.selecting = false;
                if self.rate_drag {
                    self.drag_rate(point, &mut intents);
                    self.rate_drag = false;
                }
                if let Some((source, _)) = self.drag.take() {
                    if let Some(target) = self
                        .provider_at(point)
                        .filter(|id| id != &source && self.same_group(&source, id))
                    {
                        intents.push(ServicesIntent::Reorder {
                            source_id: source,
                            target_id: target,
                        });
                    }
                }
            }
            ServicesInput::PointerCancel => {
                self.drag = None;
                self.rate_drag = false;
                self.selecting = false;
            }
            ServicesInput::Scroll { position, delta_y } => {
                if delta_y.is_finite() {
                    let p = Point {
                        x: position.x / self.scale,
                        y: position.y / self.scale,
                    };
                    if self.voice_menu && self.voice_view.contains(p) {
                        self.voice_scroll =
                            (self.voice_scroll + delta_y / self.scale).clamp(0., self.voice_limit);
                    } else if self.viewport.contains(p) {
                        if let Some(editor) = self
                            .editing
                            .as_mut()
                            .filter(|editor| editor.field.multiline() && editor.rect.contains(p))
                        {
                            editor.scroll.y = (editor.scroll.y + delta_y / self.scale).clamp(
                                0.,
                                (editor.layout.height - editor.rect.height + 24.).max(0.),
                            );
                        } else {
                            self.scroll =
                                (self.scroll + delta_y / self.scale).clamp(0., self.scroll_limit);
                        }
                    }
                }
            }
            ServicesInput::Text(text) | ServicesInput::ClipboardText(text) => {
                if !self.snapshot.busy {
                    if let Some(editor) = self.editing.as_mut() {
                        editor.buffer.insert(&text);
                        self.text_changed(&mut intents);
                    }
                }
            }
            ServicesInput::Selection(range) => {
                if let Some(editor) = self.editing.as_mut() {
                    editor.buffer.set_selection(range);
                }
            }
            ServicesInput::ImePreedit { text, selection } => {
                if let Some(editor) = self.editing.as_mut() {
                    editor.buffer.preedit(&text, selection);
                }
            }
            ServicesInput::ImeCommit(text) => {
                if !self.snapshot.busy {
                    if let Some(editor) = self.editing.as_mut() {
                        editor.buffer.commit(&text);
                        self.text_changed(&mut intents);
                    }
                }
            }
            ServicesInput::ImeDisabled => {
                if let Some(editor) = self.editing.as_mut() {
                    editor.buffer.clear_preedit();
                }
            }
            ServicesInput::FocusLost => {
                self.editing = None;
                self.focus = None;
                self.drag = None;
                self.rate_drag = false;
                self.voice_menu = false;
            }
            ServicesInput::KeyDown { key, modifiers } => self.key(key, modifiers, &mut intents),
        }
        intents
    }
    fn valid(&self, target: &ServicesTarget) -> bool {
        target.revision == self.epoch
            && self.frame.as_ref().is_some_and(|frame| {
                frame
                    .controls
                    .iter()
                    .any(|control| control.target == *target && control.enabled)
            })
    }
    fn same_group(&self, source: &str, target: &str) -> bool {
        self.snapshot.route == ServicesRoute::Translation
            && self
                .snapshot
                .providers
                .iter()
                .find(|p| p.id == source)
                .zip(self.snapshot.providers.iter().find(|p| p.id == target))
                .is_some_and(|(s, t)| s.active == t.active)
    }
    fn provider_at(&self, point: Point) -> Option<String> {
        if !self.frame.as_ref()?.scene.viewport.contains(point) {
            return None;
        }
        let point = Point {
            x: point.x / self.scale,
            y: point.y / self.scale,
        };
        self.provider_rows
            .iter()
            .find(|(_, rect)| rect.contains(point))
            .map(|(id, _)| id.clone())
    }
    fn activate(&mut self, control: ServiceControl, intents: &mut Vec<ServicesIntent>) {
        let revision = self.snapshot.form_revision;
        let route = self.snapshot.route;
        match control {
            ServiceControl::Shell(Target::Navigate(page)) => {
                intents.push(ServicesIntent::Navigate(page))
            }
            ServiceControl::Shell(Target::Capacity) => {
                intents.push(ServicesIntent::ConfigureCapacity)
            }
            ServiceControl::Route(route) => {
                if route != self.snapshot.route {
                    self.snapshot.route = route;
                    self.snapshot.form = None;
                    self.editing = None;
                    self.scroll = 0.;
                    self.epoch += 1;
                    self.frame = None;
                    intents.push(ServicesIntent::Route(route));
                }
            }
            ServiceControl::Configure(provider_id) => {
                intents.push(ServicesIntent::Configure { route, provider_id })
            }
            ServiceControl::SetActive {
                provider_id,
                active,
            } => intents.push(ServicesIntent::SetActive {
                route,
                provider_id,
                active,
            }),
            ServiceControl::TestSaved(id) => intents.push(ServicesIntent::TestSaved(id)),
            ServiceControl::Remove(id) => intents.push(ServicesIntent::ConfirmRemove(id)),
            ServiceControl::AddCustom => intents.push(ServicesIntent::AddCustom),
            ServiceControl::CancelForm => {
                self.editing = None;
                self.credentials.clear();
                intents.push(ServicesIntent::CancelForm);
            }
            ServiceControl::SaveForm => match &self.snapshot.form {
                Some(ServiceForm::Credentials { provider_id, .. }) => {
                    intents.push(ServicesIntent::SaveCredentials {
                        route,
                        revision,
                        provider_id: provider_id.clone(),
                        credentials: self.credentials.clone(),
                    })
                }
                Some(ServiceForm::Custom { .. }) => {
                    intents.push(ServicesIntent::SaveCustom { revision })
                }
                _ => {}
            },
            ServiceControl::Field(field) => self.begin_edit(field),
            ServiceControl::DeepLMode(standard) => {
                self.credentials.insert(
                    "mode".into(),
                    if standard { "deepl" } else { "deeplx" }.into(),
                );
                self.editing = None;
            }
            ServiceControl::Protocol(protocol) => {
                if let Some(ServiceForm::Custom { draft, .. }) = &mut self.snapshot.form {
                    if draft.protocol != protocol {
                        draft.protocol = protocol;
                        let (endpoint, model) = protocol.defaults();
                        draft.endpoint = endpoint.into();
                        draft.model = model.into();
                        self.snapshot.models.clear();
                        self.snapshot.model_error = None;
                        self.snapshot.test_status = None;
                        intents.push(ServicesIntent::Protocol { revision, protocol });
                    }
                }
            }
            ServiceControl::ProtocolFamily(protocol) => {
                let current = match &self.snapshot.form {
                    Some(ServiceForm::Custom { draft, .. }) => Some(draft.protocol),
                    _ => None,
                };
                if current != Some(protocol)
                    && !(protocol == ServiceProtocol::OpenAi
                        && current == Some(ServiceProtocol::OpenAiResponses))
                {
                    self.activate(ServiceControl::Protocol(protocol), intents);
                }
            }
            ServiceControl::Reasoning(value) => {
                if let Some(ServiceForm::Custom { draft, .. }) = &mut self.snapshot.form {
                    draft.reasoning_level = value.clone();
                    intents.push(ServicesIntent::Reasoning { revision, value });
                }
            }
            ServiceControl::PromptStrategy(id) => {
                if let Some(ServiceForm::Custom { draft, prompts }) = &mut self.snapshot.form {
                    draft.prompt_strategy_id = id.clone();
                    prompts.error = None;
                    prompts.draft = prompts
                        .strategies
                        .iter()
                        .find(|strategy| strategy.id == id)
                        .map(|strategy| ServicePromptDraft {
                            name: strategy.name.clone(),
                            description: strategy.description.clone(),
                            system_prompt: strategy.system_prompt.clone(),
                        })
                        .unwrap_or_default();
                    self.editing = None;
                    intents.push(ServicesIntent::PromptStrategy {
                        revision,
                        provider_strategy_id: id,
                    });
                }
            }
            ServiceControl::LoadModels => intents.push(ServicesIntent::LoadModels { revision }),
            ServiceControl::TestConnection => {
                intents.push(ServicesIntent::TestConnection { revision })
            }
            ServiceControl::ChooseModel(value) => {
                if let Some(ServiceForm::Custom { draft, .. }) = &mut self.snapshot.form {
                    draft.model = value.clone();
                    self.snapshot.test_status = None;
                    intents.push(ServicesIntent::EditCustom {
                        revision,
                        field: ServiceCustomField::Model,
                        value,
                    });
                }
            }
            ServiceControl::SavePrompt => intents.push(ServicesIntent::SavePrompt { revision }),
            ServiceControl::AddPrompt => intents.push(ServicesIntent::AddPrompt { revision }),
            ServiceControl::DeletePrompt => intents.push(ServicesIntent::DeletePrompt { revision }),
            ServiceControl::VoiceMenu => self.voice_menu = !self.voice_menu,
            ServiceControl::Voice(id) => {
                self.snapshot.voice_id = id.clone();
                self.voice_menu = false;
                intents.push(ServicesIntent::Voice(id));
            }
            ServiceControl::Speak => intents.push(ServicesIntent::Speak),
            ServiceControl::OpenUrl(url) => intents.push(ServicesIntent::OpenUrl(url)),
            _ => {}
        }
    }
    pub(crate) fn field_value(&self, field: &ServiceField) -> String {
        match field {
            ServiceField::Credential { name, .. } => {
                self.credentials.get(name).cloned().unwrap_or_default()
            }
            ServiceField::Custom(field) => match &self.snapshot.form {
                Some(ServiceForm::Custom { draft, prompts }) => match field {
                    ServiceCustomField::Name => draft.name.clone(),
                    ServiceCustomField::Endpoint => draft.endpoint.clone(),
                    ServiceCustomField::ApiKey => draft.api_key.clone(),
                    ServiceCustomField::Model => draft.model.clone(),
                    ServiceCustomField::PromptName => prompts.draft.name.clone(),
                    ServiceCustomField::PromptDescription => prompts.draft.description.clone(),
                    ServiceCustomField::PromptText => prompts.draft.system_prompt.clone(),
                },
                _ => String::new(),
            },
        }
    }
    fn begin_edit(&mut self, field: ServiceField) {
        if self
            .editing
            .as_ref()
            .is_some_and(|editor| editor.field == field)
        {
            return;
        }
        let text = self.field_value(&field);
        let multiline = field.multiline();
        let layout = self
            .input_layouts
            .iter()
            .find(|(input, _, _)| *input == field);
        self.editing = Some(Editor {
            field,
            buffer: if multiline {
                TextEdit::multiline(text)
            } else {
                TextEdit::new(text)
            },
            rect: layout.map(|(_, rect, _)| *rect).unwrap_or_default(),
            layout: layout
                .map(|(_, _, layout)| layout.clone())
                .unwrap_or_default(),
            scroll: Point::default(),
        });
    }
    fn blur_unless(&mut self, control: &ServiceControl) {
        if self
            .editing
            .as_ref()
            .is_some_and(|editor| *control != ServiceControl::Field(editor.field.clone()))
        {
            self.editing = None;
        }
    }
    fn text_changed(&mut self, intents: &mut Vec<ServicesIntent>) {
        let Some(editor) = &self.editing else {
            return;
        };
        let field = editor.field.clone();
        let value = editor.buffer.text().to_owned();
        if self.field_value(&field) == value {
            return;
        }
        let revision = self.snapshot.form_revision;
        match field {
            ServiceField::Credential { name, .. } => {
                self.credentials.insert(name, value);
            }
            ServiceField::Custom(field) => {
                if let Some(ServiceForm::Custom { draft, prompts }) = &mut self.snapshot.form {
                    match field {
                        ServiceCustomField::Name => draft.name = value.clone(),
                        ServiceCustomField::Endpoint => {
                            draft.endpoint = value.clone();
                            self.snapshot.models.clear();
                            self.snapshot.model_error = None;
                            self.snapshot.test_status = None;
                        }
                        ServiceCustomField::ApiKey => {
                            draft.api_key = value.clone();
                            self.snapshot.models.clear();
                            self.snapshot.model_error = None;
                            self.snapshot.test_status = None;
                        }
                        ServiceCustomField::Model => {
                            draft.model = value.clone();
                            self.snapshot.test_status = None;
                        }
                        ServiceCustomField::PromptName => prompts.draft.name = value.clone(),
                        ServiceCustomField::PromptDescription => {
                            prompts.draft.description = value.clone()
                        }
                        ServiceCustomField::PromptText => {
                            prompts.draft.system_prompt = value.clone()
                        }
                    };
                    if matches!(
                        field,
                        ServiceCustomField::PromptName
                            | ServiceCustomField::PromptDescription
                            | ServiceCustomField::PromptText
                    ) {
                        intents.push(ServicesIntent::EditPrompt {
                            revision,
                            draft: prompts.draft.clone(),
                        });
                    } else {
                        intents.push(ServicesIntent::EditCustom {
                            revision,
                            field,
                            value,
                        });
                    }
                }
            }
        }
    }
    fn key(&mut self, key: Key, m: Modifiers, intents: &mut Vec<ServicesIntent>) {
        if self.snapshot.busy {
            return;
        }
        if self
            .editing
            .as_ref()
            .is_some_and(|editor| editor.buffer.is_composing())
        {
            return;
        }
        if key == Key::Tab {
            self.editing = None;
            self.move_focus(m.shift);
            return;
        }
        if let Some(editor) = self.editing.as_mut() {
            let primary = if self.snapshot.shell.key_style == KeyStyle::Mac {
                m.meta
            } else {
                m.control
            };
            let word = if self.snapshot.shell.key_style == KeyStyle::Mac {
                m.alt
            } else {
                m.control
            };
            if word && matches!(key, Key::ArrowLeft | Key::ArrowRight) {
                editor.buffer.move_caret(
                    if key == Key::ArrowLeft {
                        Movement::WordLeft
                    } else {
                        Movement::WordRight
                    },
                    m.shift,
                );
                return;
            }
            if primary {
                match key {
                    Key::Character('a' | 'A') => editor.buffer.select_all(),
                    Key::Character('c' | 'C') => intents.push(ServicesIntent::CopyText(
                        editor.buffer.selected_text().into(),
                    )),
                    Key::Character('x' | 'X') => {
                        intents.push(ServicesIntent::CopyText(
                            editor.buffer.selected_text().into(),
                        ));
                        editor.buffer.delete(true);
                    }
                    Key::Character('v' | 'V') => intents.push(ServicesIntent::ReadClipboard),
                    _ => {}
                }
            } else {
                match key {
                    Key::ArrowLeft => {
                        editor.buffer.move_caret(Movement::Left, m.shift);
                    }
                    Key::ArrowRight => {
                        editor.buffer.move_caret(Movement::Right, m.shift);
                    }
                    Key::Home => {
                        editor.buffer.move_caret(Movement::Home, m.shift);
                    }
                    Key::End => {
                        editor.buffer.move_caret(Movement::End, m.shift);
                    }
                    Key::Backspace => {
                        editor.buffer.delete(true);
                    }
                    Key::Delete => {
                        editor.buffer.delete(false);
                    }
                    Key::Enter if editor.field.multiline() => {
                        editor.buffer.insert("\n");
                    }
                    Key::Enter | Key::Escape => {
                        self.editing = None;
                        return;
                    }
                    Key::ArrowUp | Key::ArrowDown if editor.field.multiline() => {
                        if let Some(caret) = editor.buffer.display_caret().and_then(|byte| {
                            editor
                                .layout
                                .caret_stops
                                .iter()
                                .find(|(index, _)| *index == byte)
                                .map(|(_, point)| *point)
                        }) {
                            let direction = if key == Key::ArrowUp { -1. } else { 1. };
                            if let Some((byte, _)) = editor
                                .layout
                                .caret_stops
                                .iter()
                                .filter(|(_, point)| (point.y - caret.y) * direction > 1.)
                                .min_by(|(_, a), (_, b)| {
                                    ((a.y - caret.y).abs() * 1000. + (a.x - caret.x).abs())
                                        .total_cmp(
                                            &((b.y - caret.y).abs() * 1000.
                                                + (b.x - caret.x).abs()),
                                        )
                                })
                            {
                                editor.buffer.move_to(*byte, m.shift);
                            }
                        }
                    }
                    _ => {}
                }
            }
            self.text_changed(intents);
            return;
        }
        match key {
            Key::Escape => {
                self.drag = None;
                self.voice_menu = false;
                if self.snapshot.form.is_some() && !self.snapshot.busy {
                    intents.push(ServicesIntent::CancelForm);
                }
            }
            Key::Enter | Key::Space => {
                if let Some(control) = self.focus.clone() {
                    self.activate(control, intents);
                }
            }
            Key::ArrowUp | Key::ArrowDown | Key::ArrowLeft | Key::ArrowRight
                if self.focus == Some(ServiceControl::Rate) =>
            {
                self.snapshot.rate = if matches!(key, Key::ArrowUp | Key::ArrowRight) {
                    (self.snapshot.rate + 10).min(400)
                } else {
                    self.snapshot.rate.saturating_sub(10).max(80)
                };
                intents.push(ServicesIntent::Rate(self.snapshot.rate));
            }
            Key::PageDown => {
                self.scroll = (self.scroll + self.viewport.height * 0.8).min(self.scroll_limit)
            }
            Key::PageUp => self.scroll = (self.scroll - self.viewport.height * 0.8).max(0.),
            _ => {}
        }
    }
    fn move_focus(&mut self, backward: bool) {
        let controls = self
            .frame
            .as_ref()
            .map(|frame| {
                frame
                    .controls
                    .iter()
                    .filter(|control| control.enabled)
                    .map(|control| control.target.control.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if controls.is_empty() {
            return;
        }
        let index = self
            .focus
            .as_ref()
            .and_then(|focus| controls.iter().position(|control| control == focus));
        let next = if backward {
            index
                .unwrap_or(0)
                .checked_sub(1)
                .unwrap_or(controls.len() - 1)
        } else {
            index.map_or(0, |index| (index + 1) % controls.len())
        };
        let control = controls[next].clone();
        self.focus = Some(control.clone());
        self.ensure_visible(&control);
        if let ServiceControl::Field(field) = control {
            self.begin_edit(field);
        }
    }
    fn ensure_visible(&mut self, control: &ServiceControl) {
        let Some(rect) = self
            .frame
            .as_ref()
            .and_then(|frame| {
                frame
                    .controls
                    .iter()
                    .find(|item| item.target.control == *control)
            })
            .map(|item| item.rect)
        else {
            return;
        };
        if matches!(control, ServiceControl::Shell(_) | ServiceControl::Route(_)) {
            return;
        }
        let top = rect.y / self.scale;
        let bottom = (rect.y + rect.height) / self.scale;
        if matches!(control, ServiceControl::Voice(_)) && self.voice_menu {
            if top < self.voice_view.y {
                self.voice_scroll = (self.voice_scroll + top - self.voice_view.y - 4.).max(0.);
            } else if bottom > self.voice_view.y + self.voice_view.height {
                self.voice_scroll =
                    (self.voice_scroll + bottom - self.voice_view.y - self.voice_view.height + 4.)
                        .min(self.voice_limit);
            }
            return;
        }
        if top < self.viewport.y {
            self.scroll = (self.scroll + top - self.viewport.y - 8.).max(0.);
        } else if bottom > self.viewport.y + self.viewport.height {
            self.scroll = (self.scroll + bottom - self.viewport.y - self.viewport.height + 8.)
                .min(self.scroll_limit);
        }
    }
    fn drag_rate(&mut self, point: Point, intents: &mut Vec<ServicesIntent>) {
        let Some(rect) = self
            .frame
            .as_ref()
            .and_then(|frame| {
                frame
                    .controls
                    .iter()
                    .find(|control| control.target.control == ServiceControl::Rate)
            })
            .map(|control| control.rect)
        else {
            return;
        };
        let fraction = ((point.x - rect.x) / rect.width.max(1.)).clamp(0., 1.);
        let value = (80. + (fraction * 320. / 10.).round() * 10.) as u32;
        if self.snapshot.rate != value {
            self.snapshot.rate = value;
            intents.push(ServicesIntent::Rate(value));
        }
    }
    fn text_pointer(&mut self, point: Point, extend: bool) {
        let Some(editor) = self.editing.as_mut() else {
            return;
        };
        let point = Point {
            x: point.x / self.scale - editor.rect.x + editor.scroll.x,
            y: point.y / self.scale - editor.rect.y + editor.scroll.y,
        };
        if let Some((byte, _)) = editor.layout.caret_stops.iter().min_by(|(_, a), (_, b)| {
            ((a.y - point.y).abs() * 1000. + (a.x - point.x).abs())
                .total_cmp(&((b.y - point.y).abs() * 1000. + (b.x - point.x).abs()))
        }) {
            editor.buffer.move_to(*byte, extend);
        }
    }
}
