#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use image::ImageEncoder;

    use crate::application::capture::{
        CaptureImageComposer, CaptureOutput, ClipboardCaptureOutput,
    };

    use crate::application::pinned_image::{PinnedImageOpenRequest, PinnedImageState};

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
        let state = PinnedImageState::new();
        let png = make_test_png(3, 2);

        let image_id = state.pin_png(png.clone()).unwrap();
        let view = state.get_pinned_image(&image_id).unwrap();

        assert_eq!(view.id, image_id);
        assert_eq!(view.width, 3);
        assert_eq!(view.height, 2);
        assert_eq!(view.source_text, None);
        assert!(!view.image_base64.is_empty());
    }

    #[test]
    fn stores_optional_source_text_for_text_backed_pins() {
        let state = PinnedImageState::new();
        let png = make_test_png(3, 2);

        let image_id = state
            .pin_png_with_source_text(png, Some("Hello from clipboard".to_string()))
            .unwrap();
        let view = state.get_pinned_image(&image_id).unwrap();

        assert_eq!(view.source_text, Some("Hello from clipboard".to_string()));
    }

    #[test]
    fn pin_png_view_returns_stored_view() {
        let state = PinnedImageState::new();
        let png = make_test_png(3, 2);

        let view = state.pin_png_view(png.clone()).unwrap();

        assert_eq!(view.width, 3);
        assert_eq!(view.height, 2);
        assert_eq!(view.source_text, None);
        assert_eq!(state.get_pinned_png(&view.id).unwrap(), png);
    }

    #[test]
    fn pin_capture_output_view_stores_png_output() {
        let state = PinnedImageState::new();
        let image_composition = CaptureImageComposer::new();
        let png = make_test_png(3, 2);

        let view = state
            .pin_capture_output_view(&image_composition, ClipboardCaptureOutput::Png(png.clone()))
            .unwrap();

        assert_eq!(view.width, 3);
        assert_eq!(view.height, 2);
        assert_eq!(view.source_text, None);
        assert_eq!(state.get_pinned_png(&view.id).unwrap(), png);
    }

    #[test]
    fn removing_pinned_image_makes_it_unavailable() {
        let state = PinnedImageState::new();
        let image_id = state.pin_png(make_test_png(1, 1)).unwrap();

        state.remove_pinned_image(&image_id).unwrap();

        assert!(state.get_pinned_image(&image_id).is_err());
    }

    #[test]
    fn closing_pinned_image_keeps_it_recoverable_once() {
        let state = PinnedImageState::new();
        let image_id = state.pin_png(make_test_png(2, 2)).unwrap();

        state.close_pinned_image(&image_id).unwrap();

        assert!(state.get_pinned_image(&image_id).is_ok());
        assert_eq!(state.pop_recoverable_pinned_image(), Some(image_id));
        assert_eq!(state.pop_recoverable_pinned_image(), None);
    }

    #[test]
    fn pin_clipboard_capture_output_reopens_recoverable_image_first() {
        let state = PinnedImageState::new();
        let image_composition = CaptureImageComposer::new();
        let output = CaptureOutput::new();
        let image_id = state.pin_png(make_test_png(2, 2)).unwrap();
        state.close_pinned_image(&image_id).unwrap();

        let request = state
            .pin_clipboard_capture_output(&image_composition, &output)
            .unwrap();

        let PinnedImageOpenRequest::Reopen(image) = request else {
            panic!("expected recoverable pinned image to reopen");
        };
        assert_eq!(image.id, image_id);
    }

    #[test]
    fn removing_pinned_image_clears_it_from_recovery() {
        let state = PinnedImageState::new();
        let image_id = state.pin_png(make_test_png(2, 2)).unwrap();

        state.close_pinned_image(&image_id).unwrap();
        state.remove_pinned_image(&image_id).unwrap();

        assert_eq!(state.pop_recoverable_pinned_image(), None);
    }

    #[test]
    fn returns_original_pinned_png_data() {
        let state = PinnedImageState::new();
        let png = make_test_png(2, 2);

        let image_id = state.pin_png(png.clone()).unwrap();

        assert_eq!(state.get_pinned_png(&image_id).unwrap(), png);

        state.remove_pinned_image(&image_id).unwrap();
        assert!(state.get_pinned_png(&image_id).is_err());
    }

    #[tokio::test]
    async fn save_pinned_png_to_path_writes_original_png() {
        let state = PinnedImageState::new();
        let output = CaptureOutput::new();
        let png = make_test_png(2, 3);
        let image_id = state.pin_png(png.clone()).unwrap();
        let path = temp_png_path();

        state
            .save_pinned_png_to_path(&output, &image_id, &path)
            .await
            .unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), png);

        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn copy_pinned_png_to_clipboard_returns_missing_image_before_clipboard() {
        let state = PinnedImageState::new();
        let output = CaptureOutput::new();

        let error = state
            .copy_pinned_png_to_clipboard(&output, "missing")
            .await
            .unwrap_err();

        assert!(error
            .to_string()
            .contains("Pinned image not found: missing"));
    }

    #[test]
    fn replaces_pinned_png_while_preserving_image_id_and_group() {
        let state = PinnedImageState::new();
        let image_id = state.pin_png(make_test_png(2, 2)).unwrap();
        let peer_id = state.pin_png(make_test_png(1, 1)).unwrap();
        let replacement = make_test_png(4, 3);

        state.move_pinned_image_to_group(&image_id, 1).unwrap();
        state
            .replace_pinned_png(&image_id, replacement.clone())
            .unwrap();

        let view = state.get_pinned_image(&image_id).unwrap();
        let membership = state.pinned_image_group_containing(&image_id).unwrap();

        assert_eq!(view.id, image_id);
        assert_eq!(view.width, 4);
        assert_eq!(view.height, 3);
        assert_eq!(view.source_text, None);
        assert_eq!(state.get_pinned_png(&image_id).unwrap(), replacement);
        assert_eq!(membership.group, 1);
        assert_eq!(membership.image_ids, vec![image_id]);
        assert!(state.get_pinned_image(&peer_id).is_ok());
    }

    #[test]
    fn replace_pinned_png_view_returns_updated_view() {
        let state = PinnedImageState::new();
        let image_id = state.pin_png(make_test_png(2, 2)).unwrap();
        let replacement = make_test_png(4, 3);

        let view = state
            .replace_pinned_png_view(&image_id, replacement.clone())
            .unwrap();

        assert_eq!(view.id, image_id);
        assert_eq!(view.width, 4);
        assert_eq!(view.height, 3);
        assert_eq!(state.get_pinned_png(&image_id).unwrap(), replacement);
    }

    #[test]
    fn replace_pinned_text_as_png_returns_text_backed_view() {
        let state = PinnedImageState::new();
        let image_composition = CaptureImageComposer::new();
        let image_id = state.pin_png(make_test_png(2, 2)).unwrap();

        let view = state
            .replace_pinned_text_as_png_view(&image_composition, &image_id, "Replacement text")
            .unwrap();

        assert_eq!(view.id, image_id);
        assert_eq!(view.source_text, Some("Replacement text".to_string()));
        assert!(view.width >= 80);
        assert!(view.height >= 60);
        assert!(!state.get_pinned_png(&image_id).unwrap().is_empty());
    }

    #[test]
    fn replace_capture_output_view_stores_text_output_as_png() {
        let state = PinnedImageState::new();
        let image_composition = CaptureImageComposer::new();
        let image_id = state.pin_png(make_test_png(2, 2)).unwrap();

        let view = state
            .replace_capture_output_view(
                &image_composition,
                &image_id,
                ClipboardCaptureOutput::Text("Replacement text".to_string()),
            )
            .unwrap();

        assert_eq!(view.id, image_id);
        assert_eq!(view.source_text, Some("Replacement text".to_string()));
        assert!(view.width >= 80);
        assert!(view.height >= 60);
        assert!(!state.get_pinned_png(&image_id).unwrap().is_empty());
    }

    #[test]
    fn pin_text_as_png_returns_text_backed_view() {
        let state = PinnedImageState::new();
        let image_composition = CaptureImageComposer::new();

        let view = state
            .pin_text_as_png_view(&image_composition, "Hello pin")
            .unwrap();

        assert_eq!(view.source_text, Some("Hello pin".to_string()));
        assert!(view.width >= 80);
        assert!(view.height >= 60);
        assert!(!state.get_pinned_png(&view.id).unwrap().is_empty());
    }

    #[test]
    fn switches_between_existing_pinned_image_groups() {
        let state = PinnedImageState::new();
        let first_group_image = state.pin_png(make_test_png(1, 1)).unwrap();
        let second_group_image = state.pin_png(make_test_png(1, 1)).unwrap();

        state
            .move_pinned_image_to_group(&second_group_image, 1)
            .unwrap();

        let switch = state.switch_to_next_group().unwrap();

        assert_eq!(switch.previous_group, 0);
        assert_eq!(switch.next_group, 1);
        assert_eq!(switch.hide_image_ids, vec![first_group_image.clone()]);
        assert_eq!(switch.show_image_ids, vec![second_group_image.clone()]);

        let switch = state.switch_to_next_group().unwrap();

        assert_eq!(switch.previous_group, 1);
        assert_eq!(switch.next_group, 0);
        assert_eq!(switch.hide_image_ids, vec![second_group_image]);
        assert_eq!(switch.show_image_ids, vec![first_group_image]);
    }

    #[test]
    fn does_not_switch_groups_when_only_one_group_exists() {
        let state = PinnedImageState::new();

        state.pin_png(make_test_png(1, 1)).unwrap();

        assert!(state.switch_to_next_group().is_none());
    }

    #[test]
    fn moves_a_pinned_image_to_the_next_group() {
        let state = PinnedImageState::new();
        let first_group_image = state.pin_png(make_test_png(1, 1)).unwrap();
        let second_group_image = state.pin_png(make_test_png(1, 1)).unwrap();

        state
            .move_pinned_image_to_group(&second_group_image, 1)
            .unwrap();

        let next_group = state
            .move_pinned_image_to_next_group(&first_group_image)
            .unwrap();

        assert_eq!(next_group, 1);

        let switch = state.switch_to_next_group().unwrap();

        assert_eq!(switch.next_group, 1);
        assert_eq!(switch.hide_image_ids, Vec::<String>::new());
        assert_eq!(
            switch.show_image_ids,
            vec![first_group_image, second_group_image]
        );
    }

    #[test]
    fn moving_a_pinned_image_creates_another_group_when_needed() {
        let state = PinnedImageState::new();
        let image_id = state.pin_png(make_test_png(1, 1)).unwrap();

        let next_group = state.move_pinned_image_to_next_group(&image_id).unwrap();

        assert_eq!(next_group, 1);
        assert_eq!(
            state.switch_to_next_group().unwrap().show_image_ids,
            vec![image_id]
        );
    }

    #[test]
    fn removes_every_pinned_image_in_the_same_group() {
        let state = PinnedImageState::new();
        let first_group_image = state.pin_png(make_test_png(1, 1)).unwrap();
        let first_group_peer = state.pin_png(make_test_png(1, 1)).unwrap();
        let second_group_image = state.pin_png(make_test_png(1, 1)).unwrap();

        state
            .move_pinned_image_to_group(&second_group_image, 1)
            .unwrap();

        let removal = state
            .remove_pinned_image_group_containing(&first_group_image)
            .unwrap();
        let mut expected_removed_image_ids =
            vec![first_group_image.clone(), first_group_peer.clone()];
        expected_removed_image_ids.sort();

        assert_eq!(removal.removed_group, 0);
        assert_eq!(removal.removed_image_ids, expected_removed_image_ids);
        assert!(state.get_pinned_image(&first_group_image).is_err());
        assert!(state.get_pinned_image(&first_group_peer).is_err());
        assert!(state.get_pinned_image(&second_group_image).is_ok());
        assert!(state.switch_to_next_group().is_none());
    }

    #[test]
    fn finds_every_pinned_image_in_the_same_group() {
        let state = PinnedImageState::new();
        let first_group_image = state.pin_png(make_test_png(1, 1)).unwrap();
        let first_group_peer = state.pin_png(make_test_png(1, 1)).unwrap();
        let second_group_image = state.pin_png(make_test_png(1, 1)).unwrap();

        state
            .move_pinned_image_to_group(&second_group_image, 1)
            .unwrap();

        let membership = state
            .pinned_image_group_containing(&first_group_image)
            .unwrap();
        let mut expected_image_ids = vec![first_group_image, first_group_peer];
        expected_image_ids.sort();

        assert_eq!(membership.group, 0);
        assert_eq!(membership.image_ids, expected_image_ids);
    }

    fn temp_png_path() -> std::path::PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir()
            .join("snaplingo-pinned-output-tests")
            .join(format!("pin-{}.png", suffix))
    }
}
