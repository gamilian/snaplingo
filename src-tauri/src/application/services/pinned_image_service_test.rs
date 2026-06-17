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

    #[test]
    fn returns_original_pinned_png_data() {
        let service = PinnedImageService::new();
        let png = make_test_png(2, 2);

        let image_id = service.pin_png(png.clone()).unwrap();

        assert_eq!(service.get_pinned_png(&image_id).unwrap(), png);

        service.remove_pinned_image(&image_id).unwrap();
        assert!(service.get_pinned_png(&image_id).is_err());
    }

    #[test]
    fn switches_between_existing_pinned_image_groups() {
        let service = PinnedImageService::new();
        let first_group_image = service.pin_png(make_test_png(1, 1)).unwrap();
        let second_group_image = service.pin_png(make_test_png(1, 1)).unwrap();

        service
            .move_pinned_image_to_group(&second_group_image, 1)
            .unwrap();

        let switch = service.switch_to_next_group().unwrap();

        assert_eq!(switch.previous_group, 0);
        assert_eq!(switch.next_group, 1);
        assert_eq!(switch.hide_image_ids, vec![first_group_image.clone()]);
        assert_eq!(switch.show_image_ids, vec![second_group_image.clone()]);

        let switch = service.switch_to_next_group().unwrap();

        assert_eq!(switch.previous_group, 1);
        assert_eq!(switch.next_group, 0);
        assert_eq!(switch.hide_image_ids, vec![second_group_image]);
        assert_eq!(switch.show_image_ids, vec![first_group_image]);
    }

    #[test]
    fn does_not_switch_groups_when_only_one_group_exists() {
        let service = PinnedImageService::new();

        service.pin_png(make_test_png(1, 1)).unwrap();

        assert!(service.switch_to_next_group().is_none());
    }
}
