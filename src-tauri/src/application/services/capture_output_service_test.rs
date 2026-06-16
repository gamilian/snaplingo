#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::application::services::capture_output_service::CaptureOutputService;

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
}
