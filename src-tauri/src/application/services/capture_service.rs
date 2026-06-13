use crate::error::Result;
use crate::infrastructure::system::screenshot::{ScreenshotBackend, ScreenRegion};
use std::path::PathBuf;
use std::sync::Arc;

/// CaptureService coordinates screenshot capture operations.
/// It wraps the platform-specific screenshot backend and provides
/// high-level capture and file management operations.
pub struct CaptureService {
    screenshot_backend: Arc<dyn ScreenshotBackend>,
}

impl CaptureService {
    /// Create a new CaptureService with the given screenshot backend
    pub fn new(screenshot_backend: Arc<dyn ScreenshotBackend>) -> Self {
        Self { screenshot_backend }
    }

    /// Capture the entire screen and return PNG data
    pub async fn capture_full_screen(&self) -> Result<Vec<u8>> {
        self.screenshot_backend.capture_full_screen().await
    }

    /// Capture a specific screen region and return PNG data
    pub async fn capture_region(&self, region: ScreenRegion) -> Result<Vec<u8>> {
        self.screenshot_backend.capture_region(region).await
    }

    /// Save screenshot data to a file
    pub async fn save_screenshot(&self, data: &[u8], path: &PathBuf) -> Result<PathBuf> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Write data to file
        std::fs::write(path, data)?;

        Ok(path.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::CaptureService;
    use crate::error::AppError;
    use crate::infrastructure::system::screenshot::{ScreenshotBackend, ScreenRegion};
    use std::sync::Arc;

    // Mock ScreenshotBackend for testing
    struct MockScreenshotBackend {
        full_screen_data: Vec<u8>,
        region_data: Vec<u8>,
    }

    impl MockScreenshotBackend {
        fn new() -> Self {
            Self {
                full_screen_data: vec![1, 2, 3, 4], // Mock PNG data
                region_data: vec![5, 6, 7, 8],
            }
        }
    }

    #[async_trait::async_trait]
    impl ScreenshotBackend for MockScreenshotBackend {
        async fn capture_full_screen(&self) -> Result<Vec<u8>, AppError> {
            Ok(self.full_screen_data.clone())
        }

        async fn capture_region(&self, _region: ScreenRegion) -> Result<Vec<u8>, AppError> {
            Ok(self.region_data.clone())
        }
    }

    #[tokio::test]
    async fn test_capture_full_screen() {
        // Arrange
        let mock_backend = Arc::new(MockScreenshotBackend::new());
        let service = CaptureService::new(mock_backend.clone());

        // Act
        let result = service.capture_full_screen().await;

        // Assert
        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data, vec![1, 2, 3, 4]);
    }

    #[tokio::test]
    async fn test_capture_region() {
        // Arrange
        let mock_backend = Arc::new(MockScreenshotBackend::new());
        let service = CaptureService::new(mock_backend.clone());

        let region = ScreenRegion {
            x: 100,
            y: 100,
            width: 500,
            height: 300,
        };

        // Act
        let result = service.capture_region(region).await;

        // Assert
        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data, vec![5, 6, 7, 8]);
    }

    #[tokio::test]
    async fn test_save_to_file() {
        // Arrange
        let mock_backend = Arc::new(MockScreenshotBackend::new());
        let service = CaptureService::new(mock_backend);

        let test_data = vec![137, 80, 78, 71]; // PNG header
        let test_dir = std::env::temp_dir();
        let test_path = test_dir.join("test_screenshot.png");

        // Clean up any existing file
        let _ = std::fs::remove_file(&test_path);

        // Act
        let result = service.save_screenshot(&test_data, &test_path).await;

        // Assert
        assert!(result.is_ok());
        let saved_path = result.unwrap();
        assert_eq!(saved_path, test_path);
        assert!(saved_path.exists());

        // Verify file contents
        let contents = std::fs::read(&saved_path).unwrap();
        assert_eq!(contents, test_data);

        // Clean up
        let _ = std::fs::remove_file(&test_path);
    }
}
