use snaplingo_ui::{library::*, RasterImage, Snapshot, Theme};

pub fn fixture(kind: LibraryKind, theme: Theme, dark: bool) -> LibrarySnapshot {
    let preview = tiny_skia::Pixmap::decode_png(include_bytes!(
        "../../../snaplingo-ui/tests/fixtures/library-preview.png"
    ))
    .unwrap();
    let image =
        RasterImage::from_rgba(preview.width(), preview.height(), preview.data().to_vec()).unwrap();
    let mut items = vec![];
    for id in 1..=45 {
        let content = if id == 2 {
            LibraryContent::Ocr {
                text: "截图中的示例文字。\n第二行 OCR 结果。".into(),
                language: None,
                provider: "system".into(),
                confidence: Some(0.98),
                can_rerun: true,
                favorite: false,
            }
        } else if id == 3 && kind == LibraryKind::Favorites {
            LibraryContent::Screenshot {
                width: 160,
                height: 90,
            }
        } else {
            LibraryContent::Translation {
                source: if id == 1 {
                    "Library reference text.\nA second line for wrapping.".into()
                } else {
                    format!(
                        "Saved reference {}",
                        if kind == LibraryKind::Favorites && id > 3 {
                            id - 1
                        } else {
                            id
                        }
                    )
                },
                source_language: "en".into(),
                target_language: "zh-CN".into(),
                results: if kind == LibraryKind::History {
                    vec![
                        TranslationResult {
                            provider: "system".into(),
                            text: "这是一段用于资料库外观验收的译文。".into(),
                            favorite: false,
                        },
                        TranslationResult {
                            provider: "other".into(),
                            text: "第二个翻译服务的结果。".into(),
                            favorite: true,
                        },
                    ]
                } else {
                    vec![TranslationResult {
                        provider: "system".into(),
                        text: "这是一段用于资料库外观验收的译文。".into(),
                        favorite: false,
                    }]
                },
            }
        };
        let content_kind = match content {
            LibraryContent::Translation { .. } => ContentKind::Translation,
            LibraryContent::Ocr { .. } => ContentKind::Ocr,
            LibraryContent::Screenshot { .. } => ContentKind::Screenshot,
        };
        let minute = if kind == LibraryKind::Favorites && id > 3 {
            id - 2
        } else {
            id - 1
        };
        items.push(LibraryItem {
            key: ItemKey {
                kind: content_kind,
                id: if kind == LibraryKind::Favorites && id > 3 {
                    id - 1
                } else {
                    id
                },
            },
            relative_time: if minute == 0 {
                "刚刚".into()
            } else {
                format!("{minute} 分钟前")
            },
            date: format!(
                "2026/11/08 {}:{:02}",
                if minute == 0 { "12" } else { "11" },
                if minute == 0 { 0 } else { 60 - minute }
            ),
            thumbnail: if content_kind != ContentKind::Translation {
                Some(image.clone())
            } else {
                None
            },
            content,
            note: if kind == LibraryKind::Favorites {
                Some(if id == 3 {
                    "收藏截图参考".into()
                } else {
                    "参考笔记\n保留第二行。".into()
                })
            } else {
                None
            },
            tags: if kind == LibraryKind::Favorites {
                vec!["work".into(), "中文".into()]
            } else {
                vec![]
            },
        });
    }
    items.truncate(20);
    LibrarySnapshot {
        shell: Snapshot {
            theme,
            dark,
            ..Default::default()
        },
        kind,
        items,
        total: 45,
        ..Default::default()
    }
}
