use snaplingo_render::Renderer;
use snaplingo_ui::{Event, Page, Section, SettingsUi, Size, Snapshot, Target, Theme};

fn main() -> Result<(), String> {
    let directory = std::env::args().nth(1).unwrap_or("target/shared-ui".into());
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let mut renderer = Renderer::default();
    for (page, page_name) in [
        (Page::General, "general"),
        (Page::Ocr, "ocr"),
        (Page::Translation, "translation"),
        (Page::Screenshot, "screenshot"),
    ] {
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
                        width: if compact { 700.0 } else { 900.0 },
                        height: 650.0,
                    };
                    ui.update(Event::Activate(Target::Navigate(page)));
                    for section in Section::for_page(page) {
                        let scene = ui.frame(size, 1.0);
                        let target = scene
                            .hits
                            .iter()
                            .find(|hit| hit.target == Target::Section(*section))
                            .unwrap();
                        ui.update(Event::PointerDown(snaplingo_ui::Point {
                            x: target.rect.x + target.rect.width * 0.5,
                            y: target.rect.y + target.rect.height * 0.5,
                        }));
                        let scene = ui.frame(size, 1.0);
                        let section = match section {
                            Section::Hotkeys => "hotkeys",
                            Section::Recognition => "recognition",
                            Section::Window => "window",
                            Section::Language => "language",
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
                        let path = format!(
                            "{directory}/{page_name}-{name}{}{}-{section}.png",
                            if dark { "-dark" } else { "" },
                            if compact { "-compact" } else { "" }
                        );
                        renderer.render(&scene, 1.0)?.save_png(path)?;
                    }
                }
            }
        }
    }
    println!("Rendered shared Rust Settings scenes into {directory}");
    Ok(())
}
