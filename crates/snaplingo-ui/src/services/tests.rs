use super::*;
use crate::{
    DrawCommand, FontFamily, Key, Modifiers, Point, Size, TextMeasurement, TextMeasurer, Theme,
};
struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: u16, _: FontFamily) -> TextMeasurement {
        let mut x = 0.;
        let mut caret_stops = vec![];
        for (byte, _) in text.char_indices() {
            caret_stops.push((byte, x));
            x += size * 0.6;
        }
        caret_stops.push((text.len(), x));
        TextMeasurement { caret_stops }
    }
}
fn frame(ui: &mut ServicesUi) -> ServicesFrame {
    ui.frame(
        Size {
            width: 900.,
            height: 650.,
        },
        1.,
        &mut Metrics,
    )
}
fn target(ui: &mut ServicesUi, control: ServiceControl) -> ServicesTarget {
    frame(ui)
        .controls
        .into_iter()
        .find(|item| item.target.control == control)
        .unwrap()
        .target
}
fn activate(ui: &mut ServicesUi, control: ServiceControl) -> Vec<ServicesIntent> {
    let target = target(ui, control);
    ui.update(ServicesInput::Activate(target))
}
fn input(ui: &mut ServicesUi, field: ServiceField, text: &str) -> Vec<ServicesIntent> {
    let target = target(ui, ServiceControl::Field(field));
    ui.update(ServicesInput::Focus(target));
    ui.update(ServicesInput::Selection(
        0..ui.text_edit().unwrap().text().len(),
    ));
    ui.update(ServicesInput::Text(text.into()))
}
fn provider(id: &str, active: bool, builtin: bool) -> ServiceProvider {
    ServiceProvider {
        id: id.into(),
        name: id.into(),
        active,
        builtin,
        configured: true,
        ..ServiceProvider::default()
    }
}
fn custom() -> ServicesUi {
    ServicesUi::new(ServicesSnapshot {
        route: ServicesRoute::Translation,
        form_revision: 41,
        form: Some(ServiceForm::Custom {
            draft: ServiceCustomDraft::default(),
            prompts: ServicePromptWorkspace {
                strategies: vec![ServicePrompt {
                    id: "general".into(),
                    name: "通用翻译".into(),
                    description: "普通文本".into(),
                    system_prompt: "Translate {target_lang}".into(),
                    builtin: true,
                    deletable: false,
                }],
                ..ServicePromptWorkspace::default()
            },
        }),
        ..ServicesSnapshot::default()
    })
}

#[test]
fn route_targets_and_provider_ids_are_invalidated_by_snapshots_and_form_changes() {
    let mut ui = ServicesUi::new(ServicesSnapshot {
        providers: vec![provider("first", false, true)],
        ..ServicesSnapshot::default()
    });
    let stale = target(&mut ui, ServiceControl::Configure("first".into()));
    ui.update(ServicesInput::Snapshot(ServicesSnapshot {
        revision: 1,
        providers: vec![provider("other", false, true)],
        ..ServicesSnapshot::default()
    }));
    assert!(ui.update(ServicesInput::Activate(stale)).is_empty());
    assert!(
        matches!(activate(&mut ui,ServiceControl::Configure("other".into())).as_slice(),[ServicesIntent::Configure{route:ServicesRoute::Ocr,provider_id}]if provider_id=="other")
    );
    assert!(matches!(
        activate(&mut ui, ServiceControl::Route(ServicesRoute::Tts)).as_slice(),
        [ServicesIntent::Route(ServicesRoute::Tts)]
    ));
    assert_eq!(ui.snapshot().route, ServicesRoute::Tts);
}

