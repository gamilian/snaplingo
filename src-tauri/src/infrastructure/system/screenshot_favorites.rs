use std::process::Command;

use crate::application::screenshot_favorites::ScreenshotFavoriteHost;
use crate::{AppError, Result};

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
