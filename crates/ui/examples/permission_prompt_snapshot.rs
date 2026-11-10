//! Render the shared first-use guide without requesting OS permissions.
use snaplingo_ui::render::{Renderer, RendererTextMetrics};
use snaplingo_ui::{permission_prompt::*, Appearance, Theme};
fn main() -> Result<(), String> {
    let directory = std::env::args()
        .nth(1)
        .unwrap_or("target/shared-permissions".into());
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let mut renderer = Renderer::default();
    let mut metrics = RendererTextMetrics::default();
    for (name, theme) in [
        ("native", Theme::Native),
        ("studio", Theme::Studio),
        ("paper", Theme::Paper),
    ] {
        for dark in [false, true] {
            for scenario in ["initial", "attempted", "error", "installation"] {
                let snapshot = PermissionPromptSnapshot {
                    appearance: Appearance {
                        theme,
                        dark,
                        ..Default::default()
                    },
                    needs_installation: scenario == "installation",
                    busy: false,
                    attempted: scenario == "attempted",
                    error: (scenario == "error").then(|| "Synthetic request failure".into()),
                };
                let mut ui = PermissionPromptUi::new(snapshot);
                let frame = ui.frame(1., &mut metrics);
                std::fs::write(format!("{directory}/{name}{}-{scenario}.json",if dark{"-dark"}else{""}),serde_json::to_vec_pretty(&serde_json::json!({"size":[frame.scene.size.width,frame.scene.size.height],"controls":frame.controls.iter().map(|c|serde_json::json!({"label":c.label,"rect":[c.rect.x,c.rect.y,c.rect.width,c.rect.height]})).collect::<Vec<_>>()})).unwrap()).map_err(|e|e.to_string())?;
                renderer.render(&frame.scene, 2.)?.save_png(format!(
                    "{directory}/{name}{}-{scenario}.png",
                    if dark { "-dark" } else { "" }
                ))?;
            }
        }
    }
    println!("Rendered 24 permission guide variants into {directory}");
    Ok(())
}
