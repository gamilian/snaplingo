use snaplingo_render::Renderer;
use snaplingo_ui::{scrolling::*, Snapshot, Theme};
fn main() -> Result<(), String> {
    let directory = "target/shared-scrolling";
    std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    let mut renderer = Renderer::default();
    for (name, dark, error, working, scale) in [
        ("normal", false, None, false, 1.),
        ("dark-175", true, None, false, 1.75),
        (
            "error",
            false,
            Some("无法追加截图，请重试或撤销上一段。".into()),
            false,
            1.,
        ),
        ("working", false, None, true, 1.),
    ] {
        let mut ui = ScrollingUi::new(ScrollingSnapshot {
            shell: Snapshot {
                theme: Theme::Native,
                dark,
                ..Default::default()
            },
            width: 1200,
            height: 12400,
            can_undo: true,
            preview_available: true,
            preview_expanded: true,
            working,
            hint: Some("已达到最大截图高度，可完成并保存。".into()),
            error,
        });
        let frame = ui.frame(scale);
        renderer
            .render(&frame.scene, 1.)?
            .save_png(format!("{directory}/{name}.png"))?;
    }
    Ok(())
}
