use super::*;
fn size() -> Size {
    Size {
        width: 1000.,
        height: 700.,
    }
}
fn center(rect: Rect) -> Point {
    Point {
        x: rect.x + rect.width / 2.,
        y: rect.y + rect.height / 2.,
    }
}
fn click(ui: &mut SettingsUi, target: Target) -> Vec<Action> {
    let scene = ui.frame(ui.size, ui.scale);
    let hit = scene
        .hits
        .iter()
        .find(|hit| hit.target == target)
        .unwrap_or_else(|| panic!("Target not visible: {target:?}"));
    ui.update(Event::PointerDown(center(hit.rect)))
}
fn key(ui: &mut SettingsUi, key: Key) -> Vec<Action> {
    ui.update(Event::KeyDown {
        key,
        modifiers: Modifiers::default(),
        repeat: false,
    })
}
#[test]
fn standalone_narrow_navigation_keeps_capacity_reachable() {
    let mut ui = SettingsUi::new(Snapshot::default());
    let frame = ui.frame(
        Size {
            width: 700.,
            height: 500.,
        },
        1.,
    );
    assert!(frame.hits.iter().any(|hit| hit.target == Target::Capacity));
    ui.set_embedded(true);
    let frame = ui.frame(
        Size {
            width: 700.,
            height: 500.,
        },
        1.,
    );
    assert!(!frame.hits.iter().any(|hit| hit.target == Target::Capacity));
}
#[test]
fn six_theme_appearances_preserve_original_colors_and_navigation_geometry() {
    for theme in [Theme::Native, Theme::Studio, Theme::Paper] {
        for dark in [false, true] {
            let mut ui = SettingsUi::new(Snapshot {
                theme,
                dark,
                ..Snapshot::default()
            });
            let scene = ui.frame(size(), 1.);
            let palette = palette(theme, dark);
            assert_eq!(
                scene.commands.first(),
                Some(&DrawCommand::FillRect {
                    rect: r(0., 0., 1000., 700.),
                    radius: 0.,
                    color: palette.canvas
                })
            );
            let ocr = scene
                .hits
                .iter()
                .find(|h| h.target == Target::Navigate(Page::Ocr))
                .unwrap();
            match theme {
                Theme::Native => assert_eq!(ocr.rect.width, 178.),
                Theme::Studio => assert_eq!(ocr.rect.width, 54.),
                Theme::Paper => assert_eq!(ocr.rect.height, 33.),
            }
            assert!(scene.commands.iter().any(|op|matches!(op,DrawCommand::Text{text,font:FontFamily::Serif,..}if text=="OCR"))== (theme==Theme::Paper));
            assert!(scene
                .hits
                .iter()
                .any(|hit| hit.target == Target::Section(Section::Recognition)));
            assert!(scene
                .commands
                .iter()
                .any(|op| matches!(op,DrawCommand::Text{text,..}if text=="本地模型 GPU 加速")));
            assert_eq!(
                click(&mut ui, Target::Navigate(Page::History)),
                [Action::Navigate(Page::History)]
            );
            assert!(
                ui.snapshot().ocr.gpu_acceleration,
                "Navigation to an external page must not replace the OCR projection"
            );
        }
    }
}
#[test]
fn field_controls_toggle_and_choose_with_pointer_and_keyboard() {
    let mut ui = SettingsUi::new(Snapshot::default());
    ui.frame(size(), 1.);
    click(&mut ui, Target::Section(Section::Recognition));
    assert_eq!(
        click(&mut ui, Target::Ocr(OcrField::GpuAcceleration)),
        [Action::EditOcr {
            field: OcrField::GpuAcceleration,
            value: Value::Toggle(false)
        }]
    );
    assert!(!ui.snapshot().ocr.gpu_acceleration);
    click(&mut ui, Target::Ocr(OcrField::RecognitionLanguage));
    assert_eq!(key(&mut ui, Key::ArrowDown), []);
    assert_eq!(
        key(&mut ui, Key::Enter),
        [Action::EditOcr {
            field: OcrField::RecognitionLanguage,
            value: Value::Text("zh-CN".into())
        }]
    );
    assert_eq!(
        ui.focused(),
        Some(Target::Ocr(OcrField::RecognitionLanguage))
    );
    key(&mut ui, Key::ArrowDown);
    assert_eq!(
        click(&mut ui, Target::Option(OcrField::RecognitionLanguage, 3)),
        [Action::EditOcr {
            field: OcrField::RecognitionLanguage,
            value: Value::Text("en".into())
        }]
    );
    click(&mut ui, Target::Section(Section::Window));
    click(&mut ui, Target::Ocr(OcrField::WindowPosition));
    key(&mut ui, Key::End);
    assert_eq!(
        key(&mut ui, Key::Space),
        [Action::EditOcr {
            field: OcrField::WindowPosition,
            value: Value::Text("last-position".into())
        }]
    );
}
#[test]
fn zoom_keeps_drawing_and_hit_testing_in_the_same_window_coordinates() {
    let mut ui = SettingsUi::new(Snapshot::default());
    let normal = ui.frame(
        Size {
            width: 1800.,
            height: 1300.,
        },
        1.,
    );
    let normal_hit = normal
        .hits
        .iter()
        .find(|h| h.target == Target::Ocr(OcrField::GpuAcceleration))
        .unwrap();
    let zoomed = ui.frame(
        Size {
            width: 3600.,
            height: 2600.,
        },
        2.,
    );
    let zoomed_hit = zoomed
        .hits
        .iter()
        .find(|h| h.target == Target::Ocr(OcrField::GpuAcceleration))
        .unwrap();
    assert_eq!(normal_hit.rect.scaled(2.), zoomed_hit.rect);
    assert_eq!(
        zoomed.hit_test(center(zoomed_hit.rect)),
        Some(Target::Ocr(OcrField::GpuAcceleration))
    );
    let actions = ui.update(Event::PointerDown(center(zoomed_hit.rect)));
    assert!(matches!(
        actions.as_slice(),
        [Action::EditOcr {
            field: OcrField::GpuAcceleration,
            value: Value::Toggle(false)
        }]
    ));
}
#[test]
fn clipped_controls_do_not_receive_clicks_and_focus_scrolls_to_hidden_controls() {
    let mut ui = SettingsUi::new(Snapshot::default());
    let scene = ui.frame(
        Size {
            width: 700.,
            height: 500.,
        },
        1.,
    );
    assert!(!scene
        .hits
        .iter()
        .any(|h| h.target == Target::Ocr(OcrField::HideSilentStatus)));
    let original_scroll = ui.scroll;
    for _ in 0..60 {
        key(&mut ui, Key::Tab);
        if ui.focused() == Some(Target::Ocr(OcrField::HideSilentStatus)) {
            break;
        }
    }
    assert_eq!(ui.focused(), Some(Target::Ocr(OcrField::HideSilentStatus)));
    assert!(ui.scroll > original_scroll);
    let scene = ui.frame(ui.size, 1.);
    let hit = scene
        .hits
        .iter()
        .find(|h| h.target == Target::Ocr(OcrField::HideSilentStatus))
        .unwrap();
    assert_eq!(hit.rect, hit.rect.intersect(scene.viewport));
    key(&mut ui, Key::Home);
    assert_eq!(ui.scroll, 0.);
    ui.update(Event::Scroll {
        position: Point { x: 650., y: 450. },
        delta_y: 100_000.,
    });
    assert_eq!(ui.scroll, ui.max_scroll);
    ui.frame(
        Size {
            width: 1400.,
            height: 2200.,
        },
        1.,
    );
    assert_eq!(ui.scroll, 0.);
}
#[test]
fn hotkey_recording_waits_for_matching_release_and_cancels_on_focus_loss() {
    let mut ui = SettingsUi::new(Snapshot::default());
    ui.frame(size(), 1.);
    assert_eq!(
        click(&mut ui, Target::Hotkey(0)),
        [Action::BeginHotkeyRecording { index: 0 }]
    );
    assert!(ui
        .update(Event::KeyDown {
            key: Key::Function(18),
            modifiers: Modifiers {
                shift: true,
                ..Modifiers::default()
            },
            repeat: false
        })
        .is_empty());
    assert!(ui
        .update(Event::KeyUp {
            key: Key::Character('a')
        })
        .is_empty());
    assert_eq!(
        ui.update(Event::KeyUp {
            key: Key::Function(18)
        }),
        [
            Action::EndHotkeyRecording,
            Action::SaveHotkey {
                index: 0,
                shortcut: "⇧F18".into()
            }
        ]
    );
    click(&mut ui, Target::Hotkey(1));
    assert_eq!(ui.update(Event::FocusLost), [Action::EndHotkeyRecording]);
    assert!(ui.update(Event::RecordedShortcut("F19".into())).is_empty());
    click(&mut ui, Target::Hotkey(2));
    assert_eq!(
        key(&mut ui, Key::Delete),
        [
            Action::EndHotkeyRecording,
            Action::SaveHotkey {
                index: 2,
                shortcut: "未设置".into()
            }
        ]
    );
}
#[test]
fn windows_hotkeys_keep_ctrl_and_win_distinct_in_display_and_storage() {
    let mut ui = SettingsUi::new(Snapshot {
        key_style: KeyStyle::Windows,
        ..Snapshot::default()
    });
    let scene = ui.frame(size(), 1.);
    assert!(scene
        .commands
        .iter()
        .any(|op| matches!(op,DrawCommand::Text{text,..} if text=="Alt")));
    assert!(!scene
        .commands
        .iter()
        .any(|op| matches!(op,DrawCommand::Text{text,..} if text=="⌥")));
    click(&mut ui, Target::Hotkey(0));
    ui.update(Event::KeyDown {
        key: Key::Function(21),
        modifiers: Modifiers {
            control: true,
            meta: true,
            ..Modifiers::default()
        },
        repeat: false,
    });
    assert_eq!(
        ui.update(Event::KeyUp {
            key: Key::Function(21)
        }),
        [
            Action::EndHotkeyRecording,
            Action::SaveHotkey {
                index: 0,
                shortcut: "⌘⊞F21".into()
            }
        ]
    );
}
#[test]
fn dropdown_overlays_own_pointer_hits_and_escape_and_resize_dismiss_them() {
    let mut ui = SettingsUi::new(Snapshot::default());
    ui.frame(size(), 1.);
    click(&mut ui, Target::Section(Section::Recognition));
    click(&mut ui, Target::Ocr(OcrField::RecognitionLanguage));
    let scene = ui.frame(ui.size, 1.);
    let option = scene
        .hits
        .iter()
        .find(|h| h.target == Target::Option(OcrField::RecognitionLanguage, 2))
        .unwrap();
    assert_eq!(scene.hit_test(center(option.rect)), Some(option.target));
    key(&mut ui, Key::Escape);
    assert!(ui.menu.is_none());
    key(&mut ui, Key::Space);
    assert!(ui.menu.is_some());
    ui.update(Event::Resize(Size {
        width: 650.,
        height: 500.,
    }));
    assert!(ui.menu.is_none());
    let scene = ui.frame(ui.size, 1.);
    assert!(
        scene
            .commands
            .iter()
            .filter(|op| matches!(op, DrawCommand::PushClip(_)))
            .count()
            == scene
                .commands
                .iter()
                .filter(|op| matches!(op, DrawCommand::PopClip))
                .count()
    );
}
