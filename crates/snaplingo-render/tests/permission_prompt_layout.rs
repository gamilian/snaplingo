//! Independently collected rectangles from the retained React permission gate.
use snaplingo_render::RendererTextMetrics;
use snaplingo_ui::{permission_prompt::*, Snapshot, Theme};
#[test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "WKWebView geometry was captured with macOS system fonts"
)]
fn permission_guide_matches_actual_retained_card_and_actions() {
    let references: serde_json::Value = serde_json::from_str(include_str!(
        "../../snaplingo-ui/tests/fixtures/react-permission-guide-layout.json"
    ))
    .unwrap();
    let mut metrics = RendererTextMetrics::default();
    let mut count = 0;
    assert_eq!(references.as_array().unwrap().len(), 24);
    for reference in references.as_array().unwrap() {
        let name = reference["name"].as_str().unwrap();
        let theme = if name.starts_with("studio") {
            Theme::Studio
        } else if name.starts_with("paper") {
            Theme::Paper
        } else {
            Theme::Native
        };
        let snapshot = PermissionPromptSnapshot {
            shell: Snapshot {
                theme,
                dark: name.contains("-dark"),
                ..Default::default()
            },
            needs_installation: name.ends_with("installation"),
            busy: false,
            attempted: name.ends_with("attempted"),
            error: name
                .ends_with("error")
                .then(|| "Synthetic request failure".into()),
        };
        let mut ui = PermissionPromptUi::new(snapshot);
        let frame = ui.frame(1., &mut metrics);
        let card = &reference["card"];
        assert_eq!(
            frame.controls.len(),
            reference["controls"].as_array().unwrap().len()
        );
        for (actual, expected) in [
            (frame.scene.size.width, card[2].as_f64().unwrap() as f32),
            (frame.scene.size.height, card[3].as_f64().unwrap() as f32),
        ] {
            assert!(
                (actual - expected).abs() <= 0.5,
                "{name} size expected{expected}actual{actual}"
            );
        }
        for (actual, expected) in frame
            .controls
            .iter()
            .zip(reference["controls"].as_array().unwrap())
        {
            assert_eq!(actual.label, expected["text"].as_str().unwrap());
            assert_eq!(!actual.enabled, expected["disabled"].as_bool().unwrap());
            let expected = &expected["frame"];
            for (i, value) in [
                actual.rect.x,
                actual.rect.y,
                actual.rect.width,
                actual.rect.height,
            ]
            .into_iter()
            .enumerate()
            {
                let origin = if i < 2 { card[i].as_f64().unwrap() } else { 0. };
                let want = (expected[i].as_f64().unwrap() - origin) as f32;
                assert!(
                    (value - want).abs() <= 0.5,
                    "{name}/{:?}/{i}expected{want}actual{value}",
                    actual.target
                );
            }
            count += 1;
        }
    }
    assert_eq!(count, 54);
}
