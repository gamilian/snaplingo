use super::*;
fn key(ui: &mut SettingsUi, key: Key) -> Vec<Action> {
    ui.update(Event::KeyDown {
        key,
        modifiers: Modifiers::default(),
        repeat: false,
    })
}
fn translation() -> SettingsUi {
    let mut ui = SettingsUi::new(Snapshot::default());
    ui.update(Event::Activate(Target::Navigate(Page::Translation)));
    ui
}
#[test]
fn translation_fields_share_layout_but_edit_their_own_projection() {
    let mut ui = translation();
    assert_eq!(ui.page(), Page::Translation);
    for field in TranslationField::ALL {
        let value = match field.value(&ui.snapshot().translation) {
            Value::Toggle(v) => Value::Toggle(!v),
            Value::Text(v) => Value::Text(
                field
                    .choices()
                    .iter()
                    .find(|(id, _)| *id != v)
                    .unwrap()
                    .0
                    .into(),
            ),
            Value::Number(v) => Value::Number(v + 1),
        };
        let actions = ui.update(Event::SetValue(Target::Translation(field), value.clone()));
        assert_eq!(
            actions,
            [Action::EditTranslation {
                field,
                value: value.clone()
            }]
        );
        assert_eq!(field.value(&ui.snapshot().translation), value);
    }
    assert_eq!(ui.snapshot().ocr, OcrPreferences::default());
    ui.update(Event::Activate(Target::Navigate(Page::Ocr)));
    assert_eq!(ui.page(), Page::Ocr);
    assert!(ui
        .update(Event::SetValue(
            Target::Translation(TranslationField::AutoCopy),
            Value::Toggle(false)
        ))
        .is_empty());
}
#[test]
fn translation_ranges_drag_in_scaled_window_coordinates_and_stop_at_release() {
    let mut ui = translation();
    let size = Size {
        width: 1200.,
        height: 1000.,
    };
    ui.frame(size, 1.5);
    ui.update(Event::Activate(Target::Section(Section::Window)));
    let scene = ui.frame(size, 1.5);
    let field = TranslationField::WindowWidth;
    let target = Target::Translation(field);
    let track = scene
        .geometry
        .iter()
        .find(|n| n.kind == GeometryKind::Control(target))
        .unwrap()
        .rect;
    let point = Point {
        x: track.x + track.width * 0.5,
        y: track.y + track.height * 0.5,
    };
    let actions = ui.update(Event::PointerDown(point));
    assert_eq!(
        actions,
        [Action::EditTranslation {
            field,
            value: Value::Number(650)
        }]
    );
    let actions = ui.update(Event::PointerMove(Point {
        x: track.x + track.width * 2.,
        y: point.y,
    }));
    assert_eq!(
        actions,
        [Action::EditTranslation {
            field,
            value: Value::Number(1000)
        }]
    );
    ui.update(Event::PointerUp(Point {
        x: track.x + track.width * 2.,
        y: point.y,
    }));
    assert!(ui.update(Event::PointerMove(point)).is_empty());
    ui.update(Event::Focus(target));
    assert_eq!(
        key(&mut ui, Key::ArrowLeft),
        [Action::EditTranslation {
            field,
            value: Value::Number(990)
        }]
    );
    assert_eq!(
        key(&mut ui, Key::Home),
        [Action::EditTranslation {
            field,
            value: Value::Number(300)
        }]
    );
}
#[test]
fn accessibility_focus_does_not_activate_and_invalid_values_do_not_change_settings() {
    let mut ui = translation();
    let before = ui.snapshot().clone();
    assert!(ui
        .update(Event::Focus(Target::Translation(
            TranslationField::AutoCopy
        )))
        .is_empty());
    assert_eq!(ui.snapshot(), &before);
    for (target, value) in [
        (
            Target::Translation(TranslationField::WindowWidth),
            Value::Number(1001),
        ),
        (
            Target::Translation(TranslationField::DefaultSourceLang),
            Value::Text("bogus".into()),
        ),
        (Target::Ocr(OcrField::GpuAcceleration), Value::Toggle(false)),
    ] {
        assert!(ui.update(Event::SetValue(target, value)).is_empty());
    }
    assert_eq!(ui.snapshot(), &before);
    let actions = ui.update(Event::Activate(Target::Translation(
        TranslationField::AutoCopy,
    )));
    assert_eq!(
        actions,
        [Action::EditTranslation {
            field: TranslationField::AutoCopy,
            value: Value::Toggle(true)
        }]
    );
}
#[test]
fn translation_hotkeys_and_long_dropdown_use_the_current_page() {
    let mut ui = translation();
    assert_eq!(ui.hotkeys()[0].value, "⌥D");
    ui.update(Event::Activate(Target::Translation(
        TranslationField::DefaultSourceLang,
    )));
    key(&mut ui, Key::End);
    let actions = key(&mut ui, Key::Enter);
    assert_eq!(
        actions,
        [Action::EditTranslation {
            field: TranslationField::DefaultSourceLang,
            value: Value::Text("ar".into())
        }]
    );
    ui.update(Event::Activate(Target::Hotkey(0)));
    assert_eq!(ui.recording_index(), Some(0));
    assert_eq!(
        ui.update(Event::Activate(Target::Navigate(Page::Ocr))),
        [Action::EndHotkeyRecording]
    );
    assert_eq!(ui.recording_index(), None);
}
