//! One normalized window interface for fixed preferences and dynamic pages.
use crate::{library as l, services as s, *};

pub enum WindowSnapshot {
    Settings(Snapshot),
    Services(s::ServicesSnapshot),
    Library(l::LibrarySnapshot),
}
pub enum WindowIntent {
    Settings(Action),
    Services(s::ServicesIntent),
    Library(l::LibraryIntent),
}
pub enum WindowInput {
    Event(Event),
    Services(s::ServicesInput),
    Library(l::LibraryInput),
}
#[derive(Clone, Debug)]
pub enum WindowControls {
    Settings,
    Services(Vec<s::ServicesControlInfo>),
    Library(Vec<l::LibraryControl>),
}
#[derive(Clone, Debug)]
pub struct WindowFrame {
    pub page: SettingsFrame,
    pub controls: WindowControls,
}
pub enum WindowUi {
    Settings(SettingsUi),
    Services(s::ServicesUi),
    Library(l::LibraryUi),
}
impl WindowUi {
    pub fn input(&mut self, input: WindowInput) -> Vec<WindowIntent> {
        match input {
            WindowInput::Event(event) => self.update(event),
            WindowInput::Services(input) => self.services_input(input),
            WindowInput::Library(input) => self.library_input(input),
        }
    }
    pub fn new(snapshot: WindowSnapshot) -> Self {
        match snapshot {
            WindowSnapshot::Settings(snapshot) => Self::Settings(SettingsUi::new(snapshot)),
            WindowSnapshot::Services(snapshot) => Self::Services(s::ServicesUi::new(snapshot)),
            WindowSnapshot::Library(snapshot) => Self::Library(l::LibraryUi::new(snapshot)),
        }
    }
    pub fn page(&self) -> Page {
        match self {
            Self::Settings(ui) => ui.page(),
            Self::Services(_) => Page::Services,
            Self::Library(ui) => {
                if ui.snapshot().kind == l::LibraryKind::History {
                    Page::History
                } else {
                    Page::Favorites
                }
            }
        }
    }
    pub fn preferences(&self) -> Option<&SettingsUi> {
        if let Self::Settings(ui) = self {
            Some(ui)
        } else {
            None
        }
    }
    pub fn text_edit(&self) -> Option<&TextEdit> {
        match self {
            Self::Settings(ui) => ui.text_edit(),
            Self::Services(ui) => ui.text_edit(),
            Self::Library(ui) => ui.text_edit(),
        }
    }
    pub fn snapshot(&mut self, snapshot: WindowSnapshot) {
        match (self, snapshot) {
            (Self::Settings(ui), WindowSnapshot::Settings(snapshot)) => {
                ui.update(Event::Snapshot(snapshot));
            }
            (Self::Services(ui), WindowSnapshot::Services(snapshot)) => {
                ui.update(s::ServicesInput::Snapshot(snapshot));
            }
            (Self::Library(ui), WindowSnapshot::Library(snapshot))
                if ui.snapshot().kind == snapshot.kind =>
            {
                ui.update(l::LibraryInput::Snapshot(snapshot));
            }
            (ui, snapshot) => *ui = Self::new(snapshot),
        }
    }
    pub fn frame(
        &mut self,
        size: Size,
        scale: f32,
        metrics: &mut impl TextMeasurer,
    ) -> WindowFrame {
        match self {
            Self::Settings(ui) => WindowFrame {
                page: ui.frame_with_text(size, scale, metrics),
                controls: WindowControls::Settings,
            },
            Self::Services(ui) => {
                let frame = ui.frame(size, scale, metrics);
                WindowFrame {
                    page: frame.page,
                    controls: WindowControls::Services(frame.controls),
                }
            }
            Self::Library(ui) => {
                let frame = ui.frame(size, scale, metrics);
                WindowFrame {
                    page: frame.page,
                    controls: WindowControls::Library(frame.controls),
                }
            }
        }
    }
    pub fn services_input(&mut self, input: s::ServicesInput) -> Vec<WindowIntent> {
        if let Self::Services(ui) = self {
            ui.update(input)
                .into_iter()
                .map(WindowIntent::Services)
                .collect()
        } else {
            vec![]
        }
    }
    pub fn library_input(&mut self, input: l::LibraryInput) -> Vec<WindowIntent> {
        if let Self::Library(ui) = self {
            let shell = match &input {
                l::LibraryInput::PointerDown(point) => ui.frame_shell_hit(*point),
                _ => None,
            };
            if let Some(target) = shell {
                let mut intents = ui
                    .update(l::LibraryInput::FocusLost)
                    .into_iter()
                    .map(WindowIntent::Library)
                    .collect::<Vec<_>>();
                match target {
                    Target::Navigate(page) => {
                        intents.push(WindowIntent::Settings(Action::Navigate(page)))
                    }
                    Target::Capacity => {
                        intents.push(WindowIntent::Settings(Action::ConfigureCapacity))
                    }
                    _ => {}
                }
                return intents;
            }
            ui.update(input)
                .into_iter()
                .map(WindowIntent::Library)
                .collect()
        } else {
            vec![]
        }
    }
    /// Hosts keep native input normalization identical across all page kinds.
    /// Dynamic accessibility requests use the page's typed input methods above.
    pub fn update(&mut self, event: Event) -> Vec<WindowIntent> {
        if let Self::Settings(ui) = self {
            return ui
                .update(event)
                .into_iter()
                .map(WindowIntent::Settings)
                .collect();
        }
        match self {
            Self::Services(_) => {
                let input = match event {
                    Event::PointerDown(p) => s::ServicesInput::PointerDown(p),
                    Event::PointerMove(p) => s::ServicesInput::PointerMove(p),
                    Event::PointerUp(p) => s::ServicesInput::PointerUp(p),
                    Event::KeyDown { key, modifiers, .. } => {
                        s::ServicesInput::KeyDown { key, modifiers }
                    }
                    Event::Scroll { position, delta_y } => {
                        s::ServicesInput::Scroll { position, delta_y }
                    }
                    Event::TextInput(text) => s::ServicesInput::Text(text),
                    Event::ClipboardText(text) => s::ServicesInput::ClipboardText(text),
                    Event::TextSelection(range) => s::ServicesInput::Selection(range),
                    Event::ImePreedit { text, selection } => {
                        s::ServicesInput::ImePreedit { text, selection }
                    }
                    Event::ImeCommit(text) => s::ServicesInput::ImeCommit(text),
                    Event::ImeDisabled => s::ServicesInput::ImeDisabled,
                    Event::FocusLost => s::ServicesInput::FocusLost,
                    Event::Activate(Target::Navigate(page)) => {
                        return vec![WindowIntent::Settings(Action::Navigate(page))]
                    }
                    _ => return vec![],
                };
                self.services_input(input)
            }
            Self::Library(_) => {
                let input = match event {
                    Event::PointerDown(p) => l::LibraryInput::PointerDown(p),
                    Event::PointerMove(p) => l::LibraryInput::PointerMove(p),
                    Event::PointerUp(_) => l::LibraryInput::PointerUp,
                    Event::KeyDown { key, modifiers, .. } => {
                        l::LibraryInput::KeyDown { key, modifiers }
                    }
                    Event::Scroll { position, delta_y } => {
                        l::LibraryInput::Scroll { position, delta_y }
                    }
                    Event::TextInput(text) => l::LibraryInput::Text(text),
                    Event::ClipboardText(text) => l::LibraryInput::ClipboardText(text),
                    Event::TextSelection(range) => l::LibraryInput::Selection(range),
                    Event::ImePreedit { text, selection } => {
                        l::LibraryInput::ImePreedit { text, selection }
                    }
                    Event::ImeCommit(text) => l::LibraryInput::ImeCommit(text),
                    Event::ImeDisabled => l::LibraryInput::ImeDisabled,
                    Event::FocusLost => l::LibraryInput::FocusLost,
                    Event::Activate(Target::Navigate(page)) => {
                        return vec![WindowIntent::Settings(Action::Navigate(page))]
                    }
                    _ => return vec![],
                };
                self.library_input(input)
            }
            Self::Settings(_) => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Metrics;
    impl TextMeasurer for Metrics {
        fn measure(&mut self, text: &str, size: f32, _: u16, _: FontFamily) -> TextMeasurement {
            TextMeasurement {
                caret_stops: text
                    .char_indices()
                    .enumerate()
                    .map(|(n, (byte, _))| (byte, n as f32 * size * 0.6))
                    .chain(std::iter::once((
                        text.len(),
                        text.chars().count() as f32 * size * 0.6,
                    )))
                    .collect(),
            }
        }
    }
    #[test]
    fn normalized_input_and_ime_feed_the_active_dynamic_editor() {
        let mut ui = WindowUi::new(WindowSnapshot::Library(l::LibrarySnapshot {
            kind: l::LibraryKind::History,
            ..Default::default()
        }));
        let frame = ui.frame(
            Size {
                width: 900.,
                height: 650.,
            },
            1.25,
            &mut Metrics,
        );
        assert!(matches!(frame.controls, WindowControls::Library(_)));
        ui.library_input(l::LibraryInput::Focus(l::LibraryTarget::Search));
        assert!(ui
            .update(Event::ImePreedit {
                text: "zhong".into(),
                selection: Some(5..5)
            })
            .is_empty());
        assert_eq!(ui.text_edit().unwrap().text(), "");
        let intents = ui.update(Event::ImeCommit("中文".into()));
        assert!(intents.iter().any(|intent|matches!(intent,WindowIntent::Library(l::LibraryIntent::Query{query,..})if query.search=="中文")));
        assert_eq!(ui.text_edit().unwrap().text(), "中文");
        assert_eq!(ui.page(), Page::History);
        ui.snapshot(WindowSnapshot::Settings(Snapshot::default()));
        assert_eq!(ui.page(), Page::Ocr);
        assert!(ui.text_edit().is_none());
    }
}
