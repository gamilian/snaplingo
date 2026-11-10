use super::*;
use crate::{FontFamily, Key, Modifiers, Point, RasterImage, Size, TextMeasurement, TextMeasurer};

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: u16, _: FontFamily) -> TextMeasurement {
        use unicode_segmentation::UnicodeSegmentation;
        let mut x = 0.;
        let mut stops = text
            .grapheme_indices(true)
            .map(|(i, _)| {
                let p = (i, x);
                x += size * 0.6;
                p
            })
            .collect::<Vec<_>>();
        stops.push((text.len(), x));
        TextMeasurement { caret_stops: stops }
    }
}
fn key(kind: ContentKind, id: i64) -> ItemKey {
    ItemKey { kind, id }
}
fn translation(id: i64) -> LibraryItem {
    LibraryItem {
        key: key(ContentKind::Translation, id),
        relative_time: "刚刚".into(),
        date: "2026/11/08 12:00".into(),
        content: LibraryContent::Translation {
            source: "first\nsource".into(),
            source_language: "en".into(),
            target_language: "zh-CN".into(),
            results: vec![
                TranslationResult {
                    provider: "system".into(),
                    text: "译文".into(),
                    favorite: false,
                },
                TranslationResult {
                    provider: "other".into(),
                    text: "其他译文".into(),
                    favorite: true,
                },
            ],
        },
        thumbnail: None,
        note: None,
        tags: vec![],
    }
}
fn screenshot(id: i64) -> LibraryItem {
    LibraryItem {
        key: key(ContentKind::Screenshot, id),
        relative_time: "1 分钟前".into(),
        date: "2026/11/08 11:59".into(),
        content: LibraryContent::Screenshot {
            width: 2,
            height: 1,
        },
        thumbnail: RasterImage::from_rgba(2, 1, vec![255, 0, 0, 255, 0, 255, 0, 255]),
        note: Some("截图笔记".into()),
        tags: vec!["work".into()],
    }
}
fn frame(ui: &mut LibraryUi) -> LibraryFrame {
    ui.frame(
        Size {
            width: 1000.,
            height: 800.,
        },
        1.,
        &mut Metrics,
    )
}
fn press(ui: &mut LibraryUi, key: Key, modifiers: Modifiers) -> Vec<LibraryIntent> {
    ui.update(LibraryInput::KeyDown { key, modifiers })
}

#[test]
fn history_filters_queries_and_stale_responses_preserve_latest_state() {
    let mut ui = LibraryUi::new(LibrarySnapshot {
        kind: LibraryKind::History,
        items: vec![translation(3)],
        total: 1,
        ..Default::default()
    });
    frame(&mut ui);
    ui.update(LibraryInput::Activate(LibraryTarget::Search));
    let first = ui.update(LibraryInput::Text("first".into()));
    let second = ui.update(LibraryInput::Text(" query".into()));
    let rev = match second[0] {
        LibraryIntent::Query { revision, .. } => revision,
        _ => unreachable!(),
    };
    assert!(
        matches!(&first[0],LibraryIntent::Query{query,..}if query.search=="first"&&query.page==0)
    );
    ui.update(LibraryInput::Snapshot(LibrarySnapshot {
        kind: LibraryKind::History,
        revision: rev - 1,
        items: vec![translation(99)],
        ..Default::default()
    }));
    assert_eq!(ui.selected(), Some(key(ContentKind::Translation, 3)));
    ui.update(LibraryInput::Snapshot(LibrarySnapshot {
        kind: LibraryKind::History,
        revision: rev,
        query: ui.query().clone(),
        items: vec![translation(4)],
        total: 1,
        ..Default::default()
    }));
    assert_eq!(ui.selected(), Some(key(ContentKind::Translation, 4)));
    assert!(
        ui.text_edit().is_some(),
        "a response selecting another item must keep the search input active"
    );
    frame(&mut ui);
    assert!(!ui
        .frame
        .as_ref()
        .unwrap()
        .controls
        .iter()
        .any(|c| c.target == LibraryTarget::Filter(LibraryFilter::Screenshot)));
}

#[test]
fn a_replaced_page_rejects_old_frame_actions_and_pointer_text_drag_keeps_unicode() {
    let mut ui = LibraryUi::new(LibrarySnapshot {
        items: vec![screenshot(1)],
        total: 1,
        ..Default::default()
    });
    frame(&mut ui);
    let old = ui
        .frame
        .as_ref()
        .unwrap()
        .controls
        .iter()
        .find(|c| c.target == LibraryTarget::Delete(key(ContentKind::Screenshot, 1)))
        .unwrap()
        .hit_rect;
    ui.update(LibraryInput::Snapshot(LibrarySnapshot {
        items: vec![screenshot(9)],
        total: 1,
        ..Default::default()
    }));
    assert!(ui
        .update(LibraryInput::PointerDown(Point {
            x: old.x + 3.,
            y: old.y + 3.
        }))
        .is_empty());
    frame(&mut ui);
    ui.update(LibraryInput::Activate(LibraryTarget::Note(key(
        ContentKind::Screenshot,
        9,
    ))));
    press(
        &mut ui,
        Key::Character('a'),
        Modifiers {
            meta: true,
            ..Default::default()
        },
    );
    ui.update(LibraryInput::Text("中文\nemoji🙂".into()));
    frame(&mut ui);
    let editor = ui.editing.as_ref().unwrap();
    let text = editor.rect;
    let first = editor
        .layout
        .caret_stops
        .iter()
        .find(|(byte, _)| *byte == 0)
        .unwrap()
        .1;
    let last = editor.layout.caret_stops.last().unwrap().1;
    ui.update(LibraryInput::PointerDown(Point {
        x: text.x + first.x,
        y: text.y + first.y,
    }));
    ui.update(LibraryInput::PointerMove(Point {
        x: text.x + last.x,
        y: text.y + last.y,
    }));
    ui.update(LibraryInput::PointerUp);
    assert_eq!(ui.text_edit().unwrap().selected_text(), "中文\nemoji🙂");
}

