use std::process::Command;
use std::sync::Arc;

use async_trait::async_trait;

use crate::application::capture::CaptureOutput;
use crate::application::screenshot_favorites::{
    ScreenshotFavoriteClipboard, ScreenshotFavoriteHost,
};
use crate::{AppError, Result};

pub struct CaptureOutputScreenshotClipboard {
    output: Arc<CaptureOutput>,
}

impl CaptureOutputScreenshotClipboard {
    pub fn new(output: Arc<CaptureOutput>) -> Self {
        Self { output }
    }
}

#[async_trait]
impl ScreenshotFavoriteClipboard for CaptureOutputScreenshotClipboard {
    async fn copy_png(&self, png_data: &[u8]) -> Result<()> {
        self.output.copy_png(png_data).await
    }
}

pub struct SystemScreenshotFavoriteHost;

impl ScreenshotFavoriteHost for SystemScreenshotFavoriteHost {
    fn reveal(&self, absolute_path: &str) -> Result<()> {
        #[cfg(target_os = "macos")]
        let status = Command::new("open").args(["-R", absolute_path]).status()?;
        #[cfg(target_os = "windows")]
        let status = Command::new("explorer")
            .arg(format!("/select,{absolute_path}"))
            .status()?;
        #[cfg(target_os = "linux")]
        let status = Command::new("xdg-open")
            .arg(
                std::path::Path::new(absolute_path)
                    .parent()
                    .ok_or_else(|| AppError::Other("Screenshot has no parent directory".into()))?,
            )
            .status()?;

        if status.success() {
            Ok(())
        } else {
            Err(AppError::System(
                "Failed to reveal screenshot favorite".into(),
            ))
        }
    }
}
