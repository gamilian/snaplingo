#[cfg(test)]
mod tests {
    use image::ImageEncoder;

    use crate::application::services::image_composition_service::ImageCompositionService;
    use crate::domain::capture::PhysicalRect;

    fn make_test_png(width: u32, height: u32) -> Vec<u8> {
        let pixels = vec![255; (width * height * 4) as usize];
        let mut png = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut png);
        encoder
            .write_image(&pixels, width, height, image::ExtendedColorType::Rgba8)
            .unwrap();
        png
    }

    fn png_dimensions(png: &[u8]) -> (u32, u32) {
        let image = image::load_from_memory(png).unwrap();
        (image.width(), image.height())
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
}
