//! Review the portable pin scene without creating a native window.
#[path = "../window/pinned/scene.rs"]
mod scene;
use scene::{PinUi, Preferences};
fn main() -> Result<(), String> {
    let directory = "target/shared-pin";
    std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    let image = image::RgbaImage::from_fn(640, 400, |x, y| {
        image::Rgba([
            (x * 255 / 640) as u8,
            (y * 255 / 400) as u8,
            140,
            if x < 60 { 120 } else { 255 },
        ])
    });
    let mut bytes = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    let mut ui = PinUi::new(
        bytes.get_ref(),
        Some("Pinned screenshot".into()),
        Preferences {
            opacity: 1.,
            shadow: true,
            border: true,
            border_shortcut: "f8".into(),
        },
    )?;
    let mut renderer = snaplingo_render::Renderer::default();
    ui.hovered = true;
    renderer
        .render(&ui.scene(), 1.)?
        .save_png(format!("{directory}/toolbar.png"))?;
    ui.cursor = snaplingo_ui::Point { x: 200., y: 160. };
    ui.modifiers(snaplingo_ui::Modifiers {
        alt: true,
        ..Default::default()
    });
    renderer
        .render(&ui.scene(), 1.)?
        .save_png(format!("{directory}/inspection.png"))?;
    ui.modifiers(Default::default());
    ui.key(snaplingo_ui::Key::Character('1'), false)?;
    ui.key(snaplingo_ui::Key::Character('5'), false)?;
    renderer
        .render(&ui.scene(), 1.)?
        .save_png(format!("{directory}/rotated-grayscale.png"))?;
    Ok(())
}
