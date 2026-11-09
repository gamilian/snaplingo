//! Isolated DTOs matching the retained React Services reference fixture.
use snaplingo_render::{Renderer, RendererTextMetrics};
use snaplingo_ui::{services::*, Point, Size, Theme};

#[path = "../tests/support/services_fixture.rs"]
mod services_fixture;
use services_fixture::fixture;

fn rect(rect: snaplingo_ui::Rect) -> String {
    format!(
        "{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}}",
        rect.x, rect.y, rect.width, rect.height
    )
}
fn main() -> Result<(), String> {
    let directory = std::env::args()
        .nth(1)
        .unwrap_or("target/shared-services".into());
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
                for (route, scenario) in [
                    (ServicesRoute::Ocr, "ocr"),
                    (ServicesRoute::Translation, "translation"),
                    (ServicesRoute::Tts, "tts"),
                    (ServicesRoute::Ocr, "ocr-credentials"),
                    (ServicesRoute::Translation, "translation-credentials"),
                    (ServicesRoute::Translation, "deeplx"),
                    (ServicesRoute::Translation, "deepl"),
                    (ServicesRoute::Translation, "custom"),
                    (ServicesRoute::Translation, "custom-prompt"),
                ] {
                    let size = Size {
                        width: if compact { 700. } else { 900. },
                        height: 650.,
                    };
                    let mut ui = ServicesUi::new(fixture(route, theme, dark, scenario));
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
                    let name = format!(
                        "{name}{}{}-{scenario}",
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
                                "{{\"target\":{},\"label\":{},\"frame\":{}}}",
                                serde_json::to_string(&format!("{:?}", c.target.control)).unwrap(),
                                serde_json::to_string(&c.label).unwrap(),
                                rect(c.rect)
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(",");
                    let fills = frame
                        .scene
                        .commands
                        .iter()
                        .filter_map(|command| match command {
                            snaplingo_ui::DrawCommand::FillRect { rect: frame, .. } => {
                                Some(rect(*frame))
                            }
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join(",");
                    std::fs::write(
                        format!("{directory}/{name}.json"),
                        format!(
                            "{{\"content\":{},\"controls\":[{controls}],\"fills\":[{fills}]}}",
                            rect(frame.scene.content)
                        ),
                    )
                    .map_err(|e| e.to_string())?;
                }
            }
        }
    }
    println!("Rendered isolated Services references into {directory}");
    Ok(())
}
