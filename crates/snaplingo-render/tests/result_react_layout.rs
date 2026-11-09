//! Geometry captured from the retained React ResultWindow in WKWebView.
use snaplingo_render::RendererTextMetrics;
use snaplingo_ui::{result::*, Rect, Size, Snapshot};
fn snapshot(mode: ResultMode) -> ResultSnapshot {
    ResultSnapshot {
        shell: Snapshot::default(),
        mode,
        source: "The best way to predict the future is to create it.\n创造未来。".into(),
        source_language: "auto".into(),
        target_language: "zh-CN".into(),
        detected_language: Some("en".into()),
        ocr_text: "SnapLingo 文字识别\n联系邮箱 support@example.com\n预约时间 2026-11-09 14:30"
            .into(),
        ocr_language: Some("zh-CN".into()),
        ocr_confidence: Some(0.98),
        rows: vec![
            ResultRow {
                id: "google".into(),
                name: "Google 翻译".into(),
                status: RowStatus::Success,
                text: "预测未来的最好方式就是创造未来。".into(),
                error: None,
                favorite: false,
                retryable: true,
            },
            ResultRow {
                id: "local".into(),
                name: "本地翻译服务".into(),
                status: RowStatus::Pending,
                text: String::new(),
                error: None,
                favorite: false,
                retryable: true,
            },
            ResultRow {
                id: "deeplx".into(),
                name: "DeepL".into(),
                status: RowStatus::Error,
                text: String::new(),
                error: Some("网络连接失败，请重试".into()),
                favorite: false,
                retryable: true,
            },
        ],
        ..Default::default()
    }
}
fn compare(actual: Rect, reference: &serde_json::Value, label: &str) {
    for (key, value) in [
        ("x", actual.x),
        ("y", actual.y),
        ("width", actual.width),
        ("height", actual.height),
    ] {
        let expected = reference[key].as_f64().unwrap() as f32;
        assert!(
            (value - expected).abs() <= 0.5,
            "{label}.{key}: expected {expected}, got {value}"
        );
    }
}
#[test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "WKWebView geometry was captured with macOS system fonts"
)]
fn translation_and_ocr_match_retained_react_major_geometry() {
    for (mode, json) in [
        (
            ResultMode::Translation,
            include_str!("fixtures/react-result-translation.json"),
        ),
        (
            ResultMode::Ocr,
            include_str!("fixtures/react-result-ocr.json"),
        ),
    ] {
        let reference: serde_json::Value = serde_json::from_str(json).unwrap();
        let mut ui = ResultUi::new(snapshot(mode));
        let frame = ui.frame(
            Size {
                width: 660.,
                height: 780.,
            },
            1.,
            &mut RendererTextMetrics::default(),
        );
        compare(frame.panel, &reference["panel"], "panel");
        let target = if mode == ResultMode::Ocr {
            ResultTarget::OcrText
        } else {
            ResultTarget::Source
        };
        let control = frame.controls.iter().find(|c| c.target == target).unwrap();
        compare(control.rect, &reference["textareas"][0], "textarea");
        for (index, card) in frame.cards.iter().enumerate() {
            compare(*card, &reference["cards"][index], "card");
        }
        let actual: Vec<_> = frame
            .controls
            .iter()
            .filter(|c| {
                matches!(
                    c.target,
                    ResultTarget::Close
                        | ResultTarget::Pin
                        | ResultTarget::CopySource
                        | ResultTarget::SpeakSource
                        | ResultTarget::Translate
                        | ResultTarget::ClearSource
                        | ResultTarget::FavoriteAll
                        | ResultTarget::CopyOcr
                        | ResultTarget::SpeakOcr
                        | ResultTarget::FavoriteOcr
                        | ResultTarget::ClearOcr
                )
            })
            .collect();
        let buttons = reference["buttons"].as_array().unwrap();
        for control in actual {
            let expected = buttons
                .iter()
                .find(|b| b["label"].as_str() == Some(&control.label))
                .unwrap();
            compare(control.rect, &expected["frame"], &control.label);
        }
    }
}

