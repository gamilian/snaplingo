use super::*;
use crate::{FontFamily, Key, Modifiers, Size, TextMeasurement, TextMeasurer};
struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, _: f32, _: u16, _: FontFamily) -> TextMeasurement {
        TextMeasurement {
            caret_stops: text
                .char_indices()
                .map(|(i, _)| (i, i as f32 * 7.))
                .chain(std::iter::once((text.len(), text.len() as f32 * 7.)))
                .collect(),
        }
    }
}
fn snapshot() -> ResultSnapshot {
    ResultSnapshot {
        source: "hello".into(),
        source_language: "auto".into(),
        target_language: "zh-CN".into(),
        rows: vec![ResultRow {
            id: "fixture".into(),
            name: "Fixture".into(),
            status: RowStatus::Success,
            text: "你好世界".into(),
            error: None,
            favorite: false,
            retryable: true,
        }],
        ..ResultSnapshot::default()
    }
}
fn frame(ui: &mut ResultUi, scale: f32) -> ResultFrame {
    ui.frame(
        Size {
            width: 660. * scale,
            height: 720. * scale,
        },
        scale,
        &mut Metrics,
    )
}
#[test]
fn ime_draft_survives_controller_feedback_and_enter_waits_for_commit() {
    let mut ui = ResultUi::new(snapshot());
    frame(&mut ui, 1.);
    ui.update(ResultInput::Activate(ResultTarget::Source));
    ui.update(ResultInput::Key(
        Key::Character('a'),
        Modifiers {
            control: true,
            ..Modifiers::default()
        },
    ));
    assert_eq!(
        ui.update(ResultInput::Preedit("ni".into(), Some(2..2))),
        [ResultIntent::Edit("hello".into(), true)]
    );
    let mut feedback = snapshot();
    feedback.revision = 1;
    feedback.composing = true;
    ui.replace(feedback);
    assert!(ui.text_edit().unwrap().is_composing());
    assert!(ui
        .update(ResultInput::Key(Key::Enter, Modifiers::default()))
        .is_empty());
    assert!(ui
        .update(ResultInput::Key(Key::Escape, Modifiers::default()))
        .is_empty());
    assert_eq!(
        ui.update(ResultInput::Commit("你".into())),
        [ResultIntent::Edit("你".into(), false)]
    );
    assert_eq!(
        ui.update(ResultInput::Key(Key::Enter, Modifiers::default())),
        [ResultIntent::Translate]
    );
    assert_eq!(
        ui.update(ResultInput::Key(
            Key::Enter,
            Modifiers {
                shift: true,
                ..Modifiers::default()
            }
        )),
        [ResultIntent::Edit("你\n".into(), false)]
    );
}
#[test]
fn scaled_pointer_and_keyboard_copy_selected_result_without_editing_it() {
    let mut ui = ResultUi::new(snapshot());
    let frame = frame(&mut ui, 1.75);
    let field = frame
        .controls
        .iter()
        .find(|c| c.target == ResultTarget::ResultText("fixture".into()))
        .unwrap();
    ui.update(ResultInput::PointerDown(Point {
        x: field.rect.x + 2.,
        y: field.rect.y + 2.,
    }));
    ui.update(ResultInput::PointerUp);
    ui.update(ResultInput::Key(
        Key::Character('a'),
        Modifiers {
            control: true,
            ..Modifiers::default()
        },
    ));
    assert_eq!(
        ui.update(ResultInput::Key(
            Key::Character('c'),
            Modifiers {
                control: true,
                ..Modifiers::default()
            }
        )),
        [ResultIntent::Copy("你好世界".into())]
    );
    assert!(ui.update(ResultInput::Text("overwrite".into())).is_empty());
    assert_eq!(ui.snapshot().rows[0].text, "你好世界");
}
#[test]
fn stale_rows_and_disabled_favorites_are_rejected_and_ocr_actions_use_ocr_content() {
    let mut ui = ResultUi::new(snapshot());
    frame(&mut ui, 1.);
    assert_eq!(
        ui.update(ResultInput::Activate(ResultTarget::FavoriteResult(
            "fixture".into()
        ))),
        [ResultIntent::Favorite(vec!["fixture".into()])]
    );
    let mut next = snapshot();
    next.revision = 1;
    next.rows[0].favorite = true;
    ui.replace(next);
    frame(&mut ui, 1.);
    assert!(ui
        .update(ResultInput::Activate(ResultTarget::FavoriteResult(
            "fixture".into()
        )))
        .is_empty());
    let next = ResultSnapshot {
        revision: 2,
        mode: ResultMode::Ocr,
        ocr_text: "识别文本".into(),
        ..ResultSnapshot::default()
    };
    ui.replace(next);
    frame(&mut ui, 1.);
    assert!(ui
        .update(ResultInput::Activate(ResultTarget::CopyResult(
            "fixture".into()
        )))
        .is_empty());
    assert_eq!(
        ui.update(ResultInput::Activate(ResultTarget::CopyOcr)),
        [ResultIntent::Copy("识别文本".into())]
    );
    assert_eq!(
        ui.update(ResultInput::Activate(ResultTarget::FileOcr)),
        [ResultIntent::FileOcr]
    );
}
#[test]
fn language_menu_targets_the_correct_side_and_escape_dismisses_before_close() {
    let mut ui = ResultUi::new(snapshot());
    frame(&mut ui, 1.);
    ui.update(ResultInput::Activate(ResultTarget::TargetLanguage));
    frame(&mut ui, 1.);
    assert_eq!(
        ui.update(ResultInput::Activate(ResultTarget::Language(
            false,
            "ja".into()
        ))),
        [ResultIntent::TargetLanguage("ja".into())]
    );
    frame(&mut ui, 1.);
    ui.update(ResultInput::Activate(ResultTarget::SourceLanguage));
    frame(&mut ui, 1.);
    assert!(ui
        .update(ResultInput::Key(Key::Escape, Modifiers::default()))
        .is_empty());
    assert_eq!(
        ui.update(ResultInput::Key(Key::Escape, Modifiers::default())),
        [ResultIntent::Close]
    );
}

