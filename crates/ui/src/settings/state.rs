use super::*;

#[derive(Clone, Copy)]
pub(super) struct Menu {
    pub(super) field: Field,
    pub(super) selected: usize,
    pub(super) scroll: f32,
}
pub(super) struct Recording {
    pub(super) index: usize,
    pub(super) pending: Option<(Key, String)>,
}
pub struct SettingsUi {
    pub(super) snapshot: Snapshot,
    pub(super) general_permission_help: bool,
    pub(super) general_release_notes: bool,
    pub(super) page: Page,
    pub(super) dragging: Option<Field>,
    pub(super) embedded: bool,
    pub(super) editing: Option<Editing>,
    pub(super) local_recording: Option<ScreenshotField>,
    pub(super) local_pending: Option<(Key, Modifiers)>,
    pub(super) size: Size,
    pub(super) scale: f32,
    pub(super) scroll: f32,
    pub(super) section: Section,
    pub(super) focus: Option<Target>,
    pub(super) keyboard_focus: bool,
    pub(super) hover: Option<Target>,
    pub(super) menu: Option<Menu>,
    pub(super) recording: Option<Recording>,
    pub(super) frame: SettingsFrame,
    pub(super) focus_order: Vec<(Target, Rect)>,
    pub(super) offsets: [f32; 6],
    pub(super) max_scroll: f32,
}
impl SettingsUi {
    pub fn new(snapshot: Snapshot) -> Self {
        let size = Size {
            width: 900.,
            height: 650.,
        };
        let mut ui = Self {
            snapshot,
            general_permission_help: false,
            general_release_notes: false,
            page: Page::Ocr,
            dragging: None,
            embedded: false,
            editing: None,
            local_recording: None,
            local_pending: None,
            size,
            scale: 1.,
            scroll: 0.,
            section: Section::Hotkeys,
            focus: None,
            keyboard_focus: false,
            hover: None,
            menu: None,
            recording: None,
            frame: SettingsFrame {
                scene: crate::Scene {
                    size,
                    commands: vec![],
                },
                content: Rect::default(),
                ime_cursor: None,
                hits: vec![],
                geometry: vec![],
                content_height: 0.,
                viewport: Rect::default(),
            },
            focus_order: vec![],
            offsets: [0.; 6],
            max_scroll: 0.,
        };
        ui.rebuild();
        ui
    }
    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }
    /// The active field's committed text, selection and separate IME draft.
    pub fn text_edit(&self) -> Option<&TextEdit> {
        self.editing.as_ref().map(|edit| &edit.buffer)
    }
    pub fn focused(&self) -> Option<Target> {
        self.focus
    }
    pub fn page(&self) -> Page {
        self.page
    }
    pub fn set_embedded(&mut self, embedded: bool) {
        self.embedded = embedded;
        self.rebuild();
    }
    pub fn active_section(&self) -> Section {
        self.section
    }
    pub fn recording_index(&self) -> Option<usize> {
        self.recording.as_ref().map(|r| r.index)
    }
    pub(super) fn sections(&self) -> &'static [Section] {
        Section::for_page(self.page)
    }
    pub(super) fn hotkeys(&self) -> &[Hotkey] {
        if self.page == Page::Screenshot {
            &self.snapshot.screenshot_hotkeys
        } else if self.page == Page::Translation {
            &self.snapshot.translation_hotkeys
        } else {
            &self.snapshot.hotkeys
        }
    }
    pub fn frame_with_text(
        &mut self,
        size: Size,
        scale: f32,
        measurer: &mut impl TextMeasurer,
    ) -> SettingsFrame {
        if let Some(edit) = self.editing.as_mut() {
            let display = edit.buffer.display_text();
            let measured =
                measurer.measure(&display, edit.field.text_size(), 400, FontFamily::Sans);
            let caret = edit.buffer.display_caret().and_then(|byte| {
                measured
                    .caret_stops
                    .iter()
                    .find(|(index, _)| *index == byte)
                    .map(|(_, x)| *x)
            });
            if let Some(x) = caret {
                let width = self
                    .frame
                    .geometry
                    .iter()
                    .find(|n| n.kind == GeometryKind::Control(edit.field.target()))
                    .map_or(230., |n| n.rect.width)
                    - 24.;
                if x < edit.scroll {
                    edit.scroll = x.max(0.);
                }
                if x > edit.scroll + width {
                    edit.scroll = (x - width).max(0.);
                }
            }
            edit.measured = Some(measured);
        }
        self.frame(size, scale)
    }
    /// `size` is window DIP, `interface_scale` is the user's UI zoom. Device
    /// pixel ratio belongs exclusively to the renderer and window adapter.
    pub fn frame(&mut self, size: Size, interface_scale: f32) -> SettingsFrame {
        self.size = Size {
            width: size.width.max(1.),
            height: size.height.max(1.),
        };
        self.scale = if interface_scale.is_finite() {
            interface_scale.clamp(0.5, 3.)
        } else {
            1.
        };
        self.rebuild();
        let mut frame = self.frame.clone().scaled(self.scale);
        frame.scene.size = self.size;
        frame
    }

    pub fn update(&mut self, event: Event) -> Vec<Action> {
        let mut actions = vec![];
        match event {
            Event::TextSelection(selection) => {
                if let Some(edit) = self.editing.as_mut() {
                    edit.buffer.set_selection(selection);
                }
            }
            Event::SetValue(target, value) => {
                if self.focus_order.iter().any(|(t, _)| *t == target) {
                    if let Some(field) = Field::from_target(target) {
                        if let Some(action) = field.edit(&mut self.snapshot, value) {
                            if self
                                .editing
                                .as_ref()
                                .is_some_and(|edit| edit.field == field)
                            {
                                self.editing =
                                    Some(Editing::new(field, field.value(&self.snapshot)));
                            }
                            actions.push(action);
                        }
                    }
                }
            }
            Event::Focus(target) => {
                if self
                    .editing
                    .as_ref()
                    .is_some_and(|edit| !edit.owns_target(target))
                {
                    self.finish_text_edit(&mut actions);
                }
                self.focus_target(target);
            }
            Event::Activate(target) => {
                if self
                    .editing
                    .as_ref()
                    .is_some_and(|edit| !edit.owns_target(target))
                {
                    self.finish_text_edit(&mut actions);
                }
                if self.focus_target(target) {
                    self.cancel_recording(&mut actions);
                    if !matches!(
                        target,
                        Target::Option(..)
                            | Target::TranslationOption(..)
                            | Target::ScreenshotOption(..)
                            | Target::GeneralOption(..)
                    ) {
                        self.menu = None;
                    }
                    self.activate(target, &mut actions);
                }
            }
            Event::PointerUp(point) => {
                if let Some(field) = self.dragging.take() {
                    self.drag_range(field, self.logical(point), &mut actions);
                }
            }
            Event::TextInput(text) | Event::ClipboardText(text) => {
                if let Some(edit) = self.editing.as_mut() {
                    edit.buffer.insert(&text);
                    self.text_changed(&mut actions);
                }
            }
            Event::ImePreedit { text, selection } => {
                if let Some(edit) = self.editing.as_mut() {
                    edit.buffer.preedit(&text, selection);
                }
            }
            Event::ImeCommit(text) => {
                if let Some(edit) = self.editing.as_mut() {
                    edit.buffer.commit(&text);
                    self.text_changed(&mut actions);
                }
            }
            Event::ImeDisabled => {
                if let Some(edit) = self.editing.as_mut() {
                    edit.buffer.clear_preedit();
                }
            }
            Event::Snapshot(snapshot) => {
                self.snapshot = snapshot;
            }
            Event::Resize(size) => {
                self.size = size;
                self.dragging = None;
                self.menu = None;
            }
            Event::FocusLost => {
                self.finish_text_edit(&mut actions);
                self.cancel_recording(&mut actions);
                self.menu = None;
                self.focus = None;
                self.dragging = None;
            }
            Event::PointerMove(point) => {
                self.hover = self.frame.hit_test(self.logical(point));
                if let Some(field) = self.dragging {
                    self.drag_range(field, self.logical(point), &mut actions);
                }
            }
            Event::PointerDown(point) => {
                let logical = self.logical(point);
                let next = self.frame.hit_test(logical);
                if self
                    .editing
                    .as_ref()
                    .is_some_and(|e| !next.is_some_and(|target| e.owns_target(target)))
                {
                    self.finish_text_edit(&mut actions);
                }
                self.keyboard_focus = false;
                let hit = self.frame.hit_test(self.logical(point));
                let close_menu = hit
                    .and_then(Field::from_target)
                    .is_some_and(|field| self.menu.is_some_and(|menu| menu.field == field));
                if !matches!(
                    hit,
                    Some(
                        Target::Hotkey(_)
                            | Target::Screenshot(
                                ScreenshotField::AnnotationShortcut(_)
                                    | ScreenshotField::PinBorderShortcut
                            )
                    )
                ) {
                    self.cancel_recording(&mut actions);
                }
                if !matches!(
                    hit,
                    Some(
                        Target::Option(..)
                            | Target::TranslationOption(..)
                            | Target::ScreenshotOption(..)
                            | Target::GeneralOption(..)
                    )
                ) {
                    self.menu = None;
                }
                self.focus = hit;
                if let Some(target) = hit.filter(|_| !close_menu) {
                    self.activate(target, &mut actions);
                    if self.editing.is_some() {
                        self.text_pointer(logical);
                    }
                    if let Some(field) = Field::from_target(target)
                        .filter(|f| f.range().is_some() && !f.text_input())
                    {
                        self.dragging = Some(field);
                        self.drag_range(field, self.logical(point), &mut actions);
                    }
                }
            }
            Event::Scroll { position, delta_y } => {
                if delta_y.is_finite() {
                    let point = self.logical(position);
                    if let Some(menu) = self.menu.as_mut() {
                        if self.frame.hits.iter().any(|h| {
                            matches!(
                                h.target,
                                Target::Option(..)
                                    | Target::TranslationOption(..)
                                    | Target::ScreenshotOption(..)
                                    | Target::GeneralOption(..)
                            ) && h.rect.contains(point)
                        }) {
                            menu.scroll = (menu.scroll + delta_y / self.scale).max(0.);
                        } else {
                            self.menu = None;
                            self.scroll =
                                (self.scroll + delta_y / self.scale).clamp(0., self.max_scroll);
                        }
                    } else if point.x >= self.frame.viewport.x && point.y >= self.page_top() {
                        self.scroll =
                            (self.scroll + delta_y / self.scale).clamp(0., self.max_scroll);
                    }
                }
            }
            Event::RecordedShortcut(shortcut) => {
                if self.local_recording.is_some() {
                    self.snapshot.error =
                        Some("与全局快捷键重复，请先修改对应的全局快捷键。".into());
                } else if let Some(index) = self.recording.as_ref().map(|r| r.index) {
                    self.cancel_recording(&mut actions);
                    actions.push(Action::SaveHotkey { index, shortcut });
                }
            }
            Event::KeyUp { key } => {
                if let Some(field) = self.local_recording {
                    if self
                        .local_pending
                        .as_ref()
                        .is_some_and(|(pending, _)| *pending == key)
                    {
                        let (_, modifiers) = self.local_pending.take().unwrap();
                        self.save_local_shortcut(field, key, modifiers, false, &mut actions);
                    }
                }
                if let Some((index, shortcut)) = self.recording.as_mut().and_then(|r| {
                    if r.pending
                        .as_ref()
                        .is_some_and(|(pending, _)| *pending == key)
                    {
                        r.pending.take().map(|(_, chord)| (r.index, chord))
                    } else {
                        None
                    }
                }) {
                    self.cancel_recording(&mut actions);
                    actions.push(Action::SaveHotkey { index, shortcut });
                }
            }
            Event::KeyDown {
                key,
                modifiers,
                repeat,
            } => {
                self.keyboard_focus = true;
                if self.text_key(key, modifiers, &mut actions) {
                    self.rebuild();
                    return actions;
                }
                if key == Key::Escape {
                    self.cancel_recording(&mut actions);
                    self.menu = None;
                } else if key == Key::Tab {
                    self.cancel_recording(&mut actions);
                    self.menu = None;
                    self.move_focus(if modifiers.shift { -1 } else { 1 });
                } else if self.local_recording.is_some() {
                    if !repeat {
                        self.local_pending = Some((key, modifiers));
                    }
                } else if let Some(recording) = self.recording.as_mut() {
                    if matches!(key, Key::Delete | Key::Backspace) {
                        let index = recording.index;
                        self.cancel_recording(&mut actions);
                        actions.push(Action::SaveHotkey {
                            index,
                            shortcut: "未设置".into(),
                        });
                    } else if !repeat {
                        recording.pending = chord(key, modifiers, self.snapshot.key_style)
                            .map(|chord| (key, chord));
                    }
                } else if let Some(menu) = self.menu.as_mut() {
                    let count = menu.field.choices().len();
                    match key {
                        Key::ArrowDown => menu.selected = (menu.selected + 1) % count,
                        Key::ArrowUp => menu.selected = (menu.selected + count - 1) % count,
                        Key::Home => menu.selected = 0,
                        Key::End => menu.selected = count - 1,
                        Key::Enter | Key::Space => {
                            let menu = *menu;
                            self.activate(menu.field.option(menu.selected), &mut actions);
                        }
                        _ => {}
                    }
                    if let Some(menu) = self.menu.as_mut() {
                        let height = (menu.field.choices().len() as f32 * 34. + 8.)
                            .min((self.size.height / self.scale - 16.).max(0.));
                        let option_top = menu.selected as f32 * 34.;
                        menu.scroll = menu
                            .scroll
                            .min(option_top)
                            .max((option_top + 42. - height).max(0.));
                    }
                } else {
                    match key {
                        Key::Enter | Key::Space => {
                            if let Some(target) = self.focus {
                                self.activate(target, &mut actions);
                            }
                        }
                        Key::ArrowDown | Key::ArrowUp | Key::ArrowLeft | Key::ArrowRight => {
                            if let Some(field) = self.focus.and_then(Field::from_target) {
                                if let Some((min, max, step, _)) = field.range() {
                                    if let Value::Number(value) = field.value(&self.snapshot) {
                                        let value =
                                            if matches!(key, Key::ArrowDown | Key::ArrowLeft) {
                                                value.saturating_sub(step).max(min)
                                            } else {
                                                (value + step).min(max)
                                            };
                                        if let Some(action) =
                                            field.edit(&mut self.snapshot, Value::Number(value))
                                        {
                                            actions.push(action);
                                        }
                                    }
                                } else if !field.choices().is_empty() {
                                    self.activate(field.target(), &mut actions);
                                } else {
                                    self.move_focus(
                                        if matches!(key, Key::ArrowDown | Key::ArrowRight) {
                                            1
                                        } else {
                                            -1
                                        },
                                    );
                                }
                            } else {
                                self.move_focus(
                                    if matches!(key, Key::ArrowDown | Key::ArrowRight) {
                                        1
                                    } else {
                                        -1
                                    },
                                );
                            }
                        }
                        Key::Home | Key::End => {
                            if let Some(field) = self
                                .focus
                                .and_then(Field::from_target)
                                .filter(|f| f.range().is_some())
                            {
                                let (min, max, _, _) = field.range().unwrap();
                                if let Some(action) = field.edit(
                                    &mut self.snapshot,
                                    Value::Number(if key == Key::Home { min } else { max }),
                                ) {
                                    actions.push(action);
                                }
                            } else if key == Key::Home {
                                self.scroll = 0.;
                            } else {
                                self.scroll = self.max_scroll;
                            }
                        }
                        Key::PageDown => {
                            self.scroll = (self.scroll + self.frame.viewport.height * 0.8)
                                .min(self.max_scroll)
                        }
                        Key::PageUp => {
                            self.scroll = (self.scroll - self.frame.viewport.height * 0.8).max(0.)
                        }
                        _ => {}
                    }
                }
            }
        }
        self.rebuild();
        actions
    }
    pub(super) fn logical(&self, p: Point) -> Point {
        Point {
            x: p.x / self.scale,
            y: p.y / self.scale,
        }
    }
    pub(super) fn page_top(&self) -> f32 {
        if self.snapshot.theme == Theme::Paper {
            68.
        } else {
            0.
        }
    }
    pub(super) fn cancel_recording(&mut self, actions: &mut Vec<Action>) {
        self.local_pending = None;
        if self.recording.take().is_some() | self.local_recording.take().is_some() {
            actions.push(Action::EndHotkeyRecording);
        }
    }
    pub(super) fn activate(&mut self, target: Target, actions: &mut Vec<Action>) {
        match target {
            Target::Navigate(page) => {
                self.finish_text_edit(actions);
                self.cancel_recording(actions);
                self.dragging = None;
                self.menu = None;
                if matches!(
                    page,
                    Page::General | Page::Ocr | Page::Translation | Page::Screenshot
                ) {
                    self.page = page;
                    self.scroll = 0.;
                    self.section = Section::Hotkeys;
                    self.focus = Some(target);
                } else {
                    actions.push(Action::Navigate(page));
                }
            }
            Target::Section(section) => {
                self.menu = None;
                self.section = section;
                self.scroll = (self.offsets[self
                    .sections()
                    .iter()
                    .position(|s| *s == section)
                    .unwrap_or(0)]
                    - self.header_height()
                    - 16.)
                    .clamp(0., self.max_scroll);
                if self.page == Page::General {
                    self.scroll = self.scroll.floor();
                }
            }
            Target::General(_)
            | Target::Ocr(_)
            | Target::Translation(_)
            | Target::Screenshot(_) => {
                let field = Field::from_target(target).unwrap();
                if matches!(field, Field::General(_)) && field.text_input() {
                    self.begin_field_edit(field);
                    return;
                }
                if let Field::Screenshot(screenshot) = field {
                    if field.text_input() {
                        self.begin_text_edit(screenshot);
                        return;
                    }
                    if field.local_shortcut() {
                        self.cancel_recording(actions);
                        self.local_recording = Some(screenshot);
                        actions.push(Action::BeginLocalShortcutRecording { field: screenshot });
                        return;
                    }
                    if screenshot == ScreenshotField::ResetAnnotationShortcuts {
                        if let Some(action) = field.edit(&mut self.snapshot, Value::Toggle(true)) {
                            actions.push(action);
                        }
                        return;
                    }
                }
                match field.value(&self.snapshot) {
                    Value::Toggle(value) => {
                        if let Some(action) = field.edit(&mut self.snapshot, Value::Toggle(!value))
                        {
                            actions.push(action);
                        }
                    }
                    Value::Text(value) => {
                        let selected = field
                            .choices()
                            .iter()
                            .position(|(id, _)| *id == value)
                            .unwrap_or(0);
                        self.menu = Some(Menu {
                            field,
                            selected,
                            scroll: 0.,
                        });
                    }
                    Value::Number(_) => {}
                }
            }
            Target::GeneralOption(field, index) => {
                self.select_option(Field::General(field), index, actions)
            }
            Target::GeneralTheme(index) => {
                if let Some((id, _)) = GeneralField::InterfaceTheme.choices().get(index) {
                    if let Some(action) = Field::General(GeneralField::InterfaceTheme)
                        .edit(&mut self.snapshot, Value::Text((*id).into()))
                    {
                        actions.push(action);
                    }
                }
            }
            Target::GeneralCommand(command) => actions.push(Action::GeneralCommand(command)),
            Target::GeneralUnavailable => {}
            Target::GeneralDisclosure(release_notes) => {
                if release_notes {
                    self.general_release_notes = !self.general_release_notes;
                } else {
                    self.general_permission_help = !self.general_permission_help;
                }
            }
            Target::GeneralStep(field, increase) => {
                let field = Field::General(field);
                if let (Value::Number(value), Some((min, max, step, _))) =
                    (field.value(&self.snapshot), field.range())
                {
                    let value = if increase {
                        value.saturating_add(step).min(max)
                    } else {
                        value.saturating_sub(step).max(min)
                    };
                    if let Some(action) = field.edit(&mut self.snapshot, Value::Number(value)) {
                        actions.push(action);
                    }
                    self.editing = None;
                }
            }
            Target::ScreenshotStep(increase) => self.step_longshot_height(increase),
            Target::ScreenshotOption(field, index) => {
                self.select_option(Field::Screenshot(field), index, actions)
            }
            Target::ScreenshotColor(field, index) => {
                if let Some(color) = field.colors().get(index) {
                    if let Some(action) = Field::Screenshot(field).edit(
                        &mut self.snapshot,
                        Value::Number(u32::from_be_bytes(*color)),
                    ) {
                        actions.push(action);
                    }
                }
            }
            Target::BrowseScreenshotDirectory => actions.push(Action::SelectScreenshotDirectory),
            Target::PickScreenshotColor(field) => {
                if let Value::Number(value) = field.value(&self.snapshot.screenshot) {
                    actions.push(Action::SelectScreenshotColor {
                        field,
                        rgba: value.to_be_bytes(),
                    });
                }
            }
            Target::Option(field, index) => self.select_option(Field::Ocr(field), index, actions),
            Target::TranslationOption(field, index) => {
                self.select_option(Field::Translation(field), index, actions)
            }
            Target::Hotkey(index) => {
                let same = self.recording.as_ref().is_some_and(|r| r.index == index);
                self.cancel_recording(actions);
                if !same {
                    self.recording = Some(Recording {
                        index,
                        pending: None,
                    });
                    actions.push(Action::BeginHotkeyRecording { index });
                }
            }
            Target::ResetHotkey(index) => actions.push(Action::ResetHotkey { index }),
            Target::ClearHotkey(index) => actions.push(Action::SaveHotkey {
                index,
                shortcut: "未设置".into(),
            }),
            Target::ResetHotkeys => actions.push(Action::ResetHotkeys),
            Target::CheckConflicts => actions.push(Action::CheckHotkeyConflicts),
            Target::Capacity => actions.push(Action::ConfigureCapacity),
        }
    }
    pub(super) fn select_option(&mut self, field: Field, index: usize, actions: &mut Vec<Action>) {
        if let Some((id, _)) = field.choices().get(index) {
            if let Some(action) = field.edit(&mut self.snapshot, Value::Text((*id).into())) {
                actions.push(action);
            }
            self.menu = None;
            self.focus = Some(field.target());
        }
    }
    pub(super) fn focus_target(&mut self, target: Target) -> bool {
        if self.embedded && matches!(target, Target::Navigate(_) | Target::Capacity) {
            return false;
        }
        let Some((_, rect)) = self.focus_order.iter().find(|(t, _)| *t == target).copied() else {
            return false;
        };
        self.keyboard_focus = true;
        self.focus = Some(target);
        if let Some(menu) = self.menu.as_mut() {
            let option = match target {
                Target::GeneralOption(field, index) => Some((Field::General(field), index)),
                Target::ScreenshotOption(field, index) => Some((Field::Screenshot(field), index)),
                Target::Option(field, index) => Some((Field::Ocr(field), index)),
                Target::TranslationOption(field, index) => Some((Field::Translation(field), index)),
                _ => None,
            };
            if let Some((field, index)) = option.filter(|(field, _)| *field == menu.field) {
                let height = (field.choices().len() as f32 * 34. + 8.)
                    .min((self.size.height / self.scale - 16.).max(0.));
                menu.selected = index;
                let top = index as f32 * 34.;
                menu.scroll = menu.scroll.min(top).max((top + 42. - height).max(0.));
            }
        }
        self.ensure_visible(target, rect);
        if let Some(field) = Field::from_target(target).filter(|f| f.text_input()) {
            self.begin_field_edit(field);
        }
        true
    }
    pub(super) fn ensure_visible(&mut self, target: Target, rect: Rect) {
        if matches!(
            target,
            Target::General(_)
                | Target::GeneralCommand(_)
                | Target::GeneralTheme(_)
                | Target::GeneralStep(..)
                | Target::GeneralDisclosure(_)
                | Target::Ocr(_)
                | Target::Translation(_)
                | Target::Screenshot(_)
                | Target::ScreenshotStep(_)
                | Target::Hotkey(_)
                | Target::ClearHotkey(_)
                | Target::ResetHotkey(_)
        ) {
            let view = self.frame.viewport;
            if rect.y < view.y {
                self.scroll = (self.scroll + rect.y - view.y - 12.).max(0.);
            }
            if rect.y + rect.height > view.y + view.height {
                self.scroll = (self.scroll + rect.y + rect.height - view.y - view.height + 12.)
                    .min(self.max_scroll);
            }
        }
    }
    pub(super) fn drag_range(&mut self, field: Field, point: Point, actions: &mut Vec<Action>) {
        let Some((min, max, step, _)) = field.range() else {
            return;
        };
        let Some(node) = self
            .frame
            .geometry
            .iter()
            .find(|g| g.kind == GeometryKind::Control(field.target()))
        else {
            return;
        };
        let ratio = ((point.x - node.rect.x - 9.) / (node.rect.width - 18.)).clamp(0., 1.);
        let value = (min as f32 + ((max - min) as f32 * ratio / step as f32).round() * step as f32)
            .clamp(min as f32, max as f32) as u32;
        if let Some(action) = field.edit(&mut self.snapshot, Value::Number(value)) {
            actions.push(action);
        }
    }
    pub(super) fn move_focus(&mut self, direction: isize) {
        if self.focus_order.is_empty() {
            return;
        }
        let count = self.focus_order.len() as isize;
        let current = self
            .focus
            .and_then(|target| self.focus_order.iter().position(|(t, _)| *t == target))
            .map(|i| i as isize)
            .unwrap_or(if direction > 0 { -1 } else { 0 });
        let (target, rect) = self.focus_order[(current + direction).rem_euclid(count) as usize];
        self.focus = Some(target);
        self.ensure_visible(target, rect);
        if let Some(field) = Field::from_target(target).filter(|field| field.text_input()) {
            self.begin_field_edit(field);
        }
    }

    pub(super) fn header_height(&self) -> f32 {
        if self.snapshot.theme == Theme::Native {
            if self.size.width / self.scale <= 700. {
                104.
            } else {
                107.
            }
        } else {
            82.
        }
    }
    pub(super) fn rebuild(&mut self) {
        self.scroll = self.scroll.max(0.);
        let (frame, focus, offsets, max_scroll) = self.layout();
        self.focus_order = focus;
        if self.embedded {
            self.focus_order
                .retain(|(target, _)| !matches!(target, Target::Navigate(_) | Target::Capacity));
        }
        self.offsets = offsets;
        self.max_scroll = max_scroll;
        if self.scroll > max_scroll {
            self.scroll = max_scroll;
            let (frame, focus, _, _) = self.layout();
            self.frame = frame;
            self.focus_order = focus;
        } else {
            self.frame = frame;
        }
        let sections = self.sections();
        let general_order = self
            .focus_order
            .iter()
            .map(|(target, _)| *target)
            .collect::<Vec<_>>();
        self.focus_order.sort_by_key(|(target, _)| match target {
            Target::Navigate(page) => Page::ALL.iter().position(|p| p == page).unwrap_or(0),
            Target::Capacity => 7,
            Target::Section(section) => 8 + sections.iter().position(|s| s == section).unwrap_or(0),
            Target::ResetHotkeys => 13,
            Target::CheckConflicts => 14,
            Target::Hotkey(index) => 15 + index * 3,
            Target::ResetHotkey(index) => 16 + index * 3,
            Target::ClearHotkey(index) => 17 + index * 3,
            Target::General(field) => 30 + *field as usize,
            Target::GeneralOption(_, index) => 100 + index,
            Target::GeneralTheme(index) => 30 + index,
            Target::GeneralCommand(command) => 60 + *command as usize,
            Target::GeneralDisclosure(release_notes) => 90 + usize::from(*release_notes),
            Target::GeneralUnavailable => 100,
            Target::GeneralStep(field, increase) => 30 + *field as usize + usize::from(*increase),
            Target::Ocr(field) => 30 + OcrField::ALL.iter().position(|f| f == field).unwrap_or(0),
            Target::Option(_, index) | Target::TranslationOption(_, index) => 60 + index,
            Target::Screenshot(field) => 60 + crate::settings::screenshot_view::field_order(*field),
            Target::ScreenshotStep(increase) => 87 + usize::from(*increase),
            Target::ScreenshotOption(_, index) => 100 + index,
            Target::ScreenshotColor(field, index) => {
                60 + crate::settings::screenshot_view::field_order(*field) + index
            }
            Target::BrowseScreenshotDirectory => 65,
            Target::PickScreenshotColor(field) => {
                60 + crate::settings::screenshot_view::field_order(*field) + 10
            }
            Target::Translation(field) => {
                40 + TranslationField::ALL
                    .iter()
                    .position(|f| f == field)
                    .unwrap_or(0)
            }
        });
        if self.page == Page::General {
            self.focus_order.sort_by_key(|(target, _)| match target {
                Target::Navigate(page) => Page::ALL.iter().position(|p| p == page).unwrap_or(0),
                Target::Capacity => 7,
                Target::Section(section) => {
                    8 + sections.iter().position(|s| s == section).unwrap_or(0)
                }
                _ => 20 + general_order.iter().position(|t| t == target).unwrap_or(0),
            });
        }
        self.section = self
            .sections()
            .iter()
            .enumerate()
            .filter(|(i, _)| self.offsets[*i] <= self.scroll + self.header_height() + 50.)
            .map(|(_, s)| *s)
            .next_back()
            .unwrap_or(self.sections()[0]);
    }
}
