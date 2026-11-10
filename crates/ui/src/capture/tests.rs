use super::*;
fn fixture() -> CaptureSnapshot {
    CaptureSnapshot {
        editing: true,
        selection: Some(box_rect(8., 8., 700., 200.)),
        tool: Some(CaptureTool::Rectangle),
        ..Default::default()
    }
}

#[test]
fn toolbar_preserves_native_left_alignment_edge_flip_and_error_rows() {
    let mut snapshot = fixture();
    snapshot.size = Size {
        width: 900.,
        height: 650.,
    };
    snapshot.selection = Some(box_rect(100., 100., 200., 100.));
    let select_rect = |snapshot: &CaptureSnapshot| {
        CaptureUi::default()
            .frame(snapshot)
            .controls
            .into_iter()
            .find(|control| control.target == CaptureTarget::Command(CaptureCommand::Select))
            .unwrap()
            .rect
    };
    // Native AppKit toolbar: 700 x 112, left edge follows the selection,
    // 14-point vertical gap; the first button is offset by (6, 4).
    assert_eq!(select_rect(&snapshot), box_rect(106., 218., 28., 28.));
    snapshot.selection = Some(box_rect(800., 550., 50., 80.));
    assert_eq!(select_rect(&snapshot), box_rect(198., 428., 28., 28.));
    snapshot.style.error = Some("保存失败，请重试".into());
    let frame = CaptureUi::default().frame(&snapshot);
    let select = frame
        .controls
        .iter()
        .find(|control| control.target == CaptureTarget::Command(CaptureCommand::Select))
        .unwrap();
    assert_eq!(select.rect, box_rect(198., 392., 28., 28.));
    assert!(frame.scene.commands.iter().any(
        |command| matches!(command, DrawCommand::Text { text, .. } if text == "保存失败，请重试")
    ));
    snapshot.size.width = 450.;
    snapshot.selection = Some(box_rect(40., 100., 100., 100.));
    assert_eq!(select_rect(&snapshot), box_rect(14., 218., 28., 28.));
    assert_eq!(
        presentation::toolbar_size(&snapshot.style, 434., 1.),
        Size {
            width: 434.,
            height: 180.
        }
    );
    snapshot.style.error = None;
    snapshot.ui_scale = 1.25;
    snapshot.size = Size {
        width: 1125.,
        height: 812.5,
    };
    snapshot.selection = Some(box_rect(125., 125., 250., 125.));
    assert_eq!(select_rect(&snapshot), box_rect(132.5, 272.5, 35., 35.));
}

#[test]
fn native_and_scene_toolbar_intents_share_busy_toggle_and_shift_save_policy() {
    let mut snapshot = fixture();
    assert!(presentation::toolbar_intent(
        CaptureTarget::Tool(CaptureTool::Rectangle),
        &snapshot,
        false,
        false
    )
    .is_none());
    assert!(matches!(
        presentation::toolbar_intent(
            CaptureTarget::Tool(CaptureTool::Rectangle),
            &snapshot,
            true,
            false
        ),
        Some(CaptureIntent::Tool(CaptureTool::Rectangle))
    ));
    snapshot.tool = Some(CaptureTool::Pen);
    assert!(matches!(
        presentation::toolbar_intent(
            CaptureTarget::Tool(CaptureTool::Pen),
            &snapshot,
            false,
            false
        ),
        Some(CaptureIntent::Command(CaptureCommand::Select))
    ));
    assert!(matches!(
        presentation::toolbar_intent(
            CaptureTarget::Command(CaptureCommand::Save),
            &snapshot,
            false,
            true
        ),
        Some(CaptureIntent::Command(CaptureCommand::QuickSave))
    ));
    assert!(presentation::toolbar_intent(
        CaptureTarget::Command(CaptureCommand::Undo),
        &snapshot,
        false,
        false
    )
    .is_none());
    snapshot.busy = true;
    assert!(presentation::toolbar_intent(
        CaptureTarget::Command(CaptureCommand::Save),
        &snapshot,
        false,
        true
    )
    .is_none());
}

