use snaplingo_ui::{capture::*, RasterImage, Rect, Size};

pub fn fixture() -> CaptureSnapshot {
    let mut snapshot = CaptureSnapshot {
        size: Size {
            width: 900.,
            height: 650.,
        },
        selection: Some(Rect {
            x: 8.,
            y: 8.,
            width: 700.,
            height: 200.,
        }),
        editing: true,
        tool: Some(CaptureTool::Rectangle),
        hint: String::new(),
        show_size: false,
        ..Default::default()
    };
    snapshot.style.stroke_width = 6;
    snapshot.style.corner_radius = 12.;
    snapshot.style.color = [90, 80, 70, 128];
    snapshot.style.presets = vec![
        [255, 77, 79, 255],
        [40, 167, 69, 255],
        [24, 144, 255, 255],
        [250, 219, 20, 255],
        [255, 255, 255, 255],
        [0, 0, 0, 255],
    ];
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../app/assets/toolbar");
    for name in [
        "select",
        "shapes",
        "rectangle",
        "ellipse",
        "arrow",
        "line",
        "pen",
        "text",
        "mosaic",
        "eraser",
        "highlight",
        "region-highlight",
        "blur",
        "spotlight",
        "undo",
        "redo",
        "favorite",
        "pin",
        "save",
        "copy",
    ] {
        let bytes = std::fs::read(root.join(format!("{name}.png"))).unwrap();
        let image = tiny_skia::Pixmap::decode_png(&bytes).unwrap();
        for (suffix, color) in ["", "-active", "-disabled", "-on"]
            .into_iter()
            .zip(icon_colors(snapshot.theme, snapshot.dark))
        {
            let mut pixels = image.data().to_vec();
            for pixel in pixels.chunks_exact_mut(4) {
                let alpha = (u16::from(pixel[3]) * u16::from(color[3]) / 255) as u8;
                pixel.copy_from_slice(&[color[0], color[1], color[2], alpha]);
            }
            snapshot.icons.insert(
                format!("{name}{suffix}"),
                RasterImage::from_rgba(image.width(), image.height(), pixels).unwrap(),
            );
        }
    }
    snapshot
}
