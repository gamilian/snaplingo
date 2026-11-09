//! Geometry captured from the retained React page; see fixtures/README.md.
use snaplingo_ui::*;

fn rect(value: &serde_json::Value) -> Rect {
    Rect {
        x: value["x"].as_f64().unwrap() as f32,
        y: value["y"].as_f64().unwrap() as f32,
        width: value["width"].as_f64().unwrap() as f32,
        height: value["height"].as_f64().unwrap() as f32,
    }
}

fn targets(card: usize, row: usize) -> Vec<Target> {
    use ScreenshotField::*;
    if card == 0 {
        return if row < 5 {
            vec![
                Target::Hotkey(row),
                Target::ResetHotkey(row),
                Target::ClearHotkey(row),
            ]
        } else {
            vec![Target::Screenshot(PinBorderShortcut)]
        };
    }
    let field = if card == 1 {
        [SavePath, Format, NamingRule, AutoCopy, LongshotMaxHeight][row]
    } else {
        [
            SelectionBorderWidth,
            SelectionBorderColor,
            SelectionMaskColor,
            MaskOpacity,
            RememberLastTool,
            ShowSelectionSize,
            ShowMagnifier,
            MagnifierZoom,
            PinOpacity,
            PinShadow,
            PinPosition,
            PinBorder,
            CaptureUiScale,
        ][row]
    };
    if field == SavePath {
        vec![Target::Screenshot(field), Target::BrowseScreenshotDirectory]
    } else if !field.colors().is_empty() {
        // The retained DOM collector reports the invisible type=color input,
        // whose frame is distinct from the clickable palette label.
        (0..field.colors().len())
            .map(|i| Target::ScreenshotColor(field, i))
            .collect()
    } else {
        vec![Target::Screenshot(field)]
    }
}

#[test]
fn screenshot_matches_actual_react_in_all_36_reference_scenes() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/react-screenshot-layout.json")).unwrap();
    let mut differences = vec![];
    let mut count = 0;
    for reference in fixture["scenes"].as_array().unwrap() {
        let theme = match reference["theme"].as_str().unwrap() {
            "studio" => Theme::Studio,
            "paper" => Theme::Paper,
            _ => Theme::Native,
        };
        let mut ui = SettingsUi::new(Snapshot {
            theme,
            dark: reference["dark"].as_bool().unwrap(),
            ..Snapshot::default()
        });
        ui.update(Event::Activate(Target::Navigate(Page::Screenshot)));
        let size = Size {
            width: reference["width"].as_f64().unwrap() as f32,
            height: 650.,
        };
        ui.frame(size, 1.);
        let section = match reference["section"].as_str().unwrap() {
            "output" => Section::Output,
            "capture-interface" => Section::CaptureInterface,
            _ => Section::Hotkeys,
        };
        ui.update(Event::Activate(Target::Section(section)));
        let scene = ui.frame(size, 1.);
        let mut compare = |actual: Rect, expected: &serde_json::Value, label: String| {
            let expected = rect(expected);
            count += 1;
            for (axis, actual, expected) in [
                ("x", actual.x, expected.x),
                ("y", actual.y, expected.y),
                ("width", actual.width, expected.width),
                ("height", actual.height, expected.height),
            ] {
                if (actual - expected).abs() > 0.5 {
                    differences.push(format!(
                        "{}/{label}/{axis}: expected {expected}, got {actual}",
                        reference["source"].as_str().unwrap()
                    ));
                }
            }
        };
        compare(scene.content, &reference["content"], "content".into());
        for (index, card) in reference["cards"].as_array().unwrap().iter().enumerate() {
            let section = Section::for_page(Page::Screenshot)[index];
            let geometry = |kind| scene.geometry.iter().find(|g| g.kind == kind).unwrap().rect;
            compare(
                geometry(GeometryKind::Card(section)),
                &card["frame"],
                format!("{section:?}"),
            );
            for (row, reference_row) in card["rows"].as_array().unwrap().iter().enumerate() {
                for (control, target) in targets(index, row).into_iter().enumerate() {
                    compare(
                        geometry(GeometryKind::Control(target)),
                        &reference_row["controls"][control]["frame"],
                        format!("{section:?}/{target:?}"),
                    );
                }
            }
        }
    }
    assert_eq!(fixture["scenes"].as_array().unwrap().len(), 36);
    assert_eq!(count, 1836);
    assert!(
        differences.is_empty(),
        "{} geometry differences:\n{}",
        differences.len(),
        differences[..differences.len().min(30)].join("\n")
    );
}
