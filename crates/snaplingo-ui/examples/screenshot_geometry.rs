use snaplingo_ui::*;
fn main() {
    for theme in [Theme::Native, Theme::Studio, Theme::Paper] {
        let mut ui = SettingsUi::new(Snapshot {
            theme,
            ..Snapshot::default()
        });
        ui.update(Event::Activate(Target::Navigate(Page::Screenshot)));
        let scene = ui.frame(
            Size {
                width: 900.,
                height: 650.,
            },
            1.,
        );
        println!("{theme:?}");
        for node in scene.geometry {
            if matches!(
                node.kind,
                GeometryKind::Card(_)
                    | GeometryKind::Control(
                        Target::Screenshot(_)
                            | Target::ScreenshotColor(..)
                            | Target::PickScreenshotColor(_)
                            | Target::BrowseScreenshotDirectory
                    )
            ) {
                println!("{:?} {:?}", node.kind, node.rect);
            }
        }
    }
}
