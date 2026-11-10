use std::io::Cursor;

use super::{CaptureImageComposer, ImageAnnotation, PngPlacement};
use crate::capture::model::PhysicalPoint;
use crate::capture::model::PhysicalRect;

fn compose(original: image::RgbaImage, annotations: &[ImageAnnotation]) -> image::RgbaImage {
    let (width, height) = original.dimensions();
    let mut png = Vec::new();
    image::DynamicImage::ImageRgba8(original)
        .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    let full = PhysicalRect {
        x: 0,
        y: 0,
        width,
        height,
    };
    let bytes = CaptureImageComposer::new()
        .compose_png_with_annotations(
            width,
            height,
            &[PngPlacement {
                png_data: &png,
                source_rect: full.clone(),
                destination_rect: full,
            }],
            annotations,
        )
        .unwrap();
    image::load_from_memory(&bytes).unwrap().to_rgba8()
}

fn white() -> image::RgbaImage {
    image::RgbaImage::from_pixel(100, 100, image::Rgba([255, 255, 255, 255]))
}

#[test]
fn rounded_rectangle_preserves_corners_and_fills_the_center() {
    let output = compose(
        white(),
        &[ImageAnnotation::Rectangle {
            rect: PhysicalRect {
                x: 20,
                y: 20,
                width: 60,
                height: 60,
            },
            color: [255, 0, 0, 255],
            stroke_width: 2,
            filled: true,
            corner_radius: 20,
        }],
    );
    assert_eq!(output.get_pixel(20, 20).0, [255, 255, 255, 255]);
    assert_eq!(output.get_pixel(50, 50).0, [255, 0, 0, 255]);
}

#[test]
fn highlighter_retains_black_text_and_tints_the_white_background() {
    let mut source = white();
    source.put_pixel(50, 50, image::Rgba([0, 0, 0, 255]));
    let output = compose(
        source,
        &[ImageAnnotation::RegionHighlight {
            rect: PhysicalRect {
                x: 20,
                y: 20,
                width: 60,
                height: 60,
            },
            color: [255, 255, 0, 255],
            corner_radius: 8,
        }],
    );
    assert_eq!(output.get_pixel(50, 50).0, [0, 0, 0, 255]);
    assert!(output.get_pixel(40, 40)[2] < 150);
    assert_eq!(output.get_pixel(10, 10).0, [255, 255, 255, 255]);
}

#[test]
fn blur_smooths_contrast_without_changing_pixels_outside_the_region() {
    let source = image::RgbaImage::from_fn(100, 100, |x, _| {
        if x < 50 {
            image::Rgba([0, 0, 0, 255])
        } else {
            image::Rgba([255, 255, 255, 255])
        }
    });
    let output = compose(
        source.clone(),
        &[ImageAnnotation::Blur {
            rect: PhysicalRect {
                x: 20,
                y: 20,
                width: 60,
                height: 60,
            },
            radius: 8.0,
            corner_radius: 8,
        }],
    );
    assert!(output.get_pixel(49, 50)[0] > 60);
    assert!(output.get_pixel(50, 50)[0] < 200);
    assert_eq!(output.get_pixel(10, 50), source.get_pixel(10, 50));
    assert_eq!(output.get_pixel(90, 50), source.get_pixel(90, 50));
}

#[test]
fn two_spotlights_restore_both_regions_and_dim_the_remaining_image_once() {
    let output = compose(
        white(),
        &[
            ImageAnnotation::Spotlight {
                rect: PhysicalRect {
                    x: 10,
                    y: 10,
                    width: 30,
                    height: 30,
                },
                corner_radius: 10,
            },
            ImageAnnotation::Spotlight {
                rect: PhysicalRect {
                    x: 60,
                    y: 60,
                    width: 30,
                    height: 30,
                },
                corner_radius: 10,
            },
        ],
    );
    assert_eq!(output.get_pixel(25, 25).0, [255, 255, 255, 255]);
    assert_eq!(output.get_pixel(75, 75).0, [255, 255, 255, 255]);
    assert_eq!(output.get_pixel(50, 50).0, [135, 135, 135, 255]);
    assert_eq!(output.get_pixel(10, 10).0, [135, 135, 135, 255]);
}

#[test]
fn curved_arrow_is_exported_along_its_curve() {
    let output = compose(
        white(),
        &[ImageAnnotation::Arrow {
            start: PhysicalPoint { x: 10, y: 80 },
            end: PhysicalPoint { x: 90, y: 80 },
            control1: Some(PhysicalPoint { x: 20, y: 10 }),
            control2: Some(PhysicalPoint { x: 80, y: 10 }),
            color: [255, 0, 0, 255],
            stroke_width: 3,
        }],
    );
    assert_eq!(output.get_pixel(50, 28)[1], 0);
    assert_eq!(output.get_pixel(50, 80).0, [255, 255, 255, 255]);
}

#[test]
fn arrow_head_keeps_the_curve_direction_when_the_last_control_point_equals_the_tip() {
    let output = compose(
        white(),
        &[ImageAnnotation::Arrow {
            start: PhysicalPoint { x: 10, y: 20 },
            end: PhysicalPoint { x: 80, y: 40 },
            control1: Some(PhysicalPoint { x: 80, y: 80 }),
            control2: Some(PhysicalPoint { x: 80, y: 40 }),
            color: [255, 0, 0, 255],
            stroke_width: 3,
        }],
    );
    assert_eq!(output.get_pixel(83, 47).0, [255, 0, 0, 255]);
}

#[test]
fn bold_and_italic_text_styles_are_preserved_in_export() {
    let text = |bold, italic| ImageAnnotation::Text {
        position: PhysicalPoint { x: 10, y: 60 },
        text: "Hello".to_string(),
        color: [0, 0, 0, 255],
        font_size: 24,
        font_family: String::new(),
        bold,
        italic,
    };
    let regular = compose(white(), &[text(false, false)]);
    let bold = compose(white(), &[text(true, false)]);
    let italic = compose(white(), &[text(false, true)]);
    let ink = |image: &image::RgbaImage| {
        image
            .pixels()
            .map(|pixel| 255 - pixel[0] as u64)
            .sum::<u64>()
    };
    assert!(ink(&bold) > ink(&regular));
    assert_ne!(italic, regular);
}
