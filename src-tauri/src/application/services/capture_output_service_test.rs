#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use image::ImageEncoder;

    use crate::application::services::capture_output_service::CaptureOutputService;

    fn make_test_png(width: u32, height: u32) -> Vec<u8> {
        let pixels = vec![255; (width * height * 4) as usize];
        let mut png = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut png);
        encoder
            .write_image(&pixels, width, height, image::ExtendedColorType::Rgba8)
            .unwrap();
        png
    }

    fn temp_png_path() -> std::path::PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir()
            .join("snaplingo-capture-output-tests")
            .join(format!("capture-{}.png", suffix))
    }

    #[tokio::test]
    async fn save_output_writes_png_to_path() {
        let service = CaptureOutputService::new();
        let path = temp_png_path();
        let png = vec![137, 80, 78, 71, 13, 10, 26, 10];

        let saved_path = service.save_png(&png, &path).await.unwrap();

        assert_eq!(saved_path, path);
        assert_eq!(std::fs::read(&saved_path).unwrap(), png);

        let _ = std::fs::remove_file(saved_path);
    }

    #[test]
    fn decodes_png_to_clipboard_image_data() {
        let png = make_test_png(3, 2);

        let image = CaptureOutputService::png_to_clipboard_image(&png).unwrap();

        assert_eq!(image.width, 3);
        assert_eq!(image.height, 2);
        assert_eq!(image.bytes.len(), 3 * 2 * 4);
    }
}
