use snaplingo_ui::*;
fn main() {
    for (theme, name) in [
        (Theme::Native, "native"),
        (Theme::Studio, "studio"),
        (Theme::Paper, "paper"),
    ] {
        let mut snapshot = Snapshot {
            theme,
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
        let scene = ui.frame(
            Size {
                width: 900.,
                height: 650.,
            },
            1.,
        );
        println!("{name}");
        for n in scene.geometry {
            println!("{:?} {:?}", n.kind, n.rect);
        }
    }
}