#[test]
fn retained_theme_colors_and_scaled_panel_geometry_are_preserved() {
    for (theme, dark, scale, json) in [
        (
            snaplingo_ui::Theme::Studio,
            true,
            1.,
            include_str!("fixtures/react-result-studio.json"),
        ),
        (
            snaplingo_ui::Theme::Paper,
            false,
            1.75,
            include_str!("fixtures/react-result-paper.json"),
        ),
    ] {
        let reference: serde_json::Value = serde_json::from_str(json).unwrap();
        let mut model = snapshot(ResultMode::Translation);
        model.shell.theme = theme;
        model.shell.dark = dark;
        let mut ui = ResultUi::new(model);
        let frame = ui.frame(
            Size {
                width: 660. * scale,
                height: 780. * scale,
            },
            scale,
            &mut RendererTextMetrics::default(),
        );
        // CSS zoom snaps borders to WebKit device pixels. The native renderer
        // preserves logical geometry; allow the accumulated subpixel edge here.
        for (key, actual) in [
            ("x", frame.panel.x / scale),
            ("y", frame.panel.y / scale),
            ("width", frame.panel.width / scale),
            ("height", frame.panel.height / scale),
        ] {
            assert!(
                (actual - reference["panel"][key].as_f64().unwrap() as f32).abs() <= 2.,
                "{theme:?} {key}"
            );
        }
        let surface = frame
            .scene
            .commands
            .iter()
            .find_map(|command| {
                if let snaplingo_ui::DrawCommand::FillRect { rect, color, .. } = command {
                    (*rect == frame.panel).then_some(*color)
                } else {
                    None
                }
            })
            .unwrap();
        let expected = if dark {
            [37, 41, 44, 255]
        } else {
            [253, 252, 247, 255]
        };
        assert_eq!(surface, expected);
        assert_eq!(
            frame.cards.len(),
            reference["cards"].as_array().unwrap().len()
        );
    }
}

fn image_snapshot(theme: snaplingo_ui::Theme, dark: bool) -> ResultSnapshot {
    let mut model = snapshot(ResultMode::Ocr);
    model.shell.theme = theme;
    model.shell.dark = dark;
    let image = tiny_skia::Pixmap::decode_png(include_bytes!(
        "../../snaplingo-ui/tests/fixtures/library-preview.png"
    ))
    .unwrap();
    model.ocr_image =
        snaplingo_ui::RasterImage::from_rgba(image.width(), image.height(), image.data().to_vec());
    model.ocr_text =
        "SnapLingo 文字识别。\nEmail: support@example.com\n预约时间 2026-11-09 14:30".into();
    model.ocr_tokens = vec![
        ResultCopyToken {
            label: "邮箱".into(),
            value: "support@example.com".into(),
        },
        ResultCopyToken {
            label: "时间".into(),
            value: "2026-11-09 14:30".into(),
        },
    ];
    model
}
#[test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "WKWebView geometry was captured with macOS system fonts"
)]
fn ocr_image_fragments_match_retained_react_geometry_in_themes_and_zoom() {
    for (theme, dark, scale, json) in [
        (
            snaplingo_ui::Theme::Native,
            false,
            1.,
            include_str!("fixtures/react-result-ocr-image.json"),
        ),
        (
            snaplingo_ui::Theme::Studio,
            true,
            1.,
            include_str!("fixtures/react-result-ocr-image-dark.json"),
        ),
        (
            snaplingo_ui::Theme::Paper,
            false,
            1.75,
            include_str!("fixtures/react-result-ocr-image-175.json"),
        ),
    ] {
        let reference: serde_json::Value = serde_json::from_str(json).unwrap();
        let mut ui = ResultUi::new(image_snapshot(theme, dark));
        let frame = ui.frame(
            Size {
                width: 660. * scale,
                height: 780. * scale,
            },
            scale,
            &mut RendererTextMetrics::default(),
        );
        let logical = |rect: Rect| Rect {
            x: rect.x / scale,
            y: rect.y / scale,
            width: rect.width / scale,
            height: rect.height / scale,
        };
        compare(logical(frame.panel), &reference["panel"], "image panel");
        for control in frame.controls.iter().filter(|c| {
            matches!(
                c.target,
                ResultTarget::OcrText
                    | ResultTarget::CopyToken(_)
                    | ResultTarget::FileOcr
                    | ResultTarget::CopyOcr
                    | ResultTarget::ClearOcr
                    | ResultTarget::SpeakOcr
                    | ResultTarget::FavoriteOcr
            )
        }) {
            let expected = if control.target == ResultTarget::OcrText {
                &reference["textareas"][0]
            } else {
                &reference["buttons"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|b| b["label"].as_str() == Some(&control.label))
                    .unwrap()["frame"]
            };
            if scale == 1. {
                compare(logical(control.rect), expected, &control.label);
            } else {
                let rect = logical(control.rect);
                for (key, value) in [
                    ("x", rect.x),
                    ("y", rect.y),
                    ("width", rect.width),
                    ("height", rect.height),
                ] {
                    assert!(
                        (value - expected[key].as_f64().unwrap() as f32).abs() <= 2.,
                        "zoom {} {key}: {} vs {}",
                        control.label,
                        value,
                        expected[key]
                    );
                }
            }
        }
        let image = frame
            .scene
            .commands
            .iter()
            .find_map(|command| {
                if let snaplingo_ui::DrawCommand::Image { rect, image } = command {
                    Some((logical(*rect), image))
                } else {
                    None
                }
            })
            .unwrap();
        assert!(
            (image.0.width / image.0.height - 160. / 90.).abs() < 0.001,
            "preview preserves original aspect ratio"
        );
        let css = &reference["images"][0]["frame"];
        let height = css["height"].as_f64().unwrap() as f32;
        let width = height * 160. / 90.;
        assert!(
            (image.0.x
                - (css["x"].as_f64().unwrap() as f32
                    + (css["width"].as_f64().unwrap() as f32 - width) / 2.))
                .abs()
                < 0.5
        );
    }
}

