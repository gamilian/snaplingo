//! Render the shared result surface without a platform window.
use snaplingo_render::{Renderer, RendererTextMetrics};
use snaplingo_ui::{result::*, Size, Snapshot, Theme};
fn main() -> Result<(), String> {
    let directory = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/snaplingo-result".into());
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let mut renderer = Renderer::default();
    let mut metrics = RendererTextMetrics::default();
    for (name, theme, dark, mode, scale) in [
        (
            "translation",
            Theme::Native,
            false,
            ResultMode::Translation,
            1.,
        ),
        (
            "translation-dark",
            Theme::Studio,
            true,
            ResultMode::Translation,
            1.,
        ),
        (
            "translation-175",
            Theme::Paper,
            false,
            ResultMode::Translation,
            1.75,
        ),
        ("ocr", Theme::Native, false, ResultMode::Ocr, 1.),
        ("ocr-image", Theme::Native, false, ResultMode::Ocr, 1.),
        ("ocr-image-dark", Theme::Studio, true, ResultMode::Ocr, 1.),
        ("ocr-image-175", Theme::Paper, false, ResultMode::Ocr, 1.75),
    ] {
        let mut model = ResultSnapshot {
            shell: Snapshot {
                theme,
                dark,
                ..Snapshot::default()
            },
            mode,
            source: "The best way to predict the future is to create it.\n创造未来。".into(),
            source_language: "auto".into(),
            detected_language: Some("en".into()),
            target_language: "zh-CN".into(),
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
            ..ResultSnapshot::default()
        };
        if name.starts_with("ocr-image") {
            let image = tiny_skia::Pixmap::decode_png(include_bytes!(
                "../../ui/tests/fixtures/library-preview.png"
            ))
            .unwrap();
            model.ocr_image = snaplingo_ui::RasterImage::from_rgba(
                image.width(),
                image.height(),
                image.data().to_vec(),
            );
            model.ocr_text =
                "SnapLingo 文字识别。\nEmail: support@example.com\n预约时间 2026-11-09 14:30"
                    .into();
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
        }
        let mut ui = ResultUi::new(model);
        let frame = ui.frame(
            Size {
                width: 660. * scale,
                height: 780. * scale,
            },
            scale,
            &mut metrics,
        );
        renderer
            .render(&frame.scene, 1.)?
            .save_png(format!("{directory}/{name}.png"))?;
    }
    println!("{directory}");
    Ok(())
}