#[test]
fn palette_keeps_native_rows_selection_and_pending_mutation_availability() {
    let mut style = CaptureStyle::default();
    style.presets = (0..8).map(|n| [n, 0, 0, 255]).collect();
    let layout = PalettePresentation::new(&style, Some(7), false, false, 1.25);
    assert_eq!(
        layout.size,
        Size {
            width: 310.,
            height: 207.5
        }
    );
    assert_eq!(
        layout.control(CaptureTarget::Palette(7)).unwrap().rect,
        box_rect(15., 75., 30., 30.)
    );
    assert!(layout.control(CaptureTarget::ReplaceColor).unwrap().enabled);
    let pending = PalettePresentation::new(&style, Some(7), false, true, 1.25);
    for target in [
        CaptureTarget::AddColor,
        CaptureTarget::ReplaceColor,
        CaptureTarget::DeleteColor,
    ] {
        assert!(!pending.control(target).unwrap().enabled);
    }
    style.color = style.presets[0];
    let duplicate = PalettePresentation::new(&style, Some(7), false, false, 1.25);
    assert!(!duplicate.control(CaptureTarget::AddColor).unwrap().enabled);
    assert!(
        !duplicate
            .control(CaptureTarget::ReplaceColor)
            .unwrap()
            .enabled
    );
    assert!(
        duplicate
            .control(CaptureTarget::DeleteColor)
            .unwrap()
            .enabled
    );
    assert_eq!(
        layout.frame(
            box_rect(250., 300., 200., 100.),
            box_rect(0., 0., 500., 500.),
            1.25
        ),
        box_rect(140., 87.5, 310., 207.5)
    );
}

#[test]
fn capture_overlay_labels_and_lens_preserve_native_edges_and_pixel_sampling() {
    let overlay = presentation::selection_overlay(
        box_rect(-10., -20., 50., 70.),
        Size {
            width: 100.,
            height: 80.,
        },
    );
    assert_eq!(
        overlay.masks,
        [
            box_rect(0., 0., 100., 0.),
            box_rect(0., 50., 100., 30.),
            box_rect(0., 0., 0., 50.),
            box_rect(40., 0., 60., 50.)
        ]
    );
    assert_eq!(
        presentation::selection_badge(
            box_rect(90., 5., 20., 20.),
            Size {
                width: 100.,
                height: 80.
            },
            30.,
            1.
        ),
        box_rect(54., 5., 46., 20.)
    );
    let lens = MagnifierPresentation::new(1.);
    assert_eq!(
        lens.frame(
            Point { x: 290., y: 240. },
            Size {
                width: 300.,
                height: 250.
            }
        ),
        box_rect(48., 0., 228., 226.)
    );
    let sample = presentation::magnifier_sample(
        Point::default(),
        Size {
            width: 64.,
            height: 64.,
        },
        8,
        1.,
    );
    assert_eq!(sample.desired, box_rect(-14., -8., 29., 17.));
    assert_eq!(sample.source, box_rect(0., 0., 15., 9.));
    assert_eq!(sample.destination, box_rect(112., 64., 120., 72.));
    assert_eq!(
        presentation::color_text([12, 34, 56], true),
        "rgb(12, 34, 56)"
    );
    assert_eq!(
        presentation::hint_frame(
            Size {
                width: 300.,
                height: 200.
            },
            Size {
                width: 999.,
                height: 90.
            },
            1.25
        ),
        box_rect(20., 95., 260., 80.)
    );
}

#[test]
fn selecting_keeps_frozen_pixels_undimmed_until_a_selection_and_has_no_quick_toolbar() {
    let mut snapshot = CaptureSnapshot {
        hint: String::new(),
        ..Default::default()
    };
    let mut ui = CaptureUi::default();
    assert!(ui.frame(&snapshot).scene.commands.is_empty());
    snapshot.selection = Some(box_rect(10., 50., 440., 200.));
    let frame = ui.frame(&snapshot);
    assert!(frame.controls.is_empty());
    assert!(frame.scene.commands.iter().any(|command|matches!(command,DrawCommand::Text{text,color,..}if text=="440 x 200 px"&&*color==[255;4])));
    assert!(frame.scene.commands.iter().any(|command|matches!(command,DrawCommand::FillRect{radius,color,..}if *radius==0.&&*color==[0,0,0,210])));
}

#[test]
fn grouped_tools_preserve_repeated_shape_selection_and_toggle_other_tools() {
    let mut snapshot = fixture();
    let mut ui = CaptureUi::default();
    let frame = ui.frame(&snapshot);
    assert!(!frame
        .controls
        .iter()
        .any(|c| c.target == CaptureTarget::Tool(CaptureTool::Ellipse)));
    ui.activate(CaptureTarget::ShapeMenu, &snapshot);
    let frame = ui.frame(&snapshot);
    assert!(frame
        .controls
        .iter()
        .any(|c| c.target == CaptureTarget::Tool(CaptureTool::Ellipse)));
    assert!(matches!(
        ui.activate(CaptureTarget::Tool(CaptureTool::Ellipse), &snapshot)
            .as_slice(),
        [CaptureIntent::Tool(CaptureTool::Ellipse)]
    ));
    assert!(!ui.dismiss_menus());
    snapshot.tool = Some(CaptureTool::Ellipse);
    ui.frame(&snapshot);
    assert!(
        ui.activate(CaptureTarget::Tool(CaptureTool::Ellipse), &snapshot)
            .is_empty(),
        "A repeated grouped-tool click preserves the native editor state without a command"
    );
    snapshot.tool = Some(CaptureTool::Pen);
    ui.frame(&snapshot);
    assert!(matches!(
        ui.activate(CaptureTarget::Tool(CaptureTool::Pen), &snapshot)
            .as_slice(),
        [CaptureIntent::Command(CaptureCommand::Select)]
    ));
}