#[test]
fn scaled_scrolling_clips_the_image_and_keeps_fragment_hit_targets_aligned() {
    for (scale, json) in [
        (
            1.,
            include_str!("fixtures/react-result-ocr-image-scrolled.json"),
        ),
        (
            1.75,
            include_str!("fixtures/react-result-ocr-image-175-scrolled.json"),
        ),
    ] {
        let mut ui = ResultUi::new(image_snapshot(snaplingo_ui::Theme::Native, false));
        let size = Size {
            width: 660. * scale,
            height: 500. * scale,
        };
        let mut metrics = RendererTextMetrics::default();
        ui.frame(size, scale, &mut metrics);
        ui.update(ResultInput::Scroll(10000.));
        let frame = ui.frame(size, scale, &mut metrics);
        let token = frame
            .controls
            .iter()
            .find(|c| c.target == ResultTarget::CopyToken("support@example.com".into()))
            .unwrap();
        // Token widths and the resulting scroll extent depend on system fonts.
        // Keep the native hit-test checks below active on every platform.
        if cfg!(target_os = "macos") {
            let reference: serde_json::Value = serde_json::from_str(json).unwrap();
            let expected = &reference["buttons"]
                .as_array()
                .unwrap()
                .iter()
                .find(|b| b["label"] == "复制邮箱: support@example.com")
                .unwrap()["frame"];
            let rect = Rect {
                x: token.rect.x / scale,
                y: token.rect.y / scale,
                width: token.rect.width / scale,
                height: token.rect.height / scale,
            };
            if scale == 1. {
                compare(rect, expected, "scrolled token");
            } else {
                assert!((rect.y - expected["y"].as_f64().unwrap() as f32).abs() < 2.);
            }
        }
        let point = snaplingo_ui::Point {
            x: token.hit_rect.x + token.hit_rect.width / 2.,
            y: token.hit_rect.y + token.hit_rect.height / 2.,
        };
        assert_eq!(
            ui.update(ResultInput::PointerDown(point)),
            [ResultIntent::Copy("support@example.com".into())]
        );
        assert!(!frame
            .controls
            .iter()
            .find(|c| c.target == ResultTarget::FileOcr)
            .unwrap()
            .hit_rect
            .contains(snaplingo_ui::Point {
                x: 20. * scale,
                y: 50. * scale
            }));
    }
}
