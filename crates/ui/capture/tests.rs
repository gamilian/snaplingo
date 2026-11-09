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
    assert!(matches!(
        ui.activate(CaptureTarget::Tool(CaptureTool::Ellipse), &snapshot)
            .as_slice(),
        [CaptureIntent::Tool(CaptureTool::Ellipse)]
    ));
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
