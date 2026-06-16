#[cfg(test)]
mod tests {
    use image::ImageEncoder;

    use crate::application::services::image_composition_service::{
        ImageAnnotation, ImageCompositionService, PngPlacement,
    };
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

    fn count_pixels_with_color(png: &[u8], rgba: [u8; 4]) -> usize {
        image::load_from_memory(png)
            .unwrap()
            .to_rgba8()
            .pixels()
            .filter(|pixel| pixel.0 == rgba)
            .count()
    }

    #[test]
    fn crops_png_to_physical_rect() {
        let service = ImageCompositionService::new();
        let png = make_test_png(4, 4);
        let rect = PhysicalRect {
            x: 1,
            y: 1,
            width: 2,
            height: 2,
        };

        let cropped = service.crop_png(&png, &rect).unwrap();

        assert_eq!(png_dimensions(&cropped), (2, 2));
    }

    #[test]
    fn composes_cropped_pngs_into_one_output() {
        let service = ImageCompositionService::new();
        let red = make_solid_png(4, 2, [255, 0, 0, 255]);
        let blue = make_solid_png(4, 2, [0, 0, 255, 255]);

        let output = service
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
        let service = ImageCompositionService::new();
        let white = make_solid_png(6, 6, [255, 255, 255, 255]);

        let output = service
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
                }],
            )
            .unwrap();

        assert_eq!(png_pixel(&output, 1, 1), [255, 0, 0, 255]);
        assert_eq!(png_pixel(&output, 4, 3), [255, 0, 0, 255]);
        assert_eq!(png_pixel(&output, 2, 2), [255, 255, 255, 255]);
    }

    #[test]
    fn composes_png_with_ellipse_annotation() {
        let service = ImageCompositionService::new();
        let white = make_solid_png(9, 7, [255, 255, 255, 255]);

        let output = service
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
    fn composes_png_with_arrow_annotation() {
        let service = ImageCompositionService::new();
        let white = make_solid_png(12, 9, [255, 255, 255, 255]);

        let output = service
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
        let service = ImageCompositionService::new();
        let white = make_solid_png(8, 6, [255, 255, 255, 255]);

        let output = service
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
        let service = ImageCompositionService::new();
        let white = make_solid_png(8, 8, [255, 255, 255, 255]);

        let output = service
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
        let service = ImageCompositionService::new();
        let white = make_solid_png(8, 6, [255, 255, 255, 255]);

        let output = service
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
        let service = ImageCompositionService::new();
        let png = make_png_from_pixels(
            4,
            3,
            &[
                255, 0, 0, 255, 0, 255, 0, 255, 10, 20, 30, 255, 11, 21, 31, 255, 0, 0, 255, 255,
                255, 255, 255, 255, 12, 22, 32, 255, 13, 23, 33, 255, 90, 91, 92, 255, 91, 92, 93,
                255, 92, 93, 94, 255, 93, 94, 95, 255,
            ],
        );

        let output = service
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
                    rect: PhysicalRect {
                        x: 0,
                        y: 0,
                        width: 2,
                        height: 2,
                    },
                    block_size: 2,
                }],
            )
            .unwrap();

        assert_eq!(png_pixel(&output, 0, 0), [127, 127, 127, 255]);
        assert_eq!(png_pixel(&output, 1, 1), [127, 127, 127, 255]);
        assert_eq!(png_pixel(&output, 2, 0), [10, 20, 30, 255]);
        assert_eq!(png_pixel(&output, 0, 2), [90, 91, 92, 255]);
    }

    #[test]
    fn composes_png_with_blur_annotation() {
        let service = ImageCompositionService::new();
        let png = make_png_from_pixels(
            5,
            1,
            &[
                10, 10, 10, 255, 0, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255, 20, 20, 20,
                255,
            ],
        );

        let output = service
            .compose_png_with_annotations(
                5,
                1,
                &[PngPlacement {
                    png_data: png.as_slice(),
                    source_rect: PhysicalRect {
                        x: 0,
                        y: 0,
                        width: 5,
                        height: 1,
                    },
                    destination_rect: PhysicalRect {
                        x: 0,
                        y: 0,
                        width: 5,
                        height: 1,
                    },
                }],
                &[ImageAnnotation::Blur {
                    rect: PhysicalRect {
                        x: 1,
                        y: 0,
                        width: 3,
                        height: 1,
                    },
                    radius: 2,
                }],
            )
            .unwrap();

        let blurred_center = png_pixel(&output, 2, 0);
        assert_ne!(blurred_center, [255, 255, 255, 255]);
        assert!(blurred_center[0] > 0);
        assert_eq!(png_pixel(&output, 0, 0), [10, 10, 10, 255]);
        assert_eq!(png_pixel(&output, 4, 0), [20, 20, 20, 255]);
    }

    #[test]
    fn composes_png_with_text_annotation() {
        let service = ImageCompositionService::new();
        let white = make_solid_png(80, 32, [255, 255, 255, 255]);

        let output = service
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

        assert!(count_pixels_with_color(&output, [255, 0, 0, 255]) > 8);
        assert_eq!(png_pixel(&output, 79, 31), [255, 255, 255, 255]);
    }
}