#[test]
fn translation_drag_reorders_only_same_group_and_capture_cancellation_does_not_write() {
    let mut ui = ServicesUi::new(ServicesSnapshot {
        route: ServicesRoute::Translation,
        providers: vec![
            provider("first", true, true),
            provider("second", true, false),
            provider("third", false, true),
        ],
        ..ServicesSnapshot::default()
    });
    let current = frame(&mut ui);
    let point = |id: &str| {
        let control = current
            .controls
            .iter()
            .find(|control| control.target.control == ServiceControl::Drag(id.into()))
            .unwrap();
        Point {
            x: control.rect.x + 4.,
            y: control.rect.y + 4.,
        }
    };
    ui.update(ServicesInput::PointerDown(point("first")));
    ui.update(ServicesInput::PointerMove(point("second")));
    assert!(
        matches!(ui.update(ServicesInput::PointerUp(point("second"))).as_slice(),[ServicesIntent::Reorder{source_id,target_id}]if source_id=="first"&&target_id=="second")
    );
    ui.update(ServicesInput::PointerDown(point("first")));
    assert!(ui
        .update(ServicesInput::PointerUp(point("third")))
        .is_empty());
    ui.update(ServicesInput::PointerDown(point("first")));
    ui.update(ServicesInput::PointerCancel);
    assert!(ui
        .update(ServicesInput::PointerUp(point("second")))
        .is_empty());
    ui.update(ServicesInput::PointerDown(point("first")));
    assert!(ui
        .update(ServicesInput::PointerUp(Point {
            x: 0.,
            y: point("second").y
        }))
        .is_empty());
}

#[test]
fn provider_hover_and_reorder_use_the_entire_row_at_each_interface_scale() {
    for scale in [1., 1.25] {
        let mut ui = ServicesUi::new(ServicesSnapshot {
            route: ServicesRoute::Translation,
            providers: vec![
                provider("first", true, true),
                provider("second", true, true),
            ],
            ..Default::default()
        });
        let size = Size {
            width: 900. * scale,
            height: 650. * scale,
        };
        let drawn = ui.frame(size, scale, &mut Metrics);
        let handle = drawn
            .controls
            .iter()
            .find(|c| c.target.control == ServiceControl::Drag("first".into()))
            .unwrap()
            .rect;
        let row = ui
            .provider_rows
            .iter()
            .find(|(id, _)| id == "second")
            .unwrap()
            .1;
        for y in [row.y + 2., row.y + row.height - 2.] {
            let drop = Point {
                x: (row.x + row.width - 2.) * scale,
                y: y * scale,
            };
            ui.update(ServicesInput::PointerMove(drop));
            assert_eq!(ui.hover, Some(ServiceControl::Configure("second".into())));
            ui.update(ServicesInput::PointerDown(Point {
                x: handle.x + 2.,
                y: handle.y + 2.,
            }));
            assert!(
                matches!(ui.update(ServicesInput::PointerUp(drop)).as_slice(),[ServicesIntent::Reorder{source_id,target_id}]if source_id=="first"&&target_id=="second")
            );
        }
    }
}

#[test]
fn first_pointer_click_places_the_caret_in_existing_and_masked_fields() {
    let mut ui = custom();
    input(
        &mut ui,
        ServiceField::Custom(ServiceCustomField::Name),
        "Existing name",
    );
    ui.update(ServicesInput::FocusLost);
    let drawn = frame(&mut ui);
    let name = drawn
        .controls
        .iter()
        .find(|c| {
            c.target.control
                == ServiceControl::Field(ServiceField::Custom(ServiceCustomField::Name))
        })
        .unwrap()
        .rect;
    ui.update(ServicesInput::PointerDown(Point {
        x: name.x + 13.,
        y: name.y + 12.,
    }));
    ui.update(ServicesInput::PointerUp(Point {
        x: name.x + 13.,
        y: name.y + 12.,
    }));
    ui.update(ServicesInput::Text("New ".into()));
    assert_eq!(
        ui.field_value(&ServiceField::Custom(ServiceCustomField::Name)),
        "New Existing name"
    );

    input(
        &mut ui,
        ServiceField::Custom(ServiceCustomField::ApiKey),
        "密🔑钥",
    );
    ui.update(ServicesInput::FocusLost);
    let drawn = frame(&mut ui);
    let field = drawn
        .controls
        .iter()
        .find(|c| {
            c.target.control
                == ServiceControl::Field(ServiceField::Custom(ServiceCustomField::ApiKey))
        })
        .unwrap()
        .rect;
    // The first visible bullet maps to the full UTF-8 grapheme, not its bytes.
    ui.update(ServicesInput::PointerDown(Point {
        x: field.x + 12. + 16. * 0.6,
        y: field.y + 12.,
    }));
    ui.update(ServicesInput::Text("A".into()));
    assert_eq!(
        ui.field_value(&ServiceField::Custom(ServiceCustomField::ApiKey)),
        "密A🔑钥"
    );
}

