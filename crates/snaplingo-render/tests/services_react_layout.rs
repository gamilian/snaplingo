//! Expected geometry comes from retained React components in WebKit, not Rust.
use snaplingo_render::RendererTextMetrics;
use snaplingo_ui::{services::*, DrawCommand, Point, Rect, Size, Theme};
mod support {
    pub mod services_fixture;
}

fn expected(value: &serde_json::Value) -> Rect {
    let values = value.as_array().unwrap();
    Rect {
        x: values[0].as_f64().unwrap() as f32,
        y: values[1].as_f64().unwrap() as f32,
        width: values[2].as_f64().unwrap() as f32,
        height: values[3].as_f64().unwrap() as f32,
    }
}

#[test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "WKWebView geometry was captured with macOS system fonts"
)]
fn services_match_108_actual_react_scenes_with_renderer_text_metrics() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../snaplingo-ui/tests/fixtures/react-services-layout.json"
    ))
    .unwrap();
    let mut metrics = RendererTextMetrics::default();
    let mut differences = vec![];
    let mut count = 0;
    for reference in fixture["scenes"].as_array().unwrap() {
        let theme = match reference["theme"].as_str().unwrap() {
            "studio" => Theme::Studio,
            "paper" => Theme::Paper,
            _ => Theme::Native,
        };
        let scenario = reference["scenario"].as_str().unwrap();
        let route = if scenario.starts_with("ocr") {
            ServicesRoute::Ocr
        } else if scenario == "tts" {
            ServicesRoute::Tts
        } else {
            ServicesRoute::Translation
        };
        let mut ui = ServicesUi::new(support::services_fixture::fixture(
            route,
            theme,
            reference["dark"].as_bool().unwrap(),
            scenario,
        ));
        let size = Size {
            width: reference["width"].as_f64().unwrap() as f32,
            height: 650.,
        };
        let first = ui.frame(size, 1., &mut metrics);
        if scenario == "deepl" {
            let target = first
                .controls
                .iter()
                .find(|c| c.target.control == ServiceControl::DeepLMode(true))
                .unwrap()
                .target
                .clone();
            ui.update(ServicesInput::Activate(target));
        }
        if scenario == "custom-prompt" {
            ui.update(ServicesInput::Scroll {
                position: Point {
                    x: size.width - 30.,
                    y: 600.,
                },
                delta_y: 650.,
            });
        }
        let frame = ui.frame(size, 1., &mut metrics);
        let mut compare = |actual: Rect, expected: Rect, label: &str| {
            count += 1;
            for (axis, a, e) in [
                ("x", actual.x, expected.x),
                ("y", actual.y, expected.y),
                ("width", actual.width, expected.width),
                ("height", actual.height, expected.height),
            ] {
                if (a - e).abs() > 0.5 {
                    differences.push(format!(
                        "{}/{label}/{axis}: expected {e}, got {a}",
                        reference["source"].as_str().unwrap()
                    ));
                }
            }
        };
        compare(
            frame.scene.content,
            expected(&reference["content"]),
            "content",
        );
        let mut controls = frame
            .controls
            .iter()
            .filter(|c| !matches!(c.target.control, ServiceControl::Shell(_)))
            .collect::<Vec<_>>();
        for control in reference["controls"].as_array().unwrap() {
            let label = match control["label"].as_str().unwrap() {
                "返回供应商列表" => "‹",
                "测试联通" => "测试",
                label => label,
            };
            let index = controls
                .iter()
                .position(|c| {
                    c.label == label
                        || (label.starts_with("婷婷")
                            && c.target.control == ServiceControl::VoiceMenu)
                })
                .unwrap_or_else(|| panic!("{} missing {label}", reference["source"]));
            let actual = controls.remove(index);
            compare(actual.rect, expected(&control["frame"]), label);
            assert_eq!(
                actual.enabled,
                control["enabled"].as_bool().unwrap(),
                "{} {label}",
                reference["source"]
            );
            if let Some(appearance) = control.get("appearance") {
                let color = |key: &str| {
                    let channels = appearance[key].as_array().unwrap();
                    [
                        channels[0].as_u64().unwrap() as u8,
                        channels[1].as_u64().unwrap() as u8,
                        channels[2].as_u64().unwrap() as u8,
                        channels[3].as_u64().unwrap() as u8,
                    ]
                };
                assert!(frame.scene.commands.iter().any(|command|matches!(command,DrawCommand::FillRect{rect,color:background,..}if *rect==actual.rect&&*background==color("background"))),"{} {label} background",reference["source"]);
                assert!(frame.scene.commands.iter().any(|command|matches!(command,DrawCommand::Text{text,color:foreground,..}if text==label&&*foreground==color("foreground"))),"{} {label} foreground",reference["source"]);
            }
        }
        for provider in reference["providers"].as_array().unwrap() {
            let expected = expected(&provider["frame"]);
            let distance = |r: Rect| {
                (r.x - expected.x).abs()
                    + (r.y - expected.y).abs()
                    + (r.width - expected.width).abs()
                    + (r.height - expected.height).abs()
            };
            let actual = frame
                .scene
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::FillRect { rect, .. } => Some(*rect),
                    _ => None,
                })
                .min_by(|a, b| distance(*a).total_cmp(&distance(*b)))
                .unwrap();
            compare(actual, expected, provider["name"].as_str().unwrap());
        }
    }
    assert_eq!(fixture["scenes"].as_array().unwrap().len(), 108);
    assert_eq!(count, 1788);
    assert!(
        differences.is_empty(),
        "{} geometry differences:\n{}",
        differences.len(),
        differences[..differences.len().min(35)].join("\n")
    );
}
