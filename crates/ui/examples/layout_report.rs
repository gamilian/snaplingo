//! Emit the same card/control geometry captured from the actual React page.
use snaplingo_ui::*;
fn rect(r: Rect) -> String {
    format!(
        "{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}}",
        r.x, r.y, r.width, r.height
    )
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::args().nth(1).unwrap_or("target/shared-ui".into());
    std::fs::create_dir_all(&directory)?;
    for (page, page_name) in [(Page::Ocr, "ocr"), (Page::Translation, "translation")] {
        for (theme, name) in [
            (Theme::Native, "native"),
            (Theme::Studio, "studio"),
            (Theme::Paper, "paper"),
        ] {
            for dark in [false, true] {
                for compact in [false, true] {
                    let mut ui = SettingsUi::new(Snapshot {
                        theme,
                        dark,
                        ..Snapshot::default()
                    });
                    let size = Size {
                        width: if compact { 700. } else { 900. },
                        height: 650.,
                    };
                    ui.update(Event::Activate(Target::Navigate(page)));
                    for &section in Section::for_page(page) {
                        let id = match section {
                            Section::Hotkeys => "hotkeys",
                            Section::Recognition => "recognition",
                            Section::Language => "language",
                            Section::Window => "window",
                            Section::Behavior => "behavior",
                            Section::Output => "output",
                            Section::CaptureInterface => "capture-interface",
                            Section::Interface => "interface",
                            Section::Permissions => "permissions",
                            Section::Network => "network",
                            Section::Maintenance => "maintenance",
                            Section::Experimental => "experimental",
                            Section::About => "about",
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
                        let mut cards = vec![];
                        for (card_index, section) in
                            Section::for_page(page).iter().copied().enumerate()
                        {
                            let frame = scene
                                .geometry
                                .iter()
                                .find(|g| g.kind == GeometryKind::Card(section))
                                .unwrap()
                                .rect;
                            let targets: Vec<_> = if page == Page::Ocr {
                                match section {
                                    Section::Recognition => {
                                        OcrField::ALL[..5].iter().map(|f| Target::Ocr(*f)).collect()
                                    }
                                    Section::Window => {
                                        OcrField::ALL[5..].iter().map(|f| Target::Ocr(*f)).collect()
                                    }
                                    _ => vec![],
                                }
                            } else {
                                match section {
                                    Section::Language => TranslationField::ALL[..3]
                                        .iter()
                                        .map(|f| Target::Translation(*f))
                                        .collect(),
                                    Section::Window => TranslationField::ALL[3..9]
                                        .iter()
                                        .map(|f| Target::Translation(*f))
                                        .collect(),
                                    Section::Behavior => TranslationField::ALL[9..]
                                        .iter()
                                        .map(|f| Target::Translation(*f))
                                        .collect(),
                                    _ => vec![],
                                }
                            };
                            let mut rows = vec![];
                            let row_count = if card_index == 0 { 3 } else { targets.len() };
                            for row in 0..row_count {
                                let targets = if card_index == 0 {
                                    vec![
                                        Target::Hotkey(row),
                                        Target::ResetHotkey(row),
                                        Target::ClearHotkey(row),
                                    ]
                                } else {
                                    vec![targets[row]]
                                };
                                let controls = targets
                                    .iter()
                                    .map(|target| {
                                        rect(
                                            scene
                                                .geometry
                                                .iter()
                                                .find(|g| g.kind == GeometryKind::Control(*target))
                                                .unwrap()
                                                .rect,
                                        )
                                    })
                                    .collect::<Vec<_>>()
                                    .join(",");
                                let label = if card_index == 0 {
                                    (if page == Page::Ocr {
                                        ["截图 OCR", "静默截图 OCR", "上传图片 OCR"]
                                    } else {
                                        ["划词翻译", "截图翻译", "显示翻译窗口"]
                                    })[row]
                                } else {
                                    match targets[0] {
                                        Target::Ocr(f) => f.label(),
                                        Target::Translation(f) => f.label(),
                                        _ => unreachable!(),
                                    }
                                };
                                rows.push(format!(
                                    "{{\"text\":\"{label}\",\"controls\":[{controls}]}}"
                                ));
                            }
                            cards.push(format!(
                                "{{\"frame\":{},\"rows\":[{}]}}",
                                rect(frame),
                                rows.join(",")
                            ));
                        }
                        let path = format!(
                            "{directory}/{page_name}-{name}{}{}-{id}.json",
                            if dark { "-dark" } else { "" },
                            if compact { "-compact" } else { "" }
                        );
                        std::fs::write(
                            path,
                            format!(
                                "{{\"content\":{},\"cards\":[{}]}}",
                                rect(scene.content),
                                cards.join(",")
                            ),
                        )?;
                    }
                }
            }
        }
    }
    println!("Wrote 84 shared Settings layout reports to {directory}");
    Ok(())
}