#[test]
fn credential_schema_drives_dynamic_keys_secret_masking_and_deepl_mode() {
    let mut ui = ServicesUi::new(ServicesSnapshot {
        form_revision: 5,
        form: Some(ServiceForm::Credentials {
            provider_id: "baidu-ocr".into(),
            name: "百度OCR".into(),
            fields: vec![
                ServiceCredentialField {
                    name: "api_key".into(),
                    label: "API Key".into(),
                    secret: false,
                },
                ServiceCredentialField {
                    name: "secret_key".into(),
                    label: "Secret Key".into(),
                    secret: true,
                },
            ],
        }),
        ..ServicesSnapshot::default()
    });
    input(
        &mut ui,
        ServiceField::Credential {
            provider_id: "baidu-ocr".into(),
            name: "api_key".into(),
        },
        "public-id",
    );
    input(
        &mut ui,
        ServiceField::Credential {
            provider_id: "baidu-ocr".into(),
            name: "secret_key".into(),
        },
        "sensitive-秘密🙂",
    );
    let drawn = frame(&mut ui);
    assert!(!format!("{:?}", drawn.scene.commands).contains("sensitive"));
    assert!(drawn
        .controls
        .iter()
        .any(|control| control.role == ServicesRole::SecretInput
            && control
                .value
                .as_ref()
                .is_some_and(|value| value.contains('•'))));
    assert!(
        matches!(activate(&mut ui,ServiceControl::SaveForm).as_slice(),[ServicesIntent::SaveCredentials{revision:5,provider_id,credentials,..}]if provider_id=="baidu-ocr"&&credentials["secret_key"]=="sensitive-秘密🙂")
    );
    ui.update(ServicesInput::Snapshot(ServicesSnapshot {
        form_revision: 6,
        route: ServicesRoute::Translation,
        form: Some(ServiceForm::Credentials {
            provider_id: "deeplx".into(),
            name: "DeepLX".into(),
            fields: vec![],
        }),
        ..ServicesSnapshot::default()
    }));
    let endpoint = ServiceField::Credential {
        provider_id: "deeplx".into(),
        name: "endpoint".into(),
    };
    assert!(frame(&mut ui)
        .controls
        .iter()
        .any(|control| control.target.control == ServiceControl::Field(endpoint.clone())));
    activate(&mut ui, ServiceControl::DeepLMode(true));
    let drawn = frame(&mut ui);
    assert!(!drawn
        .controls
        .iter()
        .any(|control| control.target.control == ServiceControl::Field(endpoint.clone())));
    assert!(drawn
        .controls
        .iter()
        .any(|control| control.role == ServicesRole::SecretInput));
}

