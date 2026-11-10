//! Synthetic records matching the retained React Library reference fixture.
use snaplingo_ui::render::{Renderer, RendererTextMetrics};
use snaplingo_ui::{library::*, Size, Theme};
#[path = "../tests/support/library_fixture.rs"]
mod library_fixture;
use library_fixture::fixture;

fn rect(rect: snaplingo_ui::Rect) -> String {
    format!(
        "{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}}",
        rect.x, rect.y, rect.width, rect.height
    )
}
fn main() -> Result<(), String> {
    let directory = std::env::args()
        .nth(1)
        .unwrap_or("target/shared-library".into());
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let mut renderer = Renderer::default();
    let mut metrics = RendererTextMetrics::default();
    for (theme, name) in [
        (Theme::Native, "native"),
        (Theme::Studio, "studio"),
        (Theme::Paper, "paper"),
    ] {
        for dark in [false, true] {
            for compact in [false, true] {
                for (kind, page) in [
                    (LibraryKind::History, "history"),
                    (LibraryKind::Favorites, "favorites"),
                ] {
                    let size = Size {
                        width: if compact { 700. } else { 900. },
                        height: 650.,
                    };
                    let mut ui = LibraryUi::new(fixture(kind, theme, dark));
                    for (index, selection) in [(0, "translation"), (1, "ocr"), (2, "screenshot")] {
                        if kind == LibraryKind::History && index == 2 {
                            continue;
                        }
                        ui.frame(size, 1., &mut metrics);
                        let key = ui.snapshot().items[index].key;
                        ui.update(LibraryInput::Activate(LibraryTarget::Item(key)));
                        let frame = ui.frame(size, 1., &mut metrics);
                        let name = format!(
                            "{name}{}{}-{page}-{selection}",
                            if dark { "-dark" } else { "" },
                            if compact { "-compact" } else { "" }
                        );
                        renderer
                            .render(&frame.scene, 1.)?
                            .save_png(format!("{directory}/{name}.png"))?;
                        let controls = frame
                            .controls
                            .iter()
                            .map(|c| {
                                format!(
                                    "{{\"target\":\"{:?}\",\"frame\":{}}}",
                                    c.target,
                                    rect(c.rect)
                                )
                            })
                            .collect::<Vec<_>>()
                            .join(",");
                        std::fs::write(
                            format!("{directory}/{name}.json"),
                            format!(
                                "{{\"content\":{},\"controls\":[{controls}]}}",
                                rect(frame.scene.content)
                            ),
                        )
                        .map_err(|e| e.to_string())?;
                    }
                }
            }
        }
    }
    println!("Rendered isolated Library references into {directory}");
    Ok(())
}
