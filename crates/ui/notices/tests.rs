use super::*;
use crate::{FontFamily, Size, Snapshot, TextMeasurement, TextMeasurer};
struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, _: &str, _: f32, _: u16, _: FontFamily) -> TextMeasurement {
        Default::default()
    }
}
fn frame(ui: &mut NoticesUi) {
    ui.frame(
        Size {
            width: 720.,
            height: 640.,
        },
        1.,
        &mut Metrics,
    );
}
fn ui() -> NoticesUi {
    NoticesUi::new(
        Snapshot::default(),
        vec![
            NoticeComponent {
                name: "Renderer".into(),
                version: "1.0".into(),
                license: "MIT".into(),
                source: "https://example.com/renderer".into(),
                text: "Full license text\nCopyright 2026".into(),
            },
            NoticeComponent {
                name: "Input".into(),
                version: "2.0".into(),
                license: "Apache-2.0".into(),
                source: "https://example.com/input".into(),
                text: "Apache text".into(),
            },
        ],
    )
}
#[test]
fn search_preserves_ime_draft_then_filters_name_or_license_on_commit() {
    let mut ui = ui();
    frame(&mut ui);
    ui.update(NoticesInput::Preedit("mit".into(), None));
    assert_eq!(ui.filtered.len(), 2);
    ui.update(NoticesInput::Commit("mit".into()));
    assert_eq!(ui.filtered, [0]);
    ui.update(NoticesInput::SetSearch("input".into()));
    assert_eq!(ui.filtered, [1]);
    frame(&mut ui);
    ui.update(NoticesInput::Activate(NoticesTarget::ClearSearch));
    assert_eq!(ui.filtered.len(), 2);
}
#[test]
fn detail_keeps_full_license_and_source_copy_and_escape_returns_before_closing() {
    let mut ui = ui();
    frame(&mut ui);
    ui.update(NoticesInput::Activate(NoticesTarget::Component(0)));
    frame(&mut ui);
    assert_eq!(
        ui.update(NoticesInput::Activate(NoticesTarget::Source)),
        [NoticesIntent::OpenSource(
            "https://example.com/renderer".into()
        )]
    );
    ui.update(NoticesInput::Activate(NoticesTarget::License));
    ui.update(NoticesInput::Key(
        Key::Character('a'),
        Modifiers {
            control: true,
            ..Modifiers::default()
        },
    ));
    assert_eq!(
        ui.update(NoticesInput::Key(
            Key::Character('c'),
            Modifiers {
                control: true,
                ..Modifiers::default()
            }
        )),
        [NoticesIntent::Copy(
            "Full license text\nCopyright 2026".into()
        )]
    );
    assert!(ui
        .update(NoticesInput::Key(Key::Escape, Modifiers::default()))
        .is_empty());
    assert!(ui.selected().is_none());
    assert_eq!(
        ui.update(NoticesInput::Key(Key::Escape, Modifiers::default())),
        [NoticesIntent::Close]
    );
}
