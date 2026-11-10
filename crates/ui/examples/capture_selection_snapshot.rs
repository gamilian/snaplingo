//! Shared selection/magnifier inspection against the actual native capture PNG.
use snaplingo_ui::render::{Renderer, RendererTextMetrics};
use snaplingo_ui::{capture::*, Point, RasterImage, Rect, Size};
fn main() -> Result<(), String> {
    let directory = std::env::args()
        .nth(1)
        .unwrap_or("target/shared-capture".into());
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let mut rgba = Vec::with_capacity(800 * 800 * 4);
    for y in 0..800 {
        for x in 0..800 {
            rgba.extend_from_slice(&match (x < 400, y < 400) {
                (true, true) => [240, 40, 40, 255],
                (false, true) => [40, 240, 40, 255],
                (true, false) => [40, 40, 240, 255],
                _ => [240, 240, 40, 255],
            });
        }
    }
    let mut snapshot = CaptureSnapshot {
        size: Size {
            width: 400.,
            height: 400.,
        },
        images: vec![CaptureImage {
            rect: Rect {
                x: 0.,
                y: 0.,
                width: 400.,
                height: 400.,
            },
            image: RasterImage::from_rgba(800, 800, rgba).unwrap(),
        }],
        hint: "拖动选区或点击窗口 · Enter 复制截图 · Tab 界面元素 · Alt 放大镜 · Esc 取消".into(),
        ..Default::default()
    };
    let mut renderer = Renderer::default();
    let mut ui = CaptureUi::default();
    let mut metrics = RendererTextMetrics::default();
    // Native selection paints its mask only after hover or a drag starts.
    renderer
        .render(&ui.frame_with_text(&snapshot, &mut metrics).scene, 2.)?
        .save_png(format!("{directory}/desktop.png"))?;
    snapshot.mask_color = [0, 0, 0, 46];
    snapshot.desktop_scale = 2.;
    snapshot.selection = Some(Rect {
        x: 50.,
        y: 40.,
        width: 220.,
        height: 100.,
    });
    snapshot.selection_tinted = true;
    snapshot.selection_size = "440 x 200 px".into();
    snapshot.point = Some(Point { x: 80., y: 70. });
    snapshot.magnifier = Some(CaptureImage {
        rect: Rect {
            x: 94.,
            y: 84.,
            width: 228.,
            height: 132.,
        },
        image: RasterImage::from_rgba(456, 264, [240, 40, 40, 255].repeat(456 * 264)).unwrap(),
    });
    snapshot.sampled_color = Some("#F02828".into());
    snapshot.magnifier_label =
        "(-320, -30)\n#F02828\n按 C 复制颜色值\n按 Shift 切换 RGB / HEX".into();
    renderer
        .render(&ui.frame_with_text(&snapshot, &mut metrics).scene, 2.)?
        .save_png(format!("{directory}/selection-lens.png"))?;
    println!("Rendered native selection inspection into {directory}");
    Ok(())
}
