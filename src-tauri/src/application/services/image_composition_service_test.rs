#[cfg(test)]
mod tests {
    use image::ImageEncoder;

    use crate::application::services::image_composition_service::{
        ImageCompositionService, PngPlacement,
    };
    use crate::domain::capture::PhysicalRect;

    fn make_solid_png(width: u32, height: u32, rgba: [u8; 4]) -> Vec<u8> {
        let pixels = rgba
            .repeat((width * height) as usize);
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
}
