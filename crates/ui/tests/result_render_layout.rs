//! Canvas presentation follows independently recorded AppKit control geometry.
use snaplingo_ui::render::{Renderer, RendererTextMetrics};
use snaplingo_ui::{result::*, DrawCommand, Point, Rect, Size, Theme};

fn translation() -> ResultSnapshot {
    ResultSnapshot {
        visible: true,
        source: "Hello from the native result window.\n第二行原文".into(),
        source_language: "auto".into(),
        target_language: "zh-CN".into(),
        detected_language: Some("en".into()),
        rows: vec![
            ResultRow {
                id: "google".into(),
                name: "Google Translate".into(),
                status: RowStatus::Success,
                text: "来自原生结果窗口的问候。\n第二行译文".into(),
                error: None,
                favorite: false,
                retryable: true,
            },
            ResultRow {
                id: "deeplx".into(),
                name: "DeepL".into(),
                status: RowStatus::Pending,
                text: String::new(),
                error: None,
                favorite: false,
                retryable: true,
            },
            ResultRow {
                id: "failed".into(),
                name: "失败的服务".into(),
                status: RowStatus::Error,
                text: String::new(),
                error: Some("测试错误：未配置翻译服务".into()),
                favorite: false,
                retryable: false,
            },
        ],
        ..Default::default()
    }
}
fn ocr() -> ResultSnapshot {
    let image =
        tiny_skia::Pixmap::decode_png(include_bytes!("fixtures/library-preview.png")).unwrap();
    ResultSnapshot {
        visible: true,
        mode: ResultMode::Ocr,
        ocr_text: "SnapLingo 文字识别。\nEmail: support@example.com\n预约时间 2026-11-09 14:30"
            .into(),
        ocr_language: Some("zh-CN".into()),
        ocr_confidence: Some(0.98),
        ocr_image: snaplingo_ui::RasterImage::from_rgba(
            image.width(),
            image.height(),
            image.data().to_vec(),
        ),
        ocr_tokens: vec![
            ResultCopyToken {
                label: "邮箱".into(),
                value: "support@example.com".into(),
            },
            ResultCopyToken {
                label: "时间".into(),
                value: "2026-11-09 14:30".into(),
            },
        ],
        ..Default::default()
    }
}
fn frame(ui: &mut ResultUi, width: f32, height: f32, scale: f32) -> ResultFrame {
    ui.frame(
        Size {
            width: width * scale,
            height: height * scale,
        },
        scale,
        &mut RendererTextMetrics::default(),
    )
}
fn compare(actual: Rect, expected: &serde_json::Value, scale: f32, x: f32, y: f32) {
    for (index, actual) in [
        actual.x / scale - x,
        actual.y / scale - y,
        actual.width / scale,
        actual.height / scale,
    ]
    .into_iter()
    .enumerate()
    {
        let expected = expected[index].as_f64().unwrap() as f32;
        assert!(
            (actual - expected).abs() <= 0.5,
            "coordinate {index}: {actual} != {expected}"
        );
    }
}
#[test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "Exact text heights use the macOS font baseline"
)]
fn rendered_translation_controls_match_native_baseline_at_normal_and_fractional_scale() {
    let references: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/native-result-layout.json")).unwrap();
    let reference = &references["result-translation"];
    for scale in [1., 1.75] {
        let mut ui = ResultUi::new(translation());
        let frame = frame(&mut ui, 660., 514., scale);
        for (target, expected, x, y) in [
            (ResultTarget::Pin, &reference["pin"], 0., 0.),
            (ResultTarget::Close, &reference["close"], 0., 0.),
            (ResultTarget::Source, &reference["source"]["text"], 12., 60.),
            (
                ResultTarget::SourceLanguage,
                &reference["source"]["languages"][0],
                12.,
                60.,
            ),
            (
                ResultTarget::ResultText("google".into()),
                &reference["rows"][0]["text"],
                12.,
                60. + reference["rows"][0]["frame"][1].as_f64().unwrap() as f32,
            ),
        ] {
            let control = frame
                .controls
                .iter()
                .find(|control| control.target == target)
                .unwrap();
            compare(control.rect, expected, scale, x, y);
        }
        for (actual, expected) in frame
            .cards
            .iter()
            .zip(reference["rows"].as_array().unwrap())
        {
            compare(*actual, &expected["frame"], scale, 12., 60.);
        }
        for (id, expected) in ["google", "deeplx", "failed"]
            .iter()
            .zip(reference["rows"].as_array().unwrap())
        {
            for (index, target) in [
                ResultTarget::Retry((*id).into()),
                ResultTarget::SpeakResult((*id).into()),
                ResultTarget::FavoriteResult((*id).into()),
                ResultTarget::CopyResult((*id).into()),
            ]
            .into_iter()
            .enumerate()
            {
                let control = frame
                    .controls
                    .iter()
                    .find(|control| control.target == target)
                    .unwrap();
                assert_eq!(
                    control.enabled,
                    expected["buttons"][index]["enabled"].as_bool().unwrap()
                );
            }
        }
    }
}
#[test]
fn ocr_popup_copies_original_values_and_scrolling_keeps_hit_targets_clipped() {
    for scale in [1., 1.75] {
        let mut ui = ResultUi::new(ocr());
        let first = frame(&mut ui, 660., 400., scale);
        assert!(first.content_height > 400. * scale);
        ui.update(ResultInput::Scroll(10000.));
        let scrolled = frame(&mut ui, 660., 400., scale);
        let fragments = scrolled
            .controls
            .iter()
            .find(|control| control.target == ResultTarget::CopyFragments)
            .unwrap();
        assert!(fragments.hit_rect.height > 0.);
        ui.update(ResultInput::PointerDown(Point {
            x: fragments.hit_rect.x + 5.,
            y: fragments.hit_rect.y + 5.,
        }));
        let menu = frame(&mut ui, 660., 400., scale);
        let token = menu
            .controls
            .iter()
            .find(|control| control.target == ResultTarget::CopyToken("support@example.com".into()))
            .unwrap();
        assert_eq!(
            ui.update(ResultInput::PointerDown(Point {
                x: token.hit_rect.x + 5.,
                y: token.hit_rect.y + 5.
            })),
            [ResultIntent::Copy("support@example.com".into())]
        );
        for control in &scrolled.controls {
            let header = matches!(
                control.target,
                ResultTarget::Header | ResultTarget::Pin | ResultTarget::Close
            );
            if !header && control.hit_rect.height > 0. {
                assert!(control.hit_rect.y >= scrolled.viewport.y);
                assert!(
                    control.hit_rect.y + control.hit_rect.height
                        <= scrolled.viewport.y + scrolled.viewport.height + 0.01
                );
            }
        }
        let image = scrolled
            .scene
            .commands
            .iter()
            .find_map(|command| {
                if let DrawCommand::Image { rect, .. } = command {
                    Some(rect)
                } else {
                    None
                }
            })
            .unwrap();
        assert!((image.width / image.height - 160. / 90.).abs() < 0.001);
    }
}
#[test]
fn six_themes_render_both_modes_and_keep_native_narrow_control_rows() {
    let mut renderer = Renderer::default();
    for theme in [Theme::Native, Theme::Studio, Theme::Paper] {
        for dark in [false, true] {
            for mut model in [translation(), ocr()] {
                model.appearance.theme = theme;
                model.appearance.dark = dark;
                let mut ui = ResultUi::new(model);
                let frame = frame(&mut ui, 300., 650., 1.);
                let pixels = renderer.render(&frame.scene, 1.).unwrap();
                assert_eq!((pixels.width(), pixels.height()), (300, 650));
                assert!(frame
                    .controls
                    .iter()
                    .all(|control| control.rect.width >= 0. && control.rect.x >= 0.));
                if ui.snapshot().mode == ResultMode::Translation {
                    let source = frame
                        .controls
                        .iter()
                        .find(|control| control.target == ResultTarget::Source)
                        .unwrap();
                    let copy = frame
                        .controls
                        .iter()
                        .find(|control| control.target == ResultTarget::CopySource)
                        .unwrap();
                    assert!(copy.rect.y >= source.rect.y + source.rect.height + 28.);
                    assert_eq!(copy.rect.height, 30.);
                }
            }
        }
    }
}