#[test]
fn custom_form_preserves_protocol_modes_and_only_enables_valid_save_and_introspection() {
    let mut ui = custom();
    assert!(
        !frame(&mut ui)
            .controls
            .iter()
            .find(|control| control.target.control == ServiceControl::SaveForm)
            .unwrap()
            .enabled
    );
    input(
        &mut ui,
        ServiceField::Custom(ServiceCustomField::Name),
        "我的模型",
    );
    input(
        &mut ui,
        ServiceField::Custom(ServiceCustomField::ApiKey),
        "private-key",
    );
    assert!(matches!(
        activate(
            &mut ui,
            ServiceControl::Protocol(ServiceProtocol::OpenAiResponses)
        )
        .as_slice(),
        [ServicesIntent::Protocol {
            revision: 41,
            protocol: ServiceProtocol::OpenAiResponses
        }]
    ));
    assert!(activate(
        &mut ui,
        ServiceControl::ProtocolFamily(ServiceProtocol::OpenAi)
    )
    .is_empty());
    let Some(ServiceForm::Custom { draft, .. }) = &ui.snapshot().form else {
        panic!()
    };
    assert_eq!(draft.protocol, ServiceProtocol::OpenAiResponses);
    assert_eq!(draft.model, "gpt-5-mini");
    assert!(matches!(
        activate(&mut ui, ServiceControl::LoadModels).as_slice(),
        [ServicesIntent::LoadModels { revision: 41 }]
    ));
    assert!(matches!(
        activate(&mut ui, ServiceControl::TestConnection).as_slice(),
        [ServicesIntent::TestConnection { revision: 41 }]
    ));
    assert!(matches!(
        activate(&mut ui, ServiceControl::SaveForm).as_slice(),
        [ServicesIntent::SaveCustom { revision: 41 }]
    ));
    assert!(!format!("{:?}", frame(&mut ui).scene.commands).contains("private-key"));
}

#[test]
fn multiline_prompt_keeps_ime_newlines_clipboard_selection_and_strategy_rules() {
    let mut ui = custom();
    activate(&mut ui, ServiceControl::PromptStrategy("general".into()));
    let field = ServiceField::Custom(ServiceCustomField::PromptText);
    input(&mut ui, field.clone(), "第一行\nsecond line");
    assert_eq!(ui.text_edit().unwrap().text(), "第一行\nsecond line");
    ui.update(ServicesInput::Selection(0..9));
    ui.update(ServicesInput::ImePreedit {
        text: "新内容".into(),
        selection: Some(9..9),
    });
    assert_eq!(ui.text_edit().unwrap().text(), "第一行\nsecond line");
    let intents = ui.update(ServicesInput::ImeCommit("新内容".into()));
    assert!(
        matches!(intents.as_slice(),[ServicesIntent::EditPrompt{revision:41,draft}]if draft.system_prompt=="新内容\nsecond line")
    );
    ui.update(ServicesInput::KeyDown {
        key: Key::Enter,
        modifiers: Modifiers::default(),
    });
    assert!(ui.text_edit().unwrap().text().contains('\n'));
    let drawn = frame(&mut ui);
    assert!(drawn.scene.ime_cursor.is_some());
    assert!(drawn.scene.commands.iter().any(
        |command| matches!(command,DrawCommand::TextBlock{text,..}if text.contains("新内容\n"))
    ));
    assert!(!drawn
        .controls
        .iter()
        .any(|control| control.target.control == ServiceControl::DeletePrompt));
    assert!(matches!(
        activate(&mut ui, ServiceControl::SavePrompt).as_slice(),
        [ServicesIntent::SavePrompt { revision: 41 }]
    ));
    activate(&mut ui, ServiceControl::PromptStrategy("smart".into()));
    let Some(ServiceForm::Custom { prompts, .. }) = &ui.snapshot().form else {
        panic!()
    };
    assert!(prompts.draft.system_prompt.is_empty());
}

