use snaplingo_ui::*;

fn page(snapshot: Snapshot) -> SettingsUi {
    let mut ui = SettingsUi::new(snapshot);
    ui.update(Event::Activate(Target::Navigate(Page::Screenshot)));
    ui
}

fn key(ui: &mut SettingsUi, key: Key, modifiers: Modifiers) -> Vec<Action> {
    ui.update(Event::KeyDown {
        key,
        modifiers,
        repeat: false,
    })
}

fn edit_actions(actions: &[Action]) -> Vec<(ScreenshotField, Value)> {
    actions
        .iter()
        .filter_map(|action| match action {
            Action::EditScreenshot { field, value } => Some((*field, value.clone())),
            _ => None,
        })
        .collect()
}

#[test]
fn conditional_output_fields_preserve_composition_and_commit_at_the_original_save_points() {
    let mut ui = page(Snapshot::default());
    ui.update(Event::Activate(Target::Screenshot(
        ScreenshotField::SavePath,
    )));
    assert!(ui
        .update(Event::ImePreedit {
            text: "截图".into(),
            selection: Some(6..6)
        })
        .is_empty());
    assert_eq!(ui.snapshot().screenshot.save_path, "");
    assert_eq!(ui.text_edit().unwrap().display_text(), "截图");
    assert!(key(&mut ui, Key::Enter, Modifiers::default()).is_empty());
    assert!(ui.text_edit().unwrap().is_composing());
    assert!(ui.update(Event::ImeCommit("/tmp/截图🙂".into())).is_empty());
    let actions = ui.update(Event::FocusLost);
    assert_eq!(
        edit_actions(&actions),
        vec![(ScreenshotField::SavePath, Value::Text("/tmp/截图🙂".into()))]
    );

    ui.update(Event::SetValue(
        Target::Screenshot(ScreenshotField::NamingRule),
        Value::Text("custom".into()),
    ));
    ui.update(Event::Activate(Target::Screenshot(
        ScreenshotField::CustomFileName,
    )));
    key(
        &mut ui,
        Key::Character('a'),
        Modifiers {
            meta: true,
            ..Modifiers::default()
        },
    );
    let actions = ui.update(Event::ClipboardText("中文\r\nname".into()));
    assert_eq!(
        edit_actions(&actions),
        vec![(
            ScreenshotField::CustomFileName,
            Value::Text("中文name".into())
        )]
    );
    assert_eq!(ui.snapshot().screenshot.custom_file_name, "中文name");

    ui.update(Event::FocusLost);
    ui.update(Event::SetValue(
        Target::Screenshot(ScreenshotField::Format),
        Value::Text("jpg".into()),
    ));
    let scene = ui.frame(
        Size {
            width: 900.,
            height: 650.,
        },
        1.,
    );
    for field in [ScreenshotField::Quality, ScreenshotField::CustomFileName] {
        assert!(scene
            .geometry
            .iter()
            .any(|node| node.kind == GeometryKind::Control(Target::Screenshot(field))));
    }
}

#[test]
fn screenshot_local_shortcuts_reject_global_and_reserved_keys_then_restore_only_annotation_bindings(
) {
    let mut ui = page(Snapshot::default());
    let field = ScreenshotField::AnnotationShortcut("rectangle");
    assert_eq!(
        ui.update(Event::Activate(Target::Screenshot(field))),
        vec![Action::BeginLocalShortcutRecording { field }]
    );
    key(&mut ui, Key::Function(1), Modifiers::default());
    assert!(edit_actions(&ui.update(Event::KeyUp {
        key: Key::Function(1)
    }))
    .is_empty());
    assert!(ui.snapshot().error.is_some());
    key(&mut ui, Key::Character('c'), Modifiers::default());
    assert!(edit_actions(&ui.update(Event::KeyUp {
        key: Key::Character('c')
    }))
    .is_empty());
    key(&mut ui, Key::Character('z'), Modifiers::default());
    let actions = ui.update(Event::KeyUp {
        key: Key::Character('z'),
    });
    assert_eq!(
        edit_actions(&actions),
        vec![(field, Value::Text("z".into()))]
    );
    assert!(actions.contains(&Action::EndHotkeyRecording));

    let pin = ScreenshotField::PinBorderShortcut;
    ui.update(Event::Activate(Target::Screenshot(pin)));
    key(
        &mut ui,
        Key::Character('b'),
        Modifiers {
            meta: true,
            shift: true,
            ..Modifiers::default()
        },
    );
    let actions = ui.update(Event::KeyUp {
        key: Key::Character('b'),
    });
    assert_eq!(
        edit_actions(&actions),
        vec![(pin, Value::Text("meta+shift+b".into()))]
    );
    ui.update(Event::Activate(Target::Screenshot(
        ScreenshotField::ResetAnnotationShortcuts,
    )));
    assert_eq!(
        ui.snapshot().screenshot.annotation_tool_shortcuts["rectangle"],
        "b"
    );
    assert_eq!(ui.snapshot().screenshot.pin_border_shortcut, "meta+shift+b");
}