#[test]
fn long_fragment_popup_keeps_keyboard_and_wheel_targets_inside_the_window() {
    let mut model = ocr();
    model.ocr_tokens = (0..30)
        .map(|index| ResultCopyToken {
            label: "编号".into(),
            value: format!("fixture-{index}"),
        })
        .collect();
    let mut ui = ResultUi::new(model);
    frame(&mut ui, 660., 400., 1.);
    ui.update(ResultInput::Scroll(10000.));
    frame(&mut ui, 660., 400., 1.);
    ui.update(ResultInput::Activate(ResultTarget::CopyFragments));
    frame(&mut ui, 660., 400., 1.);
    ui.update(ResultInput::Key(
        snaplingo_ui::Key::ArrowUp,
        Default::default(),
    ));
    let last = frame(&mut ui, 660., 400., 1.);
    let target = ResultTarget::CopyToken("fixture-29".into());
    let control = last
        .controls
        .iter()
        .find(|control| control.target == target)
        .unwrap();
    assert!(control.hit_rect.height >= 27.);
    assert!(control.hit_rect.y + control.hit_rect.height <= 400.);
    assert_eq!(
        ui.update(ResultInput::Key(
            snaplingo_ui::Key::Enter,
            Default::default()
        )),
        [ResultIntent::Copy("fixture-29".into())]
    );
    ui.update(ResultInput::Activate(ResultTarget::CopyFragments));
    frame(&mut ui, 660., 400., 1.);
    ui.update(ResultInput::Scroll(10000.));
    let last = frame(&mut ui, 660., 400., 1.);
    assert!(
        last.controls
            .iter()
            .find(|control| control.target == target)
            .unwrap()
            .hit_rect
            .height
            >= 27.
    );
}