#[test]
fn tts_voices_rate_and_speech_use_real_ids_and_scaled_hit_coordinates() {
    let mut ui = ServicesUi::new(ServicesSnapshot {
        route: ServicesRoute::Tts,
        voices: (0..20)
            .map(|index| ServiceVoice {
                id: format!("voice-{index}"),
                name: format!("声音{index}"),
                locale: "zh-CN".into(),
            })
            .collect(),
        ..ServicesSnapshot::default()
    });
    activate(&mut ui, ServiceControl::VoiceMenu);
    let current = frame(&mut ui);
    let control = current
        .controls
        .iter()
        .find(|control| control.target.control == ServiceControl::Voice("voice-2".into()))
        .unwrap();
    let intents = ui.update(ServicesInput::PointerDown(Point {
        x: control.hit_rect.x + 1.,
        y: control.hit_rect.y + 1.,
    }));
    assert!(matches!(intents.as_slice(),[ServicesIntent::Voice(id)]if id=="voice-2"));
    let current = ui.frame(
        Size {
            width: 1125.,
            height: 812.5,
        },
        1.25,
        &mut Metrics,
    );
    let track = current
        .controls
        .iter()
        .find(|control| control.target.control == ServiceControl::Rate)
        .unwrap()
        .rect;
    ui.update(ServicesInput::PointerDown(Point {
        x: track.x + track.width - 1.,
        y: track.y + 2.,
    }));
    assert_eq!(ui.snapshot().rate, 400);
    ui.update(ServicesInput::PointerUp(Point {
        x: track.x + track.width - 1.,
        y: track.y + 2.,
    }));
    assert!(matches!(
        activate(&mut ui, ServiceControl::Speak).as_slice(),
        [ServicesIntent::Speak]
    ));
    activate(&mut ui, ServiceControl::VoiceMenu);
    frame(&mut ui);
    ui.update(ServicesInput::Scroll {
        position: Point {
            x: ui.voice_view.x + 5.,
            y: ui.voice_view.y + 5.,
        },
        delta_y: 500.,
    });
    assert!(ui.voice_scroll > 0.);
}

#[test]
fn focusing_an_offscreen_voice_scrolls_the_popup_and_preserves_the_page_scroll() {
    let mut ui = ServicesUi::new(ServicesSnapshot {
        route: ServicesRoute::Tts,
        voices: (0..30)
            .map(|i| ServiceVoice {
                id: format!("voice-{i}"),
                name: format!("声音{i}"),
                locale: "zh-CN".into(),
            })
            .collect(),
        ..Default::default()
    });
    activate(&mut ui, ServiceControl::VoiceMenu);
    let target = target(&mut ui, ServiceControl::Voice("voice-29".into()));
    ui.update(ServicesInput::Focus(target));
    let drawn = frame(&mut ui);
    assert_eq!(ui.scroll, 0.);
    let last = drawn
        .controls
        .iter()
        .find(|c| c.target.control == ServiceControl::Voice("voice-29".into()))
        .unwrap();
    assert_eq!(last.hit_rect.height, last.rect.height);
    assert!(ui.voice_scroll > 0.);
}

#[test]
fn voice_popup_stays_inside_a_short_viewport_and_scrolls_to_the_last_voice() {
    let mut ui = ServicesUi::new(ServicesSnapshot {
        route: ServicesRoute::Tts,
        voices: (0..30)
            .map(|index| ServiceVoice {
                id: format!("voice-{index}"),
                name: format!("声音{index}"),
                locale: "zh-CN".into(),
            })
            .collect(),
        ..Default::default()
    });
    activate(&mut ui, ServiceControl::VoiceMenu);
    let size = Size {
        width: 700.,
        height: 350.,
    };
    ui.frame(size, 1., &mut Metrics);
    assert!(ui.voice_view.y >= ui.viewport.y);
    assert!(ui.voice_view.y + ui.voice_view.height <= size.height);
    ui.update(ServicesInput::Scroll {
        position: Point {
            x: ui.voice_view.x + 5.,
            y: ui.voice_view.y + 5.,
        },
        delta_y: 10_000.,
    });
    let frame = ui.frame(size, 1., &mut Metrics);
    let last = frame
        .controls
        .iter()
        .find(|control| control.target.control == ServiceControl::Voice("voice-29".into()))
        .unwrap();
    assert!(last.hit_rect.height > 0.);
    let intents = ui.update(ServicesInput::PointerDown(Point {
        x: last.hit_rect.x + 1.,
        y: last.hit_rect.y + 1.,
    }));
    assert!(matches!(intents.as_slice(),[ServicesIntent::Voice(id)]if id=="voice-29"));
}

