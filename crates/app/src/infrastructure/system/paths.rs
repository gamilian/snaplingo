use crate::error::{AppError, Result};
use std::path::PathBuf;

/// Returns the platform-specific application data directory for SnapLingo.
/// Creates the directory if it doesn't exist.
///
/// Platform paths:
/// - macOS: ~/Library/Application Support/snaplingo/
/// - Windows: ~/AppData/Roaming/snaplingo/
/// - Linux: ~/.config/snaplingo/
pub fn get_app_data_dir() -> Result<PathBuf> {
    let base_dir = if cfg!(target_os = "macos") {
        dirs::data_local_dir()
            .ok_or_else(|| AppError::System("Failed to get local data directory".to_string()))?
    } else if cfg!(target_os = "windows") {
        dirs::data_dir()
            .ok_or_else(|| AppError::System("Failed to get app data directory".to_string()))?
    } else {
        // Linux
        dirs::config_dir()
            .ok_or_else(|| AppError::System("Failed to get config directory".to_string()))?
    };

    let app_data_dir = base_dir.join("snaplingo");

    // Create directory if it doesn't exist
    if !app_data_dir.exists() {
        std::fs::create_dir_all(&app_data_dir)
            .map_err(|e| AppError::System(format!("Failed to create config directory: {}", e)))?;
    }

    Ok(app_data_dir)
}

/// Returns the path to SnapLingo's single persistent database.
pub fn get_database_path() -> Result<PathBuf> {
    Ok(get_app_data_dir()?.join("snaplingo.db"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_database_path() {
        let path = get_database_path().expect("Failed to get database path");

        // Verify path ends with snaplingo/snaplingo.db
        assert!(path.to_string_lossy().contains("snaplingo"));
        assert!(path.file_name().unwrap() == "snaplingo.db");

        // Verify parent directory was created
        let parent = path.parent().unwrap();
        assert!(
            parent.exists(),
            "Application data directory should be created"
        );
    }
}
