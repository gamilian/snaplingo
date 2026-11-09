//! These fixtures were captured from the actual previous React page, rather
//! than computed from this crate's layout implementation. See fixtures/README.
use snaplingo_ui::*;
fn rectangle(value: &serde_json::Value) -> Rect {
    Rect {
        x: value["x"].as_f64().unwrap() as f32,
        y: value["y"].as_f64().unwrap() as f32,
        width: value["width"].as_f64().unwrap() as f32,
        height: value["height"].as_f64().unwrap() as f32,
    }
}
fn compare(actual: Rect, expected: &serde_json::Value, label: &str) {
    let expected = rectangle(expected);
    for (actual, expected) in [
        (actual.x, expected.x),
        (actual.y, expected.y),
        (actual.width, expected.width),
        (actual.height, expected.height),
    ] {
        assert!(
            (actual - expected).abs() <= 0.5,
            "{label}: expected {expected}, got {actual}"
        );
    }
}
#[test]
fn shared_pages_match_actual_react_in_all_84_reference_scenes() {
    let mut fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/react-ocr-layout.json")).unwrap();
    let translation: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/react-translation-layout.json")).unwrap();
    fixture["scenes"]
        .as_array_mut()
        .unwrap()
        .extend(translation["scenes"].as_array().unwrap().iter().cloned());
    let mut compared = 0;
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
        let page = if reference["page"] == "translation" {
            Page::Translation
        } else {
            Page::Ocr
        };
        ui.update(Event::Activate(Target::Navigate(page)));
        let size = Size {
            width: reference["width"].as_f64().unwrap() as f32,
            height: reference["height"].as_f64().unwrap() as f32,
        };
        let section = match reference["section"].as_str().unwrap() {
            "recognition" => Section::Recognition,
            "window" => Section::Window,
            "language" => Section::Language,
            "behavior" => Section::Behavior,
            _ => Section::Hotkeys,
        };
        let scene = ui.frame(size, 1.);
        let target = scene
            .hits
            .iter()
            .find(|hit| hit.target == Target::Section(section))
            .unwrap();
        ui.update(Event::PointerDown(Point {
            x: target.rect.x + target.rect.width / 2.,
            y: target.rect.y + target.rect.height / 2.,
        }));
        let scene = ui.frame(size, 1.);
        compare(
            scene.content,
            &reference["content"],
            reference["source"].as_str().unwrap(),
        );
        for (card_index, card) in reference["cards"].as_array().unwrap().iter().enumerate() {
            let kind = GeometryKind::Card(Section::for_page(page)[card_index]);
            let actual = scene
                .geometry
                .iter()
                .find(|node| node.kind == kind)
                .unwrap()
                .rect;
            let label = format!("{}/{kind:?}", reference["source"].as_str().unwrap());
            compare(actual, &card["frame"], &label);
            compared += 1;
            for (row_index, row) in card["rows"].as_array().unwrap().iter().enumerate() {
                let targets = if card_index == 0 {
                    vec![
                        Target::Hotkey(row_index),
                        Target::ResetHotkey(row_index),
                        Target::ClearHotkey(row_index),
                    ]
                } else if page == Page::Ocr {
                    vec![Target::Ocr(
                        OcrField::ALL[if card_index == 1 {
                            row_index
                        } else {
                            5 + row_index
                        }],
                    )]
                } else {
                    vec![Target::Translation(
                        TranslationField::ALL[match card_index {
                            1 => row_index,
                            2 => 3 + row_index,
                            _ => 9 + row_index,
                        }],
                    )]
                };
                for (control_index, target) in targets.into_iter().enumerate() {
                    let actual = scene
                        .geometry
                        .iter()
                        .find(|node| node.kind == GeometryKind::Control(target))
                        .unwrap()
                        .rect;
                    compare(
                        actual,
                        &row["controls"][control_index],
                        &format!("{label}/{target:?}"),
                    );
                    compared += 1;
                }
            }
        }
    }
    assert_eq!(compared, 2028);
}