#[test]
fn rate_drag_survives_committed_snapshots_from_the_synchronous_host() {
    let mut ui = ServicesUi::new(ServicesSnapshot {
        route: ServicesRoute::Tts,
        ..Default::default()
    });
    let drawn = frame(&mut ui);
    let track = drawn
        .controls
        .iter()
        .find(|control| control.target.control == ServiceControl::Rate)
        .unwrap()
        .rect;
    ui.update(ServicesInput::PointerDown(Point {
        x: track.x + 10.,
        y: track.y + 1.,
    }));
    let mut saved = ui.snapshot().clone();
    saved.revision += 1;
    ui.update(ServicesInput::Snapshot(saved));
    frame(&mut ui);
    let result = ui.update(ServicesInput::PointerMove(Point {
        x: track.x + track.width - 1.,
        y: track.y + 1.,
    }));
    assert!(matches!(result.as_slice(), [ServicesIntent::Rate(400)]));
}

#[test]
fn busy_snapshots_keep_form_drafts_but_disable_edits_and_duplicate_save() {
    let mut ui = custom();
    input(
        &mut ui,
        ServiceField::Custom(ServiceCustomField::Name),
        "keep",
    );
    let mut snapshot = ui.snapshot().clone();
    snapshot.busy = true;
    ui.update(ServicesInput::Snapshot(snapshot));
    let current = frame(&mut ui);
    let save = current
        .controls
        .iter()
        .find(|control| control.target.control == ServiceControl::SaveForm)
        .unwrap();
    assert!(!save.enabled);
    assert!(ui
        .update(ServicesInput::Activate(save.target.clone()))
        .is_empty());
    ui.update(ServicesInput::Text("ignored".into()));
    assert_eq!(ui.text_edit().unwrap().text(), "keep");
}

#[test]
fn all_service_routes_draw_the_same_navigation_and_clip_controls_at_every_theme_scale() {
    for theme in [Theme::Native, Theme::Studio, Theme::Paper] {
        for route in ServicesRoute::ALL {
            for scale in [1., 1.25, 1.75, 2.] {
                let mut snapshot = ServicesSnapshot {
                    route,
                    ..ServicesSnapshot::default()
                };
                snapshot.shell.theme = theme;
                snapshot.providers = vec![provider("a", true, true), provider("b", false, false)];
                let mut ui = ServicesUi::new(snapshot);
                let current = ui.frame(
                    Size {
                        width: 900.,
                        height: 650.,
                    },
                    scale,
                    &mut Metrics,
                );
                assert_eq!(
                    current.scene.size,
                    Size {
                        width: 900.,
                        height: 650.
                    }
                );
                assert!(current
                    .controls
                    .iter()
                    .any(|control| matches!(control.target.control, ServiceControl::Shell(_))));
                for control in current.controls {
                    assert!(control.hit_rect.width >= 0. && control.hit_rect.height >= 0.);
                    if control.hit_rect.height > 0. {
                        assert!(control.hit_rect.x >= 0. && control.hit_rect.y >= 0.);
                    }
                }
            }
        }
    }
}

#[test]
fn endpoint_previews_match_protocol_completion_and_preserve_custom_paths() {
    use super::draw::endpoint_preview;
    for (protocol, endpoint, model, expected) in [
        (
            ServiceProtocol::OpenAi,
            "https://api.test/v1",
            "",
            "https://api.test/v1/chat/completions",
        ),
        (
            ServiceProtocol::OpenAiResponses,
            "https://api.test/gateway/v1",
            "",
            "https://api.test/gateway/v1/responses",
        ),
        (
            ServiceProtocol::Anthropic,
            "https://api.test/v1/mess",
            "",
            "https://api.test/v1/messages",
        ),
        (
            ServiceProtocol::Gemini,
            "https://api.test/v1beta",
            "models/gemini-pro",
            "https://api.test/v1beta/models/gemini-pro:generateContent",
        ),
        (
            ServiceProtocol::OpenAi,
            "https://api.test/custom",
            "",
            "https://api.test/custom",
        ),
        (
            ServiceProtocol::OpenAi,
            "https://api.test?token=x",
            "",
            "https://api.test/v1/chat/completions?token=x",
        ),
    ] {
        assert_eq!(endpoint_preview(protocol, endpoint, model), expected);
    }
}
