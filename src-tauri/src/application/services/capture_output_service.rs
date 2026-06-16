use std::path::{Path, PathBuf};

use crate::error::Result;

pub struct CaptureOutputService;

impl CaptureOutputService {
    pub fn new() -> Self {
        Self
    }

    pub async fn save_png(&self, data: &[u8], path: &Path) -> Result<PathBuf> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(path, data)?;

        Ok(path.to_path_buf())
    }
}

impl Default for CaptureOutputService {
    fn default() -> Self {
        Self::new()
    }
}
