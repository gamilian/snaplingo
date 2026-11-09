//! The retained React views provide the expected rectangles independently of Rust.
use snaplingo_render::RendererTextMetrics;
use snaplingo_ui::{library::*, Rect, Size, Theme};
mod support {
    pub mod library_fixture;
}

fn expected(value: &serde_json::Value) -> Rect {
    Rect {
        x: value["x"].as_f64().unwrap() as f32,
        y: value["y"].as_f64().unwrap() as f32,
        width: value["width"].as_f64().unwrap() as f32,
        height: value["height"].as_f64().unwrap() as f32,
    }
}
fn target(name: &str, key: ItemKey) -> LibraryTarget {
    match name {
        "search" => LibraryTarget::Search,
        "filter-all" => LibraryTarget::Filter(LibraryFilter::All),
        "filter-translation" => LibraryTarget::Filter(LibraryFilter::Translation),
        "filter-ocr" => LibraryTarget::Filter(LibraryFilter::Ocr),
        "filter-screenshot" => LibraryTarget::Filter(LibraryFilter::Screenshot),
        "previous" => LibraryTarget::PreviousPage,
        "next" => LibraryTarget::NextPage,
        "clear" => LibraryTarget::ClearHistory,
        "delete" => LibraryTarget::Delete(key),
        "unfavorite" => LibraryTarget::Unfavorite(key),
        "note" => LibraryTarget::Note(key),
        "tags" => LibraryTarget::Tags(key),
        "copy-source" => LibraryTarget::Copy(key, CopyPart::Source),
        "copy-ocr" => LibraryTarget::Copy(key, CopyPart::Ocr),
        "rerun" => LibraryTarget::RerunOcr(key),
        "copy-image" => LibraryTarget::CopyScreenshot(key),
        "reveal-image" => LibraryTarget::RevealScreenshot(key),
        _ => {
            if let Some(index) = name.strip_prefix("copy-result-") {
                LibraryTarget::Copy(key, CopyPart::Result(index.parse().unwrap()))
            } else if let Some(index) = name.strip_prefix("favorite-") {
                LibraryTarget::Favorite(key, index.parse().unwrap())
            } else {
                panic!("Unknown reference control {name}")
            }
        }
    }
}

#[test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "WKWebView geometry was captured with macOS system fonts"
)]
fn library_matches_60_real_react_scenes_with_actual_text_layout() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../ui/tests/fixtures/react-library-layout.json"
    ))
    .unwrap();
    let mut metrics = RendererTextMetrics::default();
    let mut differences = vec![];
    let mut count = 0;
    for reference in fixture["scenes"].as_array().unwrap() {
        let theme = match reference["theme"].as_str().unwrap() {
            "studio" => Theme::Studio,
            "paper" => Theme::Paper,
            _ => Theme::Native,
        };
        let kind = if reference["page"] == "history" {
            LibraryKind::History
        } else {
            LibraryKind::Favorites
        };
        let mut ui = LibraryUi::new(support::library_fixture::fixture(
            kind,
            theme,
            reference["dark"].as_bool().unwrap(),
        ));
        let size = Size {
            width: reference["width"].as_f64().unwrap() as f32,
            height: 650.,
        };
        ui.frame(size, 1., &mut metrics);
        let index = match reference["selection"].as_str().unwrap() {
            "ocr" => 1,
            "screenshot" => 2,
            _ => 0,
        };
        let key = ui.snapshot().items[index].key;
        ui.update(LibraryInput::Activate(LibraryTarget::Item(key)));
        let frame = ui.frame(size, 1., &mut metrics);
        let mut compare = |actual: Rect, value: &serde_json::Value, label: &str| {
            let expected = expected(value);
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
        compare(frame.scene.content, &reference["content"], "content");
        for (index, row) in reference["rows"].as_array().unwrap().iter().enumerate() {
            let key = ui.snapshot().items[index].key;
            let control = frame
                .controls
                .iter()
                .find(|c| c.target == LibraryTarget::Item(key))
                .unwrap();
            compare(control.rect, row, &format!("row{index}"));
        }
        for control in reference["controls"].as_array().unwrap() {
            let name = control["target"].as_str().unwrap();
            let target = target(name, key);
            let actual = frame
                .controls
                .iter()
                .find(|c| c.target == target)
                .unwrap_or_else(|| panic!("{} missing {target:?}", reference["source"]));
            compare(actual.rect, &control["frame"], name);
            assert_eq!(
                actual.enabled,
                control["enabled"].as_bool().unwrap(),
                "{target:?}"
            );
        }
    }
    assert_eq!(fixture["scenes"].as_array().unwrap().len(), 60);
    assert!(count > 2000);
    assert!(
        differences.is_empty(),
        "{} differences among {count} rectangles:\n{}",
        differences.len(),
        differences[..differences.len().min(35)].join("\n")
    );
}
