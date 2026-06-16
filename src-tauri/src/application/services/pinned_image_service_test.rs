#[cfg(test)]
mod tests {
    use image::ImageEncoder;

    use crate::application::services::pinned_image_service::PinnedImageService;

    fn make_test_png(width: u32, height: u32) -> Vec<u8> {
        let pixels = vec![255; (width * height * 4) as usize];
        let mut png = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut png);
        encoder
            .write_image(&pixels, width, height, image::ExtendedColorType::Rgba8)
            .unwrap();
        png
    }

    #[test]
    fn stores_pinned_png_with_dimensions_and_base64_view() {
        let service = PinnedImageService::new();
        let png = make_test_png(3, 2);

        let image_id = service.pin_png(png.clone()).unwrap();
        let view = service.get_pinned_image(&image_id).unwrap();

        assert_eq!(view.id, image_id);
        assert_eq!(view.width, 3);
        assert_eq!(view.height, 2);
        assert!(!view.image_base64.is_empty());
    }

    #[test]
    fn removing_pinned_image_makes_it_unavailable() {
        let service = PinnedImageService::new();
        let image_id = service.pin_png(make_test_png(1, 1)).unwrap();

        service.remove_pinned_image(&image_id).unwrap();

        assert!(service.get_pinned_image(&image_id).is_err());
    }
}