#[test]
fn palette_color_and_system_picker_are_separate_and_unavailable_actions_do_not_write() {
    let mut snapshot = fixture();
    snapshot.style.presets = vec![[1, 2, 3, 255], [4, 5, 6, 128]];
    let mut ui = CaptureUi::default();
    ui.frame(&snapshot);
    assert!(matches!(
        ui.activate(CaptureTarget::Color, &snapshot).as_slice(),
        [CaptureIntent::SystemColor(_)]
    ));
    ui.activate(CaptureTarget::PaletteMenu, &snapshot);
    let frame = ui.frame(&snapshot);
    assert!(
        !frame
            .controls
            .iter()
            .find(|c| c.target == CaptureTarget::ReplaceColor)
            .unwrap()
            .enabled
    );
    assert!(ui
        .activate(CaptureTarget::DeleteColor, &snapshot)
        .is_empty());
    assert!(matches!(
        ui.activate(CaptureTarget::Palette(1), &snapshot).as_slice(),
        [CaptureIntent::Style {
            value: CaptureStyleValue::Color([4, 5, 6, 128]),
            ..
        }]
    ));
    assert!(!ui.dismiss_menus());
    ui.activate(CaptureTarget::PaletteMenu, &snapshot);
    ui.frame(&snapshot);
    assert!(ui.dismiss_menus());
    assert!(!ui.dismiss_menus());
}

#[test]
fn toolbar_background_consumes_clicks_and_long_font_menu_reaches_the_last_family() {
    let mut snapshot = fixture();
    snapshot.style.text_mode = true;
    snapshot.style.round_enabled = false;
    snapshot.style.fonts = (0..70).map(|i| format!("Family {i}")).collect();
    let mut ui = CaptureUi::default();
    let frame = ui.frame(&snapshot);
    let select = frame
        .controls
        .iter()
        .find(|c| c.target == CaptureTarget::Command(CaptureCommand::Select))
        .unwrap()
        .rect;
    assert!(ui
        .pointer_down(
            Point {
                x: select.x + 500.,
                y: select.y + 3.
            },
            &snapshot
        )
        .unwrap()
        .is_empty());
    ui.activate(CaptureTarget::Font, &snapshot);
    ui.frame(&snapshot);
    assert!(ui.scroll(
        Point {
            x: ui.font_view.x + 1.,
            y: ui.font_view.y + 1.
        },
        10_000.,
        &snapshot
    ));
    let frame = ui.frame(&snapshot);
    let last = frame
        .controls
        .iter()
        .find(|c| c.target == CaptureTarget::FontOption(70))
        .unwrap();
    assert!(last.rect.height > 0.);
    assert!(
        matches!(ui.activate(last.target,&snapshot).as_slice(),[CaptureIntent::Style{value:CaptureStyleValue::FontFamily(f),..}]if f=="Family 69")
    );
}

#[test]
fn slider_emits_preview_then_one_committed_edit_and_busy_controls_do_not_edit() {
    let mut snapshot = fixture();
    let mut ui = CaptureUi::default();
    let frame = ui.frame(&snapshot);
    let size = frame
        .controls
        .iter()
        .find(|c| c.target == CaptureTarget::Size)
        .unwrap()
        .rect;
    let point = Point {
        x: size.x + size.width - 1.,
        y: size.y + size.height / 2.,
    };
    assert!(matches!(
        ui.pointer_down(point, &snapshot).unwrap().as_slice(),
        [CaptureIntent::Style {
            value: CaptureStyleValue::StrokeWidth(32),
            commit: false
        }]
    ));
    assert!(matches!(
        ui.pointer_up(point, &snapshot).as_slice(),
        [CaptureIntent::Style { commit: true, .. }]
    ));
    assert!(ui.pointer_up(point, &snapshot).is_empty());
    snapshot.busy = true;
    assert!(ui.pointer_down(point, &snapshot).unwrap().is_empty());
    assert!(ui.pointer_up(point, &snapshot).is_empty());
    ui.frame(&snapshot);
    assert!(ui
        .activate(CaptureTarget::Tool(CaptureTool::Pen), &snapshot)
        .is_empty());
}
