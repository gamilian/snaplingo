#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use arboard::ImageData;
    use chrono::TimeZone;
    use image::ImageEncoder;

    use crate::application::capture::output::{
        configured_capture_save_dir, configured_capture_save_path,
    };
    use crate::application::capture::CaptureOutput;

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
        let output = CaptureOutput::new();
        let path = temp_png_path();
        let png = vec![137, 80, 78, 71, 13, 10, 26, 10];

        let saved_path = output.save_png(&png, &path).await.unwrap();

        assert_eq!(saved_path, path);
        assert_eq!(std::fs::read(&saved_path).unwrap(), png);

        let _ = std::fs::remove_file(saved_path);
    }

    #[tokio::test]
    async fn save_image_encodes_jpeg_and_webp() {
        let output = CaptureOutput::new();
        let png = make_test_png(3, 2);
        let directory = tempfile::tempdir().unwrap();

        for (format, extension) in [("jpg", "jpg"), ("webp", "webp")] {
            let path = directory.path().join(format!("capture.{extension}"));
            output.save_image(&png, &path, format, 72).await.unwrap();

            let saved = std::fs::read(path).unwrap();
            let decoded = image::load_from_memory(&saved).unwrap();
            assert_eq!((decoded.width(), decoded.height()), (3, 2));
        }
    }

    #[test]
    fn configured_capture_path_uses_format_and_naming_rule() {
        let now = chrono::Local
            .with_ymd_and_hms(2026, 7, 14, 13, 30, 45)
            .unwrap();
        let base = std::path::Path::new("/tmp/captures");

        assert_eq!(
            configured_capture_save_path(base, "jpg", "timestamp", "", now).to_string_lossy(),
            "/tmp/captures/SnapLingo-20260714-133045.jpg"
        );
        assert_eq!(
            configured_capture_save_path(base, "webp", "date", "", now).to_string_lossy(),
            "/tmp/captures/SnapLingo-2026-07-14.webp"
        );
        assert_eq!(
            configured_capture_save_path(base, "png", "custom", "Work/Notes", now)
                .to_string_lossy(),
            "/tmp/captures/Work_Notes.png"
        );
    }

    #[test]
    fn configured_capture_save_dir_expands_home_prefix() {
        let dir = configured_capture_save_dir(
            "~/Pictures/SnapLingo",
            std::path::Path::new("/Users/alice"),
        );

        assert_eq!(
            dir,
            std::path::PathBuf::from("/Users/alice/Pictures/SnapLingo")
        );
    }

    #[test]
    fn decodes_png_to_clipboard_image_data() {
        let png = make_test_png(3, 2);

        let image = CaptureOutput::png_to_clipboard_image(&png).unwrap();

        assert_eq!(image.width, 3);
        assert_eq!(image.height, 2);
        assert_eq!(image.bytes.len(), 3 * 2 * 4);
    }

    #[test]
    fn encodes_clipboard_image_data_to_png() {
        let image = ImageData {
            width: 2,
            height: 1,
            bytes: std::borrow::Cow::Owned(vec![255, 0, 0, 255, 0, 255, 0, 255]),
        };

        let png = CaptureOutput::clipboard_image_to_png(image).unwrap();
        let decoded = image::load_from_memory(&png).unwrap().to_rgba8();

        assert_eq!(decoded.width(), 2);
        assert_eq!(decoded.height(), 1);
        assert_eq!(decoded.into_raw(), vec![255, 0, 0, 255, 0, 255, 0, 255]);
    }
}
