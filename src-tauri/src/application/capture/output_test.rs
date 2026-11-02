#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    use chrono::{DateTime, Local, TimeZone};
    use image::ImageEncoder;

    use crate::application::capture::output::{
        configured_capture_save_dir, configured_capture_save_path, CaptureOutputHost,
        CaptureOutputSystemPaths,
    };
    use crate::application::capture::{CaptureOutput, ClipboardCaptureOutput};
    use crate::{AppError, Result};

    struct RecordingCaptureOutputHost {
        paths: CaptureOutputSystemPaths,
        now: DateTime<Local>,
        existing_paths: Vec<PathBuf>,
        writes: Mutex<Vec<(PathBuf, Vec<u8>)>>,
        copied_pngs: Mutex<Vec<Vec<u8>>>,
        clipboard_png: Option<Vec<u8>>,
        clipboard_text: Option<String>,
    }

    impl RecordingCaptureOutputHost {
        fn new() -> Self {
            Self {
                paths: CaptureOutputSystemPaths {
                    download_dir: Some(PathBuf::from("/system/downloads")),
                    picture_dir: Some(PathBuf::from("/system/pictures")),
                    home_dir: Some(PathBuf::from("/system/home")),
                    temp_dir: PathBuf::from("/system/temp"),
                },
                now: Local.with_ymd_and_hms(2026, 7, 14, 13, 30, 45).unwrap(),
                existing_paths: Vec::new(),
                writes: Mutex::new(Vec::new()),
                copied_pngs: Mutex::new(Vec::new()),
                clipboard_png: None,
                clipboard_text: None,
            }
        }

        fn writes(&self) -> Vec<(PathBuf, Vec<u8>)> {
            self.writes.lock().unwrap().clone()
        }

        fn copied_pngs(&self) -> Vec<Vec<u8>> {
            self.copied_pngs.lock().unwrap().clone()
        }
    }

    impl CaptureOutputHost for RecordingCaptureOutputHost {
        fn system_paths(&self) -> CaptureOutputSystemPaths {
            self.paths.clone()
        }

        fn now(&self) -> DateTime<Local> {
            self.now
        }

        fn path_exists(&self, path: &Path) -> bool {
            self.existing_paths.iter().any(|existing| existing == path)
        }

        fn write_file(&self, path: &Path, data: &[u8]) -> Result<()> {
            self.writes
                .lock()
                .unwrap()
                .push((path.to_path_buf(), data.to_vec()));
            Ok(())
        }

        fn copy_png(&self, data: &[u8]) -> Result<()> {
            self.copied_pngs.lock().unwrap().push(data.to_vec());
            Ok(())
        }

        fn read_clipboard_png(&self) -> Result<Vec<u8>> {
            self.clipboard_png
                .clone()
                .ok_or_else(|| AppError::System("clipboard has no image".into()))
        }

        fn read_clipboard_text(&self) -> Result<String> {
            self.clipboard_text
                .clone()
                .ok_or_else(|| AppError::System("clipboard has no text".into()))
        }
    }

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
    fn default_capture_path_uses_system_directories_and_skips_existing_names() {
        let mut host = RecordingCaptureOutputHost::new();
        host.existing_paths = vec![
            PathBuf::from("/system/downloads/SnapLingo-20260714-133045.png"),
            PathBuf::from("/system/downloads/SnapLingo-20260714-133045_2.png"),
        ];
        let output = CaptureOutput::with_host(Arc::new(host));

        let path = output.default_capture_save_path(None, "png", "timestamp", "SnapLingo");

        assert_eq!(
            path,
            PathBuf::from("/system/downloads/SnapLingo-20260714-133045_3.png")
        );
    }

    #[test]
    fn quick_capture_path_uses_snaplingo_picture_directory() {
        let output = CaptureOutput::with_host(Arc::new(RecordingCaptureOutputHost::new()));

        let path = output.quick_capture_save_path(None, "webp", "date", "SnapLingo");

        assert_eq!(
            path,
            PathBuf::from("/system/pictures/SnapLingo/SnapLingo-2026-07-14.webp")
        );
    }

    #[test]
    fn missing_remembered_save_directory_falls_back_to_the_configured_directory() {
        let output = CaptureOutput::with_host(Arc::new(RecordingCaptureOutputHost::new()));
        let path = output.dialog_capture_save_path(
            Some("/removed"),
            Some("/configured"),
            "png",
            "custom",
            "region",
        );
        assert_eq!(path, PathBuf::from("/configured/region.png"));
    }

    #[test]
    fn configured_tilde_directory_uses_host_home_directory() {
        let output = CaptureOutput::with_host(Arc::new(RecordingCaptureOutputHost::new()));

        let path =
            output.default_capture_save_path(Some("~/Screenshots"), "png", "custom", "capture");

        assert_eq!(path, PathBuf::from("/system/home/Screenshots/capture.png"));
    }

    #[tokio::test]
    async fn save_output_writes_png_through_host() {
        let host = Arc::new(RecordingCaptureOutputHost::new());
        let output = CaptureOutput::with_host(host.clone());
        let path = PathBuf::from("/captures/capture.png");
        let png = vec![137, 80, 78, 71, 13, 10, 26, 10];

        let saved_path = output.save_png(&png, &path).await.unwrap();

        assert_eq!(saved_path, path);
        assert_eq!(host.writes(), vec![(path, png)]);
    }

    #[tokio::test]
    async fn save_image_encodes_jpeg_and_webp_before_writing() {
        let host = Arc::new(RecordingCaptureOutputHost::new());
        let output = CaptureOutput::with_host(host.clone());
        let png = make_test_png(3, 2);

        for (format, extension) in [("jpg", "jpg"), ("webp", "webp")] {
            let path = PathBuf::from(format!("/captures/capture.{extension}"));
            output.save_image(&png, &path, format, 72).await.unwrap();
        }

        let writes = host.writes();
        assert_eq!(writes.len(), 2);
        for (_, encoded) in writes {
            let decoded = image::load_from_memory(&encoded).unwrap();
            assert_eq!((decoded.width(), decoded.height()), (3, 2));
        }
    }

    #[tokio::test]
    async fn copy_png_uses_platform_host() {
        let host = Arc::new(RecordingCaptureOutputHost::new());
        let output = CaptureOutput::with_host(host.clone());
        let png = make_test_png(2, 1);

        output.copy_png(&png).await.unwrap();

        assert_eq!(host.copied_pngs(), vec![png]);
    }

    #[test]
    fn clipboard_capture_output_falls_back_from_image_to_text() {
        let mut host = RecordingCaptureOutputHost::new();
        host.clipboard_text = Some("Pinned text".to_string());
        let output = CaptureOutput::with_host(Arc::new(host));

        let clipboard = output.read_clipboard_capture_output().unwrap();

        assert!(matches!(
            clipboard,
            ClipboardCaptureOutput::Text(text) if text == "Pinned text"
        ));
    }

    #[test]
    fn configured_capture_path_uses_format_and_naming_rule() {
        let now = Local.with_ymd_and_hms(2026, 7, 14, 13, 30, 45).unwrap();
        let base = Path::new("/tmp/captures");
        let path_exists = |_: &Path| false;

        assert_eq!(
            configured_capture_save_path(base, "jpg", "timestamp", "", now, &path_exists)
                .to_string_lossy(),
            base.join("SnapLingo-20260714-133045.jpg").to_string_lossy()
        );
        assert_eq!(
            configured_capture_save_path(base, "webp", "date", "", now, &path_exists)
                .to_string_lossy(),
            base.join("SnapLingo-2026-07-14.webp").to_string_lossy()
        );
        assert_eq!(
            configured_capture_save_path(base, "png", "custom", "Work/Notes", now, &path_exists)
                .to_string_lossy(),
            base.join("Work_Notes.png").to_string_lossy()
        );
    }

    #[test]
    fn configured_capture_save_dir_expands_home_prefix() {
        let dir = configured_capture_save_dir("~/Pictures/SnapLingo", Path::new("/Users/alice"));

        assert_eq!(dir, PathBuf::from("/Users/alice/Pictures/SnapLingo"));
    }

    #[tokio::test]
    async fn filename_extension_selects_the_codec_and_webp_preserves_every_pixel() {
        let host = Arc::new(RecordingCaptureOutputHost::new());
        let output = CaptureOutput::with_host(host.clone());
        let pixels = image::RgbaImage::from_fn(20, 10, |x, y| {
            image::Rgba([(x * 11) as u8, (y * 23) as u8, ((x + y) * 7) as u8, 180])
        });
        let mut png = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png)
            .write_image(pixels.as_raw(), 20, 10, image::ExtendedColorType::Rgba8)
            .unwrap();
        output
            .save_image(&png, Path::new("image.webp"), "jpg", 50)
            .await
            .unwrap();
        let writes = host.writes();
        assert_eq!(
            image::guess_format(&writes[0].1).unwrap(),
            image::ImageFormat::WebP
        );
        assert_eq!(
            image::load_from_memory(&writes[0].1).unwrap().to_rgba8(),
            pixels
        );
        output
            .save_image(&png, Path::new("image.JPEG"), "png", 90)
            .await
            .unwrap();
        assert_eq!(
            image::guess_format(&host.writes()[1].1).unwrap(),
            image::ImageFormat::Jpeg
        );
        let saved = output
            .save_image(&png, Path::new("no-extension"), "png", 90)
            .await
            .unwrap();
        assert_eq!(saved, PathBuf::from("no-extension.png"));
        assert!(output
            .save_image(&png, Path::new("image.txt"), "png", 90)
            .await
            .is_err());
        assert_eq!(host.writes().len(), 3);
    }

    #[test]
    fn unix_millisecond_names_contain_only_the_timestamp_before_the_extension() {
        let now = Local.with_ymd_and_hms(2026, 7, 14, 13, 30, 45).unwrap();
        let path = configured_capture_save_path(
            Path::new("captures"),
            "webp",
            "unix-ms",
            "ignored",
            now,
            &|_| false,
        );
        assert_eq!(
            path.file_name().unwrap().to_string_lossy(),
            format!("{}.webp", now.timestamp_millis())
        );
    }
}
