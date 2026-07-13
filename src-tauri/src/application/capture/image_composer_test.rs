#[cfg(test)]
mod tests {
    use image::ImageEncoder;

    use crate::application::capture::{CaptureImageComposer, ImageAnnotation, PngPlacement};
    use crate::domain::capture::{PhysicalPoint, PhysicalRect};

    fn make_solid_png(width: u32, height: u32, rgba: [u8; 4]) -> Vec<u8> {
        let pixels = rgba.repeat((width * height) as usize);
        make_png_from_pixels(width, height, &pixels)
    }

    fn make_png_from_pixels(width: u32, height: u32, pixels: &[u8]) -> Vec<u8> {
        let mut png = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut png);
        encoder
            .write_image(&pixels, width, height, image::ExtendedColorType::Rgba8)
            .unwrap();
        png
    }

    fn make_test_png(width: u32, height: u32) -> Vec<u8> {
        make_solid_png(width, height, [255, 255, 255, 255])
    }

    fn png_dimensions(png: &[u8]) -> (u32, u32) {
        let image = image::load_from_memory(png).unwrap();
        (image.width(), image.height())
    }

    fn png_pixel(png: &[u8], x: u32, y: u32) -> [u8; 4] {
        let image = image::load_from_memory(png).unwrap().to_rgba8();
        image.get_pixel(x, y).0
    }

    fn count_pixels_matching(png: &[u8], matches: impl Fn([u8; 4]) -> bool) -> usize {
        image::load_from_memory(png)
            .unwrap()
            .to_rgba8()
            .pixels()
            .filter(|pixel| matches(pixel.0))
            .count()
    }

    fn count_non_white_pixels(png: &[u8]) -> usize {
        image::load_from_memory(png)
            .unwrap()
            .to_rgba8()
            .pixels()
            .filter(|pixel| pixel.0 != [255, 255, 255, 255])
            .count()
    }

    #[test]
    fn crops_png_to_physical_rect() {
        let composer = CaptureImageComposer::new();
        let png = make_test_png(4, 4);
        let rect = PhysicalRect {
            x: 1,
            y: 1,
            width: 2,
            height: 2,
        };

        let cropped = composer.crop_png(&png, &rect).unwrap();

        assert_eq!(png_dimensions(&cropped), (2, 2));
    }

    #[test]
    fn composes_cropped_pngs_into_one_output() {
        let composer = CaptureImageComposer::new();
        let red = make_solid_png(4, 2, [255, 0, 0, 255]);
        let blue = make_solid_png(4, 2, [0, 0, 255, 255]);

        let output = composer
            .compose_png(
                6,
                2,
                &[
                    PngPlacement {
                        png_data: red.as_slice(),
                        source_rect: PhysicalRect {
                            x: 1,
                            y: 0,
                            width: 3,
                            height: 2,
                        },
                        destination_rect: PhysicalRect {
                            x: 0,
                            y: 0,
                            width: 3,
                            height: 2,
                        },
                    },
                    PngPlacement {
                        png_data: blue.as_slice(),
                        source_rect: PhysicalRect {
                            x: 0,
                            y: 0,
                            width: 3,
                            height: 2,
                        },
                        destination_rect: PhysicalRect {
                            x: 3,
                            y: 0,
                            width: 3,
                            height: 2,
                        },
                    },
                ],
            )
            .unwrap();

        assert_eq!(png_dimensions(&output), (6, 2));
        assert_eq!(png_pixel(&output, 0, 0), [255, 0, 0, 255]);
        assert_eq!(png_pixel(&output, 2, 1), [255, 0, 0, 255]);
        assert_eq!(png_pixel(&output, 3, 0), [0, 0, 255, 255]);
        assert_eq!(png_pixel(&output, 5, 1), [0, 0, 255, 255]);
    }

    #[test]
    fn composes_png_with_rectangle_annotation() {
        let composer = CaptureImageComposer::new();
        let white = make_solid_png(6, 6, [255, 255, 255, 255]);

        let output = composer
            .compose_png_with_annotations(
                6,
                6,
                &[PngPlacement {
                    png_data: white.as_slice(),
                    source_rect: PhysicalRect {
                        x: 0,
                        y: 0,
                        width: 6,
                        height: 6,
                    },
                    destination_rect: PhysicalRect {
                        x: 0,
                        y: 0,
                        width: 6,
                        height: 6,
                    },
                }],
                &[ImageAnnotation::Rectangle {
                    rect: PhysicalRect {
                        x: 1,
                        y: 1,
                        width: 4,
                        height: 3,
                    },
                    color: [255, 0, 0, 255],
                    stroke_width: 1,
                    filled: false,
                }],
            )
            .unwrap();

        assert_eq!(png_pixel(&output, 1, 1), [255, 0, 0, 255]);
        assert_eq!(png_pixel(&output, 4, 3), [255, 0, 0, 255]);
        assert_eq!(png_pixel(&output, 2, 2), [255, 255, 255, 255]);
    }

    #[test]
    fn composes_png_with_filled_rectangle_annotation() {
        let composer = CaptureImageComposer::new();
        let white = make_solid_png(6, 6, [255, 255, 255, 255]);

        let output = composer
            .compose_png_with_annotations(
                6,
                6,
                &[PngPlacement {
                    png_data: white.as_slice(),
                    source_rect: PhysicalRect {
                        x: 0,
                        y: 0,
                        width: 6,
                        height: 6,
                    },
                    destination_rect: PhysicalRect {
                        x: 0,
                        y: 0,
                        width: 6,
                        height: 6,
                    },
                }],
                &[ImageAnnotation::Rectangle {
                    rect: PhysicalRect {
                        x: 1,
                        y: 1,
                        width: 4,
                        height: 3,
                    },
                    color: [255, 0, 0, 255],
                    stroke_width: 1,
                    filled: true,
                }],
            )
            .unwrap();

        assert_eq!(png_pixel(&output, 2, 2), [255, 0, 0, 255]);
    }

    #[test]
    fn composes_png_with_ellipse_annotation() {
        let composer = CaptureImageComposer::new();
        let white = make_solid_png(9, 7, [255, 255, 255, 255]);

        let output = composer
            .compose_png_with_annotations(
                9,
                7,
                &[PngPlacement {
                    png_data: white.as_slice(),
                    source_rect: PhysicalRect {
                        x: 0,
                        y: 0,
                        width: 9,
                        height: 7,
                    },
                    destination_rect: PhysicalRect {
                        x: 0,
                        y: 0,
                        width: 9,
                        height: 7,
                    },
                }],
                &[ImageAnnotation::Ellipse {
                    rect: PhysicalRect {
                        x: 1,
                        y: 1,
                        width: 7,
                        height: 5,
                    },
                    color: [255, 0, 0, 255],
                    stroke_width: 1,
                    filled: false,
                }],
            )
            .unwrap();

        assert_eq!(png_pixel(&output, 1, 3), [255, 0, 0, 255]);
        assert_eq!(png_pixel(&output, 7, 3), [255, 0, 0, 255]);
        assert_eq!(png_pixel(&output, 4, 1), [255, 0, 0, 255]);
        assert_eq!(png_pixel(&output, 4, 5), [255, 0, 0, 255]);
        assert_eq!(png_pixel(&output, 4, 3), [255, 255, 255, 255]);
    }

    #[test]
    fn composes_png_with_filled_ellipse_annotation() {
        let composer = CaptureImageComposer::new();
        let white = make_solid_png(9, 7, [255, 255, 255, 255]);

        let output = composer
            .compose_png_with_annotations(
                9,
                7,
                &[PngPlacement {
                    png_data: white.as_slice(),
                    source_rect: PhysicalRect {
                        x: 0,
                        y: 0,
                        width: 9,
                        height: 7,
                    },
                    destination_rect: PhysicalRect {
                        x: 0,
                        y: 0,
                        width: 9,
                        height: 7,
                    },
                }],
                &[ImageAnnotation::Ellipse {
                    rect: PhysicalRect {
                        x: 1,
                        y: 1,
                        width: 7,
                        height: 5,
                    },
                    color: [255, 0, 0, 255],
                    stroke_width: 1,
                    filled: true,
                }],
            )
            .unwrap();

        assert_eq!(png_pixel(&output, 4, 3), [255, 0, 0, 255]);
    }

    #[test]
    fn composes_png_with_arrow_annotation() {
        let composer = CaptureImageComposer::new();
        let white = make_solid_png(12, 9, [255, 255, 255, 255]);

        let output = composer
            .compose_png_with_annotations(
                12,
                9,
                &[PngPlacement {
                    png_data: white.as_slice(),
                    source_rect: PhysicalRect {
                        x: 0,
                        y: 0,
                        width: 12,
                        height: 9,
                    },
                    destination_rect: PhysicalRect {
                        x: 0,
                        y: 0,
                        width: 12,
                        height: 9,
                    },
                }],
                &[ImageAnnotation::Arrow {
                    start: PhysicalPoint { x: 1, y: 4 },
                    end: PhysicalPoint { x: 10, y: 4 },
                    color: [255, 0, 0, 255],
                    stroke_width: 1,
                }],
            )
            .unwrap();

        assert_eq!(png_pixel(&output, 1, 4), [255, 0, 0, 255]);
        assert_eq!(png_pixel(&output, 10, 4), [255, 0, 0, 255]);
        assert_eq!(png_pixel(&output, 5, 1), [255, 0, 0, 255]);
        assert_eq!(png_pixel(&output, 5, 7), [255, 0, 0, 255]);
    }

    #[test]
    fn composes_png_with_line_annotation() {
        let composer = CaptureImageComposer::new();
        let white = make_solid_png(8, 6, [255, 255, 255, 255]);

        let output = composer
            .compose_png_with_annotations(
                8,
                6,
                &[PngPlacement {
                    png_data: white.as_slice(),
                    source_rect: PhysicalRect {
                        x: 0,
                        y: 0,
                        width: 8,
                        height: 6,
                    },
                    destination_rect: PhysicalRect {
                        x: 0,
                        y: 0,
                        width: 8,
                        height: 6,
                    },
                }],
                &[ImageAnnotation::Line {
                    start: PhysicalPoint { x: 1, y: 3 },
                    end: PhysicalPoint { x: 6, y: 3 },
                    color: [255, 0, 0, 255],
                    stroke_width: 1,
                }],
            )
            .unwrap();

        assert_eq!(png_pixel(&output, 1, 3), [255, 0, 0, 255]);
        assert_eq!(png_pixel(&output, 6, 3), [255, 0, 0, 255]);
        assert_eq!(png_pixel(&output, 4, 3), [255, 0, 0, 255]);
        assert_eq!(png_pixel(&output, 4, 1), [255, 255, 255, 255]);
    }

    #[test]
    fn composes_png_with_freehand_annotation() {
        let composer = CaptureImageComposer::new();
        let white = make_solid_png(8, 8, [255, 255, 255, 255]);

        let output = composer
            .compose_png_with_annotations(
                8,
                8,
                &[PngPlacement {
                    png_data: white.as_slice(),
                    source_rect: PhysicalRect {
                        x: 0,
                        y: 0,
                        width: 8,
                        height: 8,
                    },
                    destination_rect: PhysicalRect {
                        x: 0,
                        y: 0,
                        width: 8,
                        height: 8,
                    },
                }],
                &[ImageAnnotation::Freehand {
                    points: vec![
                        PhysicalPoint { x: 1, y: 1 },
                        PhysicalPoint { x: 4, y: 1 },
                        PhysicalPoint { x: 4, y: 5 },
                    ],
                    color: [255, 0, 0, 255],
                    stroke_width: 1,
                }],
            )
            .unwrap();

        assert_eq!(png_pixel(&output, 1, 1), [255, 0, 0, 255]);
        assert_eq!(png_pixel(&output, 4, 1), [255, 0, 0, 255]);
        assert_eq!(png_pixel(&output, 4, 5), [255, 0, 0, 255]);
        assert_eq!(png_pixel(&output, 2, 4), [255, 255, 255, 255]);
    }

    #[test]
    fn composes_png_with_highlight_annotation() {
        let composer = CaptureImageComposer::new();
        let white = make_solid_png(8, 6, [255, 255, 255, 255]);

        let output = composer
            .compose_png_with_annotations(
                8,
                6,
                &[PngPlacement {
                    png_data: white.as_slice(),
                    source_rect: PhysicalRect {
                        x: 0,
                        y: 0,
                        width: 8,
                        height: 6,
                    },
                    destination_rect: PhysicalRect {
                        x: 0,
                        y: 0,
                        width: 8,
                        height: 6,
                    },
                }],
                &[ImageAnnotation::Highlight {
                    points: vec![PhysicalPoint { x: 1, y: 3 }, PhysicalPoint { x: 6, y: 3 }],
                    color: [255, 0, 0, 128],
                    stroke_width: 1,
                }],
            )
            .unwrap();

        assert_eq!(png_pixel(&output, 1, 3), [255, 127, 127, 255]);
        assert_eq!(png_pixel(&output, 6, 3), [255, 127, 127, 255]);
        assert_eq!(png_pixel(&output, 4, 3), [255, 127, 127, 255]);
        assert_eq!(png_pixel(&output, 4, 1), [255, 255, 255, 255]);
    }

    #[test]
    fn composes_png_with_mosaic_annotation() {
        let composer = CaptureImageComposer::new();
        let png = make_png_from_pixels(
            4,
            3,
            &[
                255, 0, 0, 255, 0, 255, 0, 255, 10, 20, 30, 255, 11, 21, 31, 255, 0, 0, 255, 255,
                255, 255, 255, 255, 12, 22, 32, 255, 13, 23, 33, 255, 90, 91, 92, 255, 91, 92, 93,
                255, 92, 93, 94, 255, 93, 94, 95, 255,
            ],
        );

        let output = composer
            .compose_png_with_annotations(
                4,
                3,
                &[PngPlacement {
                    png_data: png.as_slice(),
                    source_rect: PhysicalRect {
                        x: 0,
                        y: 0,
                        width: 4,
                        height: 3,
                    },
                    destination_rect: PhysicalRect {
                        x: 0,
                        y: 0,
                        width: 4,
                        height: 3,
                    },
                }],
                &[ImageAnnotation::Mosaic {
                    points: vec![PhysicalPoint { x: 1, y: 1 }],
                    stroke_width: 4,
                    block_size: 2,
                }],
            )
            .unwrap();

        assert_eq!(png_pixel(&output, 0, 0), [127, 127, 127, 255]);
        assert_eq!(png_pixel(&output, 1, 1), [127, 127, 127, 255]);
        assert_eq!(png_pixel(&output, 3, 0), [11, 21, 31, 255]);
        assert_eq!(png_pixel(&output, 3, 2), [93, 94, 95, 255]);
    }

    #[test]
    fn eraser_restores_only_the_brushed_pixels() {
        let composer = CaptureImageComposer::new();
        let png = make_solid_png(5, 3, [12, 34, 56, 255]);

        let output = composer
            .compose_png_with_annotations(
                5,
                3,
                &[PngPlacement {
                    png_data: png.as_slice(),
                    source_rect: PhysicalRect {
                        x: 0,
                        y: 0,
                        width: 5,
                        height: 3,
                    },
                    destination_rect: PhysicalRect {
                        x: 0,
                        y: 0,
                        width: 5,
                        height: 3,
                    },
                }],
                &[
                    ImageAnnotation::Line {
                        start: PhysicalPoint { x: 0, y: 1 },
                        end: PhysicalPoint { x: 4, y: 1 },
                        color: [255, 77, 79, 255],
                        stroke_width: 1,
                    },
                    ImageAnnotation::Eraser {
                        points: vec![PhysicalPoint { x: 2, y: 1 }],
                        stroke_width: 3,
                    },
                ],
            )
            .unwrap();

        assert_eq!(png_pixel(&output, 2, 1), [12, 34, 56, 255]);
        assert_eq!(png_pixel(&output, 4, 1), [255, 77, 79, 255]);
    }

    #[test]
    fn composes_png_with_text_annotation() {
        let composer = CaptureImageComposer::new();
        let white = make_solid_png(80, 32, [255, 255, 255, 255]);

        let output = composer
            .compose_png_with_annotations(
                80,
                32,
                &[PngPlacement {
                    png_data: white.as_slice(),
                    source_rect: PhysicalRect {
                        x: 0,
                        y: 0,
                        width: 80,
                        height: 32,
                    },
                    destination_rect: PhysicalRect {
                        x: 0,
                        y: 0,
                        width: 80,
                        height: 32,
                    },
                }],
                &[ImageAnnotation::Text {
                    position: PhysicalPoint { x: 4, y: 22 },
                    text: "Snap".to_string(),
                    color: [255, 0, 0, 255],
                    font_size: 18,
                }],
            )
            .unwrap();

        assert!(count_non_white_pixels(&output) > 8);
        assert_eq!(png_pixel(&output, 79, 31), [255, 255, 255, 255]);
    }

    #[test]
    fn composes_png_with_chinese_text_annotation() {
        let composer = CaptureImageComposer::new();
        let white = make_solid_png(96, 40, [255, 255, 255, 255]);

        let output = composer
            .compose_png_with_annotations(
                96,
                40,
                &[PngPlacement {
                    png_data: white.as_slice(),
                    source_rect: PhysicalRect {
                        x: 0,
                        y: 0,
                        width: 96,
                        height: 40,
                    },
                    destination_rect: PhysicalRect {
                        x: 0,
                        y: 0,
                        width: 96,
                        height: 40,
                    },
                }],
                &[ImageAnnotation::Text {
                    position: PhysicalPoint { x: 4, y: 28 },
                    text: "中文".to_string(),
                    color: [255, 0, 0, 255],
                    font_size: 24,
                }],
            )
            .unwrap();

        assert!(count_non_white_pixels(&output) > 8);
    }

    #[test]
    fn text_annotation_respects_position() {
        let composer = CaptureImageComposer::new();
        let white = make_solid_png(120, 60, [255, 255, 255, 255]);

        let output = composer
            .compose_png_with_annotations(
                120,
                60,
                &[PngPlacement {
                    png_data: white.as_slice(),
                    source_rect: PhysicalRect {
                        x: 0,
                        y: 0,
                        width: 120,
                        height: 60,
                    },
                    destination_rect: PhysicalRect {
                        x: 0,
                        y: 0,
                        width: 120,
                        height: 60,
                    },
                }],
                &[ImageAnnotation::Text {
                    position: PhysicalPoint { x: 40, y: 42 },
                    text: "Snap".to_string(),
                    color: [255, 0, 0, 255],
                    font_size: 18,
                }],
            )
            .unwrap();

        let decoded = image::load_from_memory(&output).unwrap().to_rgba8();
        let mut left_edge_red_pixels = 0;
        for x in 0..20 {
            for y in 0..60 {
                let pixel = decoded.get_pixel(x, y).0;
                if pixel[0] > 200 && pixel[1] < 80 && pixel[2] < 80 {
                    left_edge_red_pixels += 1;
                }
            }
        }

        assert_eq!(left_edge_red_pixels, 0);
        assert!(
            count_pixels_matching(&output, |pixel| pixel[0] > 200
                && pixel[1] < 80
                && pixel[2] < 80)
                > 8
        );
    }

    #[test]
    fn renders_clipboard_text_as_pinned_png() {
        let composer = CaptureImageComposer::new();

        let output = composer
            .render_text_png("Hello from clipboard\nSnapLingo")
            .unwrap();
        let (width, height) = png_dimensions(&output);

        assert!(width >= 160);
        assert!(height >= 60);
        assert_eq!(png_pixel(&output, 0, 0), [255, 255, 255, 255]);
        assert!(count_non_white_pixels(&output) > 16);
    }

    #[test]
    fn renders_clipboard_hex_color_as_pinned_png() {
        let composer = CaptureImageComposer::new();

        let output = composer.render_clipboard_text_png("#0A141E").unwrap();
        let (width, height) = png_dimensions(&output);

        assert!(width >= 160);
        assert!(height >= 100);
        assert_eq!(png_pixel(&output, width / 2, height / 2), [10, 20, 30, 255]);
    }

    #[test]
    fn renders_clipboard_rgb_color_as_pinned_png() {
        let composer = CaptureImageComposer::new();

        let output = composer
            .render_clipboard_text_png("rgb(10, 20, 30)")
            .unwrap();
        let (width, height) = png_dimensions(&output);

        assert_eq!(png_pixel(&output, width / 2, height / 2), [10, 20, 30, 255]);
    }

    #[test]
    fn renders_clipboard_decimal_rgb_color_as_pinned_png() {
        let composer = CaptureImageComposer::new();

        let output = composer.render_clipboard_text_png("0.5 0.25 1.0").unwrap();
        let (width, height) = png_dimensions(&output);

        assert_eq!(
            png_pixel(&output, width / 2, height / 2),
            [128, 64, 255, 255]
        );
    }

    #[test]
    fn rejects_blank_clipboard_text_pngs() {
        let composer = CaptureImageComposer::new();

        assert!(composer.render_text_png(" \n\t ").is_err());
    }
}
