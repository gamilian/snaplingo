//! Baselines were captured from the retained React GeneralPage in WebKit.
use snaplingo_ui::*;
#[test]
fn general_matches_actual_react_cards_and_controls_in_seventy_two_scenes() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/react-general-layout.json")).unwrap();
    let mut differences = vec![];
    let mut compared = 0;
    for reference in fixture["scenes"].as_array().unwrap() {
        let name = reference["theme"].as_str().unwrap();
        let theme = match name {
            "studio" => Theme::Studio,
            "paper" => Theme::Paper,
            _ => Theme::Native,
        };
        let mut snapshot = Snapshot {
            theme,
            dark: reference["dark"].as_bool().unwrap(),
            ..Snapshot::default()
        };
        snapshot.general = GeneralPreferences {
            interface_theme: name.into(),
            appearance: "light".into(),
            start_on_boot: true,
            proxy_mode: "manual".into(),
            proxy_url: "http://127.0.0.1:7890".into(),
            request_timeout_ms: 25000,
            retry_count: 4,
            log_level: "debug".into(),
            log_retention_days: 20,
            gpu_composition: true,
            performance_monitoring: true,
            ..GeneralPreferences::default()
        };
        snapshot.general_runtime = GeneralRuntime {
            permissions_required: true,
            screen_recording: Some(true),
            accessibility: Some(true),
            can_reset_permissions: true,
            version: "0.1.23".into(),
            update_description: "打开设置时自动查找最新正式版".into(),
            ..GeneralRuntime::default()
        };
        let mut ui = SettingsUi::new(snapshot);
        ui.update(Event::Activate(Target::Navigate(Page::General)));
        let section = match reference["section"].as_str().unwrap() {
            "permissions" => Section::Permissions,
            "network" => Section::Network,
            "maintenance" => Section::Maintenance,
            "experimental" => Section::Experimental,
            "about" => Section::About,
            _ => Section::Interface,
        };
        ui.frame(
            Size {
                width: reference["width"].as_f64().unwrap() as f32,
                height: 650.,
            },
            1.,
        );
        if section != Section::Interface {
            ui.update(Event::Activate(Target::Section(section)));
        }
        let scene = ui.frame(
            Size {
                width: reference["width"].as_f64().unwrap() as f32,
                height: 650.,
            },
            1.,
        );
        let mut compare = |actual: Rect, expected: &serde_json::Value, label: &str| {
            compared += 1;
            for (key, value) in [
                ("x", actual.x),
                ("y", actual.y),
                ("width", actual.width),
                ("height", actual.height),
            ] {
                let expected = expected[key].as_f64().unwrap() as f32;
                if (value - expected).abs() > 0.5 {
                    differences.push(format!(
                        "{name}/{section:?}/{label}.{key}: expected{expected},got{value}"
                    ));
                }
            }
        };
        compare(scene.content, &reference["content"], "content");
        for (index, card) in reference["cards"].as_array().unwrap().iter().enumerate() {
            let kind = GeometryKind::Card(Section::for_page(Page::General)[index]);
            compare(
                scene.geometry.iter().find(|n| n.kind == kind).unwrap().rect,
                &card["frame"],
                &format!("{kind:?}"),
            );
            for row in card["rows"].as_array().unwrap() {
                let label = row["text"].as_str().unwrap().split('\n').next().unwrap();
                let target = GeneralField::ALL
                    .into_iter()
                    .find(|field| field.label() == label)
                    .map(Target::General);
                let controls = row["controls"].as_array().unwrap();
                for (control_index, control) in controls.iter().enumerate() {
                    let target = if let Some(target) = target {
                        if control_index == 0 {
                            Some(target)
                        } else if let Target::General(field) = target {
                            Some(Target::GeneralStep(field, control_index == 1))
                        } else {
                            None
                        }
                    } else {
                        let command = match (label, control_index) {
                            ("日志操作", 0) => Some(GeneralCommand::RefreshLogs),
                            ("日志操作", 1) => Some(GeneralCommand::ClearLogs),
                            ("历史记录", _) => Some(GeneralCommand::ClearHistory),
                            ("项目主页", _) => Some(GeneralCommand::OpenProject),
                            ("第三方组件", _) => Some(GeneralCommand::ShowNotices),
                            ("软件更新", 0) => Some(GeneralCommand::CheckUpdates),
                            ("软件更新", 1) => Some(GeneralCommand::OpenReleasePage),
                            _ => None,
                        };
                        if label == "应用设置" {
                            Some(Target::GeneralUnavailable)
                        } else {
                            command.map(Target::GeneralCommand)
                        }
                    };
                    if let Some(target) = target {
                        compare(
                            scene
                                .geometry
                                .iter()
                                .find(|n| n.kind == GeometryKind::Control(target))
                                .unwrap()
                                .rect,
                            &control["frame"],
                            &format!("{target:?}"),
                        );
                    }
                }
            }
        }
        for (index, expected) in reference["themes"].as_array().unwrap().iter().enumerate() {
            compare(
                scene
                    .geometry
                    .iter()
                    .find(|n| n.kind == GeometryKind::Control(Target::GeneralTheme(index)))
                    .unwrap()
                    .rect,
                expected,
                "theme",
            );
        }
    }
    assert!(
        differences.is_empty(),
        "{} geometry differences:\n{}",
        differences.len(),
        differences
            .into_iter()
            .take(50)
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert_eq!(compared, 2520);
}