#[test]
fn retained_pin_source_and_provider_collapse_controls_keep_their_state() {
    let mut ui = ResultUi::new(snapshot());
    frame(&mut ui, 1.);
    ui.update(ResultInput::Activate(ResultTarget::Pin));
    assert!(ui.pinned());
    ui.update(ResultInput::Activate(ResultTarget::ToggleSource));
    let collapsed = frame(&mut ui, 1.);
    assert!(!collapsed
        .controls
        .iter()
        .any(|c| c.target == ResultTarget::Source));
    ui.update(ResultInput::Activate(ResultTarget::ToggleSource));
    frame(&mut ui, 1.);
    ui.update(ResultInput::Activate(ResultTarget::ToggleRow(
        "fixture".into(),
    )));
    let collapsed = frame(&mut ui, 1.);
    assert!(!collapsed
        .controls
        .iter()
        .any(|c| c.target == ResultTarget::ResultText("fixture".into())));
}

#[test]
fn ocr_image_copy_fragments_dispatch_the_original_token_value() {
    let mut ui = ResultUi::new(ResultSnapshot {
        mode: ResultMode::Ocr,
        ocr_text: "Email support@example.com".into(),
        ocr_image: crate::RasterImage::from_rgba(1, 1, vec![255; 4]),
        ocr_tokens: vec![ResultCopyToken {
            label: "邮箱".into(),
            value: "support@example.com".into(),
        }],
        ..Default::default()
    });
    frame(&mut ui, 1.);
    assert_eq!(
        ui.update(ResultInput::Activate(ResultTarget::CopyToken(
            "support@example.com".into()
        ))),
        [ResultIntent::Copy("support@example.com".into())]
    );
}