#[test]
fn history_translation_actions_keep_per_result_membership_and_stable_keys() {
    let item = translation(2);
    let id = item.key;
    let mut ui = LibraryUi::new(LibrarySnapshot {
        kind: LibraryKind::History,
        items: vec![item],
        total: 1,
        ..Default::default()
    });
    frame(&mut ui);
    assert_eq!(
        ui.update(LibraryInput::Activate(LibraryTarget::Copy(
            id,
            CopyPart::Result(0)
        ))),
        vec![LibraryIntent::CopyText("译文".into())]
    );
    assert_eq!(
        ui.update(LibraryInput::Activate(LibraryTarget::Favorite(id, 0))),
        vec![LibraryIntent::Favorite { key: id, result: 0 }]
    );
    assert!(ui
        .update(LibraryInput::Activate(LibraryTarget::Favorite(id, 1)))
        .is_empty());
    assert!(ui
        .update(LibraryInput::Activate(LibraryTarget::Delete(key(
            ContentKind::Translation,
            999
        ))))
        .is_empty());
    assert_eq!(
        ui.update(LibraryInput::Activate(LibraryTarget::ClearHistory)),
        vec![LibraryIntent::ConfirmClearHistory(LibraryFilter::All)]
    );
}

#[test]
fn screenshot_uses_real_pixels_and_metadata_keeps_multiline_note_and_separate_tags() {
    let item = screenshot(1);
    let id = item.key;
    let mut ui = LibraryUi::new(LibrarySnapshot {
        items: vec![item],
        total: 1,
        ..Default::default()
    });
    let f = frame(&mut ui);
    assert!(f.page.scene.commands.iter().any(
        |c| matches!(c,crate::DrawCommand::Image{image,..}if image.width()==2&&image.height()==1)
    ));
    assert_eq!(
        ui.update(LibraryInput::Activate(LibraryTarget::CopyScreenshot(id))),
        vec![LibraryIntent::CopyScreenshot(id)]
    );
    ui.update(LibraryInput::Activate(LibraryTarget::Note(id)));
    press(
        &mut ui,
        Key::Character('a'),
        Modifiers {
            meta: true,
            ..Default::default()
        },
    );
    ui.update(LibraryInput::Text("第一行\nsecond🙂".into()));
    let actions = ui.update(LibraryInput::Activate(LibraryTarget::Tags(id)));
    assert_eq!(
        actions,
        vec![LibraryIntent::SaveMetadata {
            key: id,
            note: Some("第一行\nsecond🙂".into()),
            tags: vec!["work".into()]
        }]
    );
    press(
        &mut ui,
        Key::Character('a'),
        Modifiers {
            meta: true,
            ..Default::default()
        },
    );
    ui.update(LibraryInput::Text("a, , 中文".into()));
    assert_eq!(
        ui.update(LibraryInput::FocusLost),
        vec![LibraryIntent::SaveMetadata {
            key: id,
            note: Some("第一行\nsecond🙂".into()),
            tags: vec!["a".into(), "中文".into()]
        }]
    );
}

#[test]
fn scaled_hit_regions_and_independent_scroll_panels_do_not_overlap_navigation() {
    let mut ui = LibraryUi::new(LibrarySnapshot {
        items: (1..=20).map(translation).collect(),
        total: 40,
        ..Default::default()
    });
    let f = ui.frame(
        Size {
            width: 1400.,
            height: 1000.,
        },
        1.5,
        &mut Metrics,
    );
    let next = f
        .controls
        .iter()
        .find(|c| c.target == LibraryTarget::NextPage)
        .unwrap();
    let p = Point {
        x: next.hit_rect.x + 3.,
        y: next.hit_rect.y + 3.,
    };
    assert_eq!(f.hit_test(p), Some(LibraryTarget::NextPage));
    let list = ui.list_view;
    let detail = ui.detail_view;
    ui.update(LibraryInput::Scroll {
        position: Point {
            x: (list.x + 5.) * 1.5,
            y: (list.y + 5.) * 1.5,
        },
        delta_y: 200.,
    });
    assert!(ui.list_scroll > 0.);
    assert_eq!(ui.detail_scroll, 0.);
    let before = ui.list_scroll;
    ui.update(LibraryInput::Scroll {
        position: Point {
            x: (detail.x + 5.) * 1.5,
            y: (detail.y + 5.) * 1.5,
        },
        delta_y: 200.,
    });
    assert_eq!(ui.list_scroll, before);
    assert!(ui
        .update(LibraryInput::PointerDown(Point { x: 0., y: 0. }))
        .is_empty());
}
