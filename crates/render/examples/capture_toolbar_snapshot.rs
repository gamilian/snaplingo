//! Raster inspection against the actual native AppKit capture toolbar.
use snaplingo_render::{Renderer, RendererTextMetrics};
use snaplingo_ui::capture::*;
#[path = "../tests/support/capture_fixture.rs"]
mod capture_fixture;
fn main() -> Result<(), String> {
    let directory = std::env::args()
        .nth(1)
        .unwrap_or("target/shared-capture".into());
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let mut snapshot = capture_fixture::fixture();
    let mut ui = CaptureUi::default();
    let mut renderer = Renderer::default();
    let mut text = RendererTextMetrics::default();
    let frame = ui.frame_with_text(&snapshot, &mut text);
    renderer
        .render(&frame.scene, 2.)?
        .save_png(format!("{directory}/toolbar.png"))?;
    ui.activate(CaptureTarget::PaletteMenu, &snapshot);
    let frame = ui.frame_with_text(&snapshot, &mut text);
    renderer
        .render(&frame.scene, 2.)?
        .save_png(format!("{directory}/palette.png"))?;
    ui.activate(CaptureTarget::ShapeMenu, &snapshot);
    let frame = ui.frame_with_text(&snapshot, &mut text);
    renderer
        .render(&frame.scene, 2.)?
        .save_png(format!("{directory}/shape-menu.png"))?;
    ui.dismiss_menus();
    snapshot.tool = Some(CaptureTool::Text);
    snapshot.style.text_mode = true;
    snapshot.style.round_enabled = false;
    snapshot.style.font_family = ".Al Bayan PUA".into();
    snapshot.style.stroke_width = 36;
    snapshot.style.font_size = 36;
    snapshot.style.bold = true;
    snapshot.style.italic = true;
    let frame = ui.frame_with_text(&snapshot, &mut text);
    renderer
        .render(&frame.scene, 2.)?
        .save_png(format!("{directory}/text-toolbar.png"))?;
    println!("Rendered capture toolbar, palette, grouped tools and text style into {directory}");
    Ok(())
}