#[test]
fn native_and_canvas_actions_share_enabled_policy_without_requiring_a_frame() {
    let mut model = snapshot();
    model.source = " \n".into();
    model.rows[0].status = RowStatus::Pending;
    let mut ui = ResultUi::new(model.clone());
    for target in [
        ResultTarget::Translate,
        ResultTarget::CopySource,
        ResultTarget::Retry("fixture".into()),
        ResultTarget::FavoriteResult("fixture".into()),
    ] {
        assert!(ui.update(ResultInput::Activate(target)).is_empty());
    }
    assert_eq!(
        ui.update(ResultInput::Activate(ResultTarget::ClearSource)),
        [ResultIntent::Edit(String::new(), false)]
    );
    model.revision = 1;
    model.source = "hello".into();
    model.composing = true;
    model.rows[0].status = RowStatus::Error;
    model.rows[0].error = Some("Configure the provider".into());
    model.rows[0].retryable = false;
    ui.replace(model.clone());
    assert!(ui
        .update(ResultInput::Activate(ResultTarget::Translate))
        .is_empty());
    assert!(ui
        .update(ResultInput::Activate(ResultTarget::Retry("fixture".into())))
        .is_empty());
    model.revision = 2;
    model.composing = false;
    model.rows[0].retryable = true;
    ui.replace(model);
    assert_eq!(
        ui.update(ResultInput::Activate(ResultTarget::Translate)),
        [ResultIntent::Translate]
    );
    assert_eq!(
        ui.update(ResultInput::Activate(ResultTarget::Retry("fixture".into()))),
        [ResultIntent::Retry("fixture".into())]
    );
}

#[test]
fn fragment_popup_supports_keyboard_copy_escape_and_stale_value_rejection() {
    let mut ui = ResultUi::new(ResultSnapshot {
        visible: true,
        mode: ResultMode::Ocr,
        ocr_text: "support@example.com".into(),
        ocr_tokens: vec![ResultCopyToken {
            label: "邮箱".into(),
            value: "support@example.com".into(),
        }],
        ..Default::default()
    });
    ui.update(ResultInput::Activate(ResultTarget::CopyFragments));
    ui.update(ResultInput::Key(Key::ArrowDown, Modifiers::default()));
    assert_eq!(
        ui.focused(),
        Some(&ResultTarget::CopyToken("support@example.com".into()))
    );
    assert_eq!(
        ui.update(ResultInput::Key(Key::Enter, Modifiers::default())),
        [ResultIntent::Copy("support@example.com".into())]
    );
    ui.update(ResultInput::Activate(ResultTarget::CopyFragments));
    assert!(ui
        .update(ResultInput::Key(Key::Escape, Modifiers::default()))
        .is_empty());
    ui.replace(ResultSnapshot {
        revision: 1,
        mode: ResultMode::Ocr,
        ..Default::default()
    });
    assert!(ui
        .update(ResultInput::Activate(ResultTarget::CopyToken(
            "support@example.com".into()
        )))
        .is_empty());
}

#[test]
fn presentation_uses_native_unicode_counts_and_character_density() {
    let model = ResultSnapshot {
        mode: ResultMode::Ocr,
        source: "📷A".into(),
        ocr_text: "📷A".into(),
        ocr_confidence: Some(0.98),
        ..Default::default()
    };
    let view = model.presentation();
    assert_eq!(view.subtitle, "已识别 3 个字符 · 置信度 98%");
    assert_eq!(view.ocr_metadata, "3 chars");
    assert!(view.source_metadata.starts_with("3 chars"));
    assert_eq!(text_size("hello", true), 14.);
    assert_eq!(text_size(&"A".repeat(170), true), 13.);
    assert_eq!(text_size(&"A".repeat(350), true), 12.);
    assert_eq!(text_size(&"A".repeat(170), false), 14.);
}
