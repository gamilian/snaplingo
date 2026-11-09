//! Baseline coordinates are actual NSView frames from the retained AppKit toolbar.
use snaplingo_ui::{capture::*, Rect};
mod support {
    pub mod capture_fixture;
}
fn target(name: &str) -> CaptureTarget {
    for tool in CaptureTool::ALL {
        if tool.name() == name {
            return CaptureTarget::Tool(tool);
        }
    }
    match name {
        "shape-menu" => CaptureTarget::ShapeMenu,
        "arrow-menu" => CaptureTarget::ArrowMenu,
        "color" => CaptureTarget::Color,
        "palette" => CaptureTarget::PaletteMenu,
        "size" => CaptureTarget::Size,
        "filled" => CaptureTarget::Fill,
        "round" => CaptureTarget::Round,
        "font" => CaptureTarget::Font,
        "bold" => CaptureTarget::Bold,
        "italic" => CaptureTarget::Italic,
        "font-decrease" => CaptureTarget::FontDecrease,
        "font-increase" => CaptureTarget::FontIncrease,
        _ => CaptureTarget::Command(match name {
            "select" => CaptureCommand::Select,
            "undo" => CaptureCommand::Undo,
            "redo" => CaptureCommand::Redo,
            "clear" => CaptureCommand::Clear,
            "cancel" => CaptureCommand::Cancel,
            "ocr" => CaptureCommand::Ocr,
            "scrolling" => CaptureCommand::Scrolling,
            "favorite" => CaptureCommand::Favorite,
            "pin" => CaptureCommand::Pin,
            "save" => CaptureCommand::Save,
            "copy" => CaptureCommand::Copy,
            _ => panic!("Unknown native control {name}"),
        }),
    }
}
fn compare(actual: Rect, expected: &serde_json::Value, origin: Rect, scale: f32, label: &str) {
    let e = expected.as_array().unwrap();
    for (index, value) in [
        actual.x - origin.x,
        actual.y - origin.y,
        actual.width,
        actual.height,
    ]
    .into_iter()
    .enumerate()
    {
        assert!(
            (value - e[index].as_f64().unwrap() as f32 * scale).abs() <= 0.5,
            "{label}/{index}: expected {},got {value}",
            e[index]
        );
    }
}
#[test]
fn shared_capture_toolbar_and_palette_match_actual_native_controls() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../ui/tests/fixtures/native-capture-toolbar-layout.json"
    ))
    .unwrap();
    let mut count = 0;
    for scale in [1., 1.25, 2.] {
        let mut snapshot = support::capture_fixture::fixture();
        snapshot.ui_scale = scale;
        snapshot.size.width *= scale;
        snapshot.size.height *= scale;
        snapshot.selection = snapshot.selection.map(|r| Rect {
            x: r.x * scale,
            y: r.y * scale,
            width: r.width * scale,
            height: r.height * scale,
        });
        let mut ui = CaptureUi::default();
        let frame = ui.frame(&snapshot);
        let select = frame
            .controls
            .iter()
            .find(|c| c.target == CaptureTarget::Command(CaptureCommand::Select))
            .unwrap();
        let origin = Rect {
            x: select.rect.x - 6. * scale,
            y: select.rect.y - 4. * scale,
            ..Default::default()
        };
        for control in reference["controls"]
            .as_array()
            .unwrap()
            .iter()
            .chain(reference["style"]["controls"].as_array().unwrap())
        {
            let name = control["target"].as_str().unwrap();
            let actual = frame
                .controls
                .iter()
                .find(|c| c.target == target(name))
                .unwrap();
            compare(actual.rect, &control["frame"], origin, scale, name);
            count += 1;
            if let Some(enabled) = control.get("enabled") {
                assert_eq!(actual.enabled, enabled.as_bool().unwrap(), "{name}");
            }
            if let Some(selected) = control.get("selected") {
                assert_eq!(actual.selected, selected.as_bool().unwrap(), "{name}");
            }
        }
        ui.activate(CaptureTarget::PaletteMenu, &snapshot);
        let frame = ui.frame(&snapshot);
        let first = frame
            .controls
            .iter()
            .find(|c| c.target == CaptureTarget::Palette(0))
            .unwrap();
        let origin = Rect {
            x: first.rect.x - 12. * scale,
            y: first.rect.y - 30. * scale,
            ..Default::default()
        };
        for (index, expected) in reference["palette"]["swatches"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            let actual = frame
                .controls
                .iter()
                .find(|c| c.target == CaptureTarget::Palette(index))
                .unwrap();
            compare(actual.rect, expected, origin, scale, "swatch");
            count += 1;
        }
        for (expected, target) in reference["palette"]["buttons"]
            .as_array()
            .unwrap()
            .iter()
            .zip([
                CaptureTarget::AddColor,
                CaptureTarget::ReplaceColor,
                CaptureTarget::DeleteColor,
            ])
        {
            let actual = frame.controls.iter().find(|c| c.target == target).unwrap();
            compare(
                actual.rect,
                &expected["frame"],
                origin,
                scale,
                expected["label"].as_str().unwrap(),
            );
            assert_eq!(actual.enabled, expected["enabled"].as_bool().unwrap());
            count += 1;
        }
    }
    assert_eq!(count, 111);
}

#[test]
fn text_style_row_matches_actual_native_font_and_increment_controls() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../ui/tests/fixtures/native-capture-toolbar-layout.json"
    ))
    .unwrap();
    let reference = &fixture["text"];
    let mut count = 0;
    for scale in [1., 1.25, 2.] {
        let mut snapshot = support::capture_fixture::fixture();
        snapshot.tool = Some(CaptureTool::Text);
        snapshot.can_undo = false;
        snapshot.style.text_mode = true;
        snapshot.style.round_enabled = false;
        snapshot.style.bold = true;
        snapshot.style.italic = true;
        snapshot.style.font_size = 36;
        snapshot.ui_scale = scale;
        snapshot.size.width *= scale;
        snapshot.size.height *= scale;
        snapshot.selection = snapshot.selection.map(|r| Rect {
            x: r.x * scale,
            y: r.y * scale,
            width: r.width * scale,
            height: r.height * scale,
        });
        let frame = CaptureUi::default().frame(&snapshot);
        let first = frame
            .controls
            .iter()
            .find(|c| c.target == CaptureTarget::Command(CaptureCommand::Select))
            .unwrap()
            .rect;
        let origin = Rect {
            x: first.x - 6. * scale,
            y: first.y - 4. * scale,
            ..Default::default()
        };
        for control in reference["controls"]
            .as_array()
            .unwrap()
            .iter()
            .chain(reference["style"]["controls"].as_array().unwrap())
        {
            let name = control["target"].as_str().unwrap();
            let actual = frame
                .controls
                .iter()
                .find(|c| c.target == target(name))
                .unwrap();
            compare(actual.rect, &control["frame"], origin, scale, name);
            count += 1;
            if let Some(selected) = control.get("selected") {
                assert_eq!(actual.selected, selected.as_bool().unwrap(), "{name}");
            }
        }
    }
    assert_eq!(count, 96);
}
