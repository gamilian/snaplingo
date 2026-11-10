use snaplingo_ui::{capture::icon_colors, Color, RasterImage, Theme};
use std::collections::HashMap;

pub(super) fn icons(theme: Theme, dark: bool) -> HashMap<String, RasterImage> {
    macro_rules! asset {
        ($name:literal) => {
            (
                $name,
                include_bytes!(concat!("../../../../../assets/toolbar/", $name, ".png")).as_slice(),
            )
        };
    }
    let mut result = HashMap::new();
    let colors = icon_colors(theme, dark);
    for (name, data) in [
        asset!("select"),
        asset!("rectangle"),
        asset!("ellipse"),
        asset!("shapes"),
        asset!("arrow"),
        asset!("line"),
        asset!("pen"),
        asset!("text"),
        asset!("mosaic"),
        asset!("eraser"),
        asset!("highlight"),
        asset!("region-highlight"),
        asset!("blur"),
        asset!("spotlight"),
        asset!("undo"),
        asset!("redo"),
        asset!("favorite"),
        asset!("pin"),
        asset!("copy"),
        asset!("save"),
    ] {
        let image = image::load_from_memory(data)
            .expect("Bundled capture icon is valid")
            .into_rgba8();
        for (variant, color) in [
            ("", colors[0]),
            ("-active", colors[1]),
            ("-disabled", colors[2]),
            ("-on", colors[3]),
        ] {
            result.insert(format!("{name}{variant}"), tint(&image, color));
        }
    }
    result
}
fn tint(image: &image::RgbaImage, color: Color) -> RasterImage {
    let pixels = image
        .pixels()
        .flat_map(|p| {
            [
                color[0],
                color[1],
                color[2],
                ((u16::from(p[3]) * u16::from(color[3]) + 127) / 255) as u8,
            ]
        })
        .collect();
    RasterImage::from_rgba(image.width(), image.height(), pixels)
        .expect("Decoded icon dimensions are valid")
}
