//! AppKit frames recorded before the Result presentation was shared.
use serde_json::Value;
use snaplingo_ui::result::*;
use snaplingo_ui::{Rect, Size};

fn compare(actual: Rect, expected: &Value, label: &str) {
    let expected = expected.as_array().unwrap();
    for (index, actual) in [actual.x, actual.y, actual.width, actual.height]
        .into_iter()
        .enumerate()
    {
        let expected = expected[index].as_f64().unwrap() as f32;
        assert!(
            (actual - expected).abs() <= 0.5,
            "{label}[{index}]: {actual} != {expected}"
        );
    }
}

#[test]
fn layout_preserves_recorded_native_controls_in_all_seven_scenes() {
    let references: Value =
        serde_json::from_str(include_str!("fixtures/native-result-layout.json")).unwrap();
    for (name, reference) in references.as_object().unwrap() {
        let ocr = name.contains("ocr");
        let mut snapshot = ResultSnapshot {
            mode: if ocr {
                ResultMode::Ocr
            } else {
                ResultMode::Translation
            },
            ..Default::default()
        };
        let mut measured = ResultMeasurements::default();
        if ocr {
            let expected = &reference["ocr"];
            snapshot.ocr_language = expected.get("language").map(|_| "zh-CN".into());
            if expected.get("fragments").is_some() {
                snapshot.ocr_tokens.push(ResultCopyToken {
                    label: "邮箱".into(),
                    value: "support@example.com".into(),
                });
            }
            measured.ocr = expected["text"][3].as_f64().unwrap() as f32;
            measured.image = expected["image_size"].as_array().map(|size| Size {
                width: size[0].as_f64().unwrap() as f32,
                height: size[1].as_f64().unwrap() as f32,
            });
        } else {
            measured.source = reference["source"]["text"][3].as_f64().unwrap_or(0.) as f32;
            for row in reference["rows"].as_array().unwrap() {
                let id = row["id"].as_str().unwrap();
                snapshot.rows.push(ResultRow {
                    id: id.into(),
                    name: id.into(),
                    status: if id == "deeplx" {
                        RowStatus::Pending
                    } else if id == "failed" {
                        RowStatus::Error
                    } else {
                        RowStatus::Success
                    },
                    text: if id == "google" {
                        "来自原生结果窗口的问候。\n第二行译文".into()
                    } else {
                        String::new()
                    },
                    error: (id == "failed").then(|| "测试错误：未配置翻译服务".into()),
                    favorite: false,
                    retryable: id != "failed",
                });
                measured.rows.push(row["text"][3].as_f64().unwrap() as f32);
            }
        }
        let mut ui = ResultUi::new(snapshot);
        if !ocr {
            ui.restore_source_collapsed(reference["source"]["collapsed"].as_bool().unwrap());
            for row in reference["rows"].as_array().unwrap() {
                if row["expanded"] == false {
                    ui.update(ResultInput::Activate(ResultTarget::ToggleRow(
                        row["id"].as_str().unwrap().into(),
                    )));
                }
            }
        }
        let width = reference["width"].as_f64().unwrap() as f32;
        let height = reference["height"].as_f64().unwrap() as f32;
        let header = ResultLayout::header(Size { width, height });
        compare(header.pin, &reference["pin"], name);
        compare(header.close, &reference["close"], name);
        compare(header.title, &reference["title"], name);
        compare(header.subtitle, &reference["subtitle"], name);
        compare(header.viewport, &reference["scroll"], name);
        let layout = ui.layout(width, &measured);
        assert_eq!(
            layout.content_height,
            reference["document"][3].as_f64().unwrap() as f32,
            "{name} document"
        );
        if let Some(source) = &layout.source {
            let expected = &reference["source"];
            for (actual, key) in [
                (source.frame, "frame"),
                (source.text, "text"),
                (source.placeholder, "placeholder"),
                (source.metadata, "metadata"),
            ] {
                compare(actual, &expected[key], name);
            }
            for (rect, button) in source
                .buttons
                .iter()
                .zip(expected["buttons"].as_array().unwrap())
            {
                compare(*rect, &button["frame"], name);
            }
        }
        if let Some(rect) = layout.collapsed_source {
            compare(rect, &reference["source"]["toggle"], name);
        }
        if let Some(rects) = layout.languages {
            for (index, rect) in rects.iter().enumerate() {
                compare(*rect, &reference["source"]["languages"][index], name);
            }
        }
        for ((actual, expected), row) in layout
            .rows
            .iter()
            .zip(reference["rows"].as_array().into_iter().flatten())
            .zip(&ui.snapshot().rows)
        {
            for (rect, key) in [
                (actual.frame, "frame"),
                (actual.title, "title"),
                (actual.status, "status"),
                (actual.text, "text"),
            ] {
                compare(rect, &expected[key], name);
            }
            for ((rect, button), state) in actual
                .buttons
                .iter()
                .zip(expected["buttons"].as_array().unwrap())
                .zip(row.presentation(false).buttons)
            {
                compare(*rect, &button["frame"], name);
                assert_eq!(
                    state.enabled,
                    button["enabled"].as_bool().unwrap(),
                    "{name} {:?}",
                    state.target
                );
            }
        }
        if let Some(ocr) = &layout.ocr {
            let expected = &reference["ocr"];
            for (rect, key) in [
                (ocr.image_title, "image_title"),
                (ocr.upload, "upload"),
                (ocr.title, "title"),
                (ocr.frame, "frame"),
                (ocr.text, "text"),
                (ocr.placeholder, "placeholder"),
                (ocr.metadata, "metadata"),
            ] {
                compare(rect, &expected[key], name);
            }
            if let Some(rect) = ocr.language {
                compare(rect, &expected["language"], name);
            }
            if let Some(rect) = ocr.image {
                compare(rect, &expected["image"], name);
            }
            if expected.get("fragments").is_some() {
                compare(ocr.fragments, &expected["fragments"], name);
            }
            for (rect, button) in ocr
                .buttons
                .iter()
                .zip(expected["buttons"].as_array().unwrap())
            {
                compare(*rect, &button["frame"], name);
            }
        }
    }
}