#[test]
fn screenshot_palette_actions_and_scaled_slider_use_the_same_visible_controls() {
    let mut ui = page(Snapshot::default());
    let color = ScreenshotField::SelectionMaskColor;
    ui.update(Event::Activate(Target::ScreenshotColor(color, 3)));
    assert_eq!(
        ui.snapshot().screenshot.selection_mask_color,
        color.colors()[3]
    );
    assert_eq!(
        ui.update(Event::Activate(Target::PickScreenshotColor(color))),
        vec![Action::SelectScreenshotColor {
            field: color,
            rgba: color.colors()[3]
        }]
    );

    let target = Target::Screenshot(ScreenshotField::MaskOpacity);
    ui.update(Event::Focus(target));
    let scene = ui.frame(
        Size {
            width: 1350.,
            height: 975.,
        },
        1.5,
    );
    let hit = scene.hits.iter().find(|hit| hit.target == target).unwrap();
    let endpoint = Point {
        x: hit.rect.x + hit.rect.width - 9. * 1.5,
        y: hit.rect.y + hit.rect.height / 2.,
    };
    let actions = ui.update(Event::PointerDown(endpoint));
    assert_eq!(
        edit_actions(&actions),
        vec![(ScreenshotField::MaskOpacity, Value::Number(90))]
    );
    ui.update(Event::PointerUp(endpoint));
    assert_eq!(
        &ui.snapshot().screenshot.selection_mask_color[..3],
        &color.colors()[3][..3]
    );
    assert_eq!(ui.snapshot().screenshot.selection_mask_color[3], 230);
}

#[test]
fn longshot_height_matches_number_input_rounding_clamping_and_invalid_blur() {
    for (text, expected) in [
        ("100.5", Some(101)),
        ("1e4", Some(10_000)),
        ("100001", Some(100_000)),
        ("99.9", None),
        ("NaN", None),
    ] {
        let mut ui = page(Snapshot::default());
        let field = ScreenshotField::LongshotMaxHeight;
        ui.update(Event::Activate(Target::Screenshot(field)));
        key(
            &mut ui,
            Key::Character('a'),
            Modifiers {
                meta: true,
                ..Modifiers::default()
            },
        );
        ui.update(Event::TextInput(text.into()));
        let actions = ui.update(Event::FocusLost);
        assert_eq!(
            edit_actions(&actions),
            expected
                .map(|height| vec![(field, Value::Number(height))])
                .unwrap_or_default(),
            "{text}"
        );
        assert_eq!(
            ui.snapshot().screenshot.longshot_max_height,
            expected.unwrap_or(30_000)
        );
    }
}

#[test]
fn longshot_number_arrows_match_webkit_drafts_and_save_only_after_leaving_the_field() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/react-screenshot-number-steps.json")).unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let text = case["value"].as_str().unwrap();
        for increase in [true, false] {
            let expected = case[if increase { "up" } else { "down" }].as_str().unwrap();
            for pointer in [false, true] {
                let mut ui = page(Snapshot::default());
                ui.update(Event::Activate(Target::Screenshot(
                    ScreenshotField::LongshotMaxHeight,
                )));
                key(
                    &mut ui,
                    Key::Character('a'),
                    Modifiers {
                        meta: true,
                        ..Modifiers::default()
                    },
                );
                ui.update(Event::TextInput(text.into()));
                let actions = if pointer {
                    let scene = ui.frame(
                        Size {
                            width: 1350.,
                            height: 975.,
                        },
                        1.5,
                    );
                    let target = Target::ScreenshotStep(increase);
                    let hit = scene.hits.iter().find(|hit| hit.target == target).unwrap();
                    ui.update(Event::PointerDown(Point {
                        x: hit.rect.x + hit.rect.width / 2.,
                        y: hit.rect.y + hit.rect.height / 2.,
                    }))
                } else {
                    key(
                        &mut ui,
                        if increase {
                            Key::ArrowUp
                        } else {
                            Key::ArrowDown
                        },
                        Modifiers::default(),
                    )
                };
                assert!(
                    edit_actions(&actions).is_empty(),
                    "{text} increase={increase} pointer={pointer}"
                );
                assert_eq!(
                    ui.text_edit().unwrap().text(),
                    expected,
                    "{text} increase={increase} pointer={pointer}"
                );
                assert_eq!(ui.snapshot().screenshot.longshot_max_height, 30_000);
            }
        }
    }

    let mut ui = page(Snapshot::default());
    ui.update(Event::Activate(Target::Screenshot(
        ScreenshotField::LongshotMaxHeight,
    )));
    assert!(ui
        .update(Event::Activate(Target::ScreenshotStep(true)))
        .is_empty());
    assert!(ui
        .update(Event::Activate(Target::ScreenshotStep(true)))
        .is_empty());
    assert_eq!(ui.text_edit().unwrap().text(), "32000");
    assert_eq!(
        edit_actions(&ui.update(Event::FocusLost)),
        vec![(ScreenshotField::LongshotMaxHeight, Value::Number(32_000))]
    );
}
