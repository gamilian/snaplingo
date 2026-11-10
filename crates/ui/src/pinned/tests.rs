use super::*;

fn state() -> State {
    State::new(
        600,
        300,
        Some("OCR source".into()),
        Preferences {
            opacity: 0.75,
            shadow: true,
            border: false,
            border_shortcut: "f8".into(),
        },
    )
}

#[test]
fn shortcuts_keep_operations_display_changes_and_toolbar_dismissal_separate() {
    let mut ui = state();
    assert_eq!(
        ui.key(Key::Function(8), false, None),
        Some(Action::ToggleBorder)
    );
    assert_eq!(ui.key(Key::Function(8), true, None), None);
    for command in [
        Modifiers {
            control: true,
            shift: true,
            ..Modifiers::default()
        },
        Modifiers {
            meta: true,
            shift: true,
            ..Modifiers::default()
        },
    ] {
        ui.modifiers(command);
        assert_eq!(
            ui.key(Key::Character('c'), false, None),
            Some(Action::CopyText("OCR source".into()))
        );
        assert_eq!(
            ui.key(Key::Character('s'), false, None),
            Some(Action::QuickSave)
        );
    }
    ui.modifiers(Modifiers::default());
    ui.key(Key::Space, false, None);
    assert!(ui.forced_toolbar);
    assert_eq!(ui.key(Key::Escape, false, None), None);
    assert!(!ui.forced_toolbar);
    assert_eq!(ui.key(Key::Escape, false, None), Some(Action::Close));
    ui.modifiers(Modifiers {
        shift: true,
        ..Modifiers::default()
    });
    assert_eq!(ui.key(Key::Escape, false, None), Some(Action::Destroy));
    assert_eq!(ui.movement(Key::ArrowUp), Some((0., -5.)));
}

#[test]
fn inspection_copies_original_channels_and_toggles_format_on_shift_press() {
    let mut ui = state();
    ui.modifiers(Modifiers {
        alt: true,
        ..Modifiers::default()
    });
    assert_eq!(
        ui.key(Key::Character('c'), false, Some([180, 60, 30])),
        Some(Action::CopyText("#B43C1E".into()))
    );
    ui.modifiers(Modifiers {
        alt: true,
        shift: true,
        ..Modifiers::default()
    });
    assert_eq!(
        ui.key(Key::Character('c'), false, Some([180, 60, 30])),
        Some(Action::CopyText("rgb(180, 60, 30)".into()))
    );
    ui.modifiers(Modifiers {
        alt: true,
        shift: true,
        ..Modifiers::default()
    });
    assert!(ui.rgb);
    assert_eq!(ui.key(Key::Character('c'), false, None), None);
    let before = ui.presentation.clone();
    assert!(!ui.scroll(10.));
    assert_eq!(ui.presentation, before);
    assert_eq!(ui.movement(Key::ArrowUp), None);
}

#[test]
fn layout_keeps_the_mac_toolbar_and_lens_edge_placement() {
    let mut ui = state();
    assert!(ui.toolbar().is_empty());
    ui.hovered = true;
    let controls = ui.toolbar();
    assert_eq!(
        controls
            .iter()
            .map(|(_, rect, label)| (*rect, *label))
            .collect::<Vec<_>>(),
        [
            (
                Rect {
                    x: 462.,
                    y: 6.,
                    width: 50.,
                    height: 26.
                },
                "Copy"
            ),
            (
                Rect {
                    x: 512.,
                    y: 6.,
                    width: 50.,
                    height: 26.
                },
                "Save"
            ),
            (
                Rect {
                    x: 562.,
                    y: 6.,
                    width: 30.,
                    height: 26.
                },
                "X"
            ),
        ]
    );
    ui.cursor = Point { x: 590., y: 290. };
    assert_eq!(
        ui.lens_rect(),
        Rect {
            x: 416.,
            y: 164.,
            width: 160.,
            height: 112.
        }
    );
    ui.presentation.rotation = 90;
    assert_eq!(ui.toolbar_rect().x, 160.);
    ui.presentation.zoom_by(-100.);
    assert_eq!(ui.toolbar_rect().x, 0.);
}

#[test]
fn menu_and_wheel_preserve_transform_policy_and_export_dimensions() {
    let mut ui = state();
    ui.key(Key::Character('1'), false, None);
    assert_eq!(ui.dimensions(), (300., 600.));
    ui.key(Key::Character('3'), false, None);
    ui.key(Key::Character('5'), false, None);
    assert!(ui.presentation.flip_x && ui.presentation.grayscale);
    assert_eq!(ui.menu(10), Some(Action::Save));
    ui.menu(4);
    assert_eq!(ui.presentation.opacity, 0.5);
    ui.scroll(4.);
    assert_eq!(ui.presentation.zoom, 1.1);
    ui.modifiers(Modifiers {
        meta: true,
        ..Modifiers::default()
    });
    ui.scroll(-2.);
    assert_eq!(ui.presentation.opacity, 104. / 255.);
    assert_eq!(
        ui.presentation.export_size(600, 300),
        ImageSize {
            width: 660,
            height: 330
        }
    );
}
