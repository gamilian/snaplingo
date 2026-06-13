use crate::error::{AppError, Result};
use std::path::PathBuf;

/// Returns the platform-specific config directory for SnapLingo.
/// Creates the directory if it doesn't exist.
///
/// Platform paths:
/// - macOS: ~/Library/Application Support/snaplingo/
/// - Windows: ~/AppData/Roaming/snaplingo/
/// - Linux: ~/.config/snaplingo/
pub fn get_config_dir() -> Result<PathBuf> {
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

    let config_dir = base_dir.join("snaplingo");

    // Create directory if it doesn't exist
    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir).map_err(|e| {
            AppError::System(format!("Failed to create config directory: {}", e))
        })?;
    }

    Ok(config_dir)
}

/// Returns the path to the config.json file.
pub fn get_config_path() -> Result<PathBuf> {
    Ok(get_config_dir()?.join("config.json"))
}

/// Returns the path to the history.db file.
pub fn get_history_db_path() -> Result<PathBuf> {
    Ok(get_config_dir()?.join("history.db"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_config_path() {
        let path = get_config_path().expect("Failed to get config path");

        // Verify path ends with snaplingo/config.json
        assert!(path.to_string_lossy().contains("snaplingo"));
        assert!(path.file_name().unwrap() == "config.json");

        // Verify parent directory was created
        let parent = path.parent().unwrap();
        assert!(parent.exists(), "Config directory should be created");
    }

    #[test]
    fn test_get_history_db_path() {
        let path = get_history_db_path().expect("Failed to get history db path");

        // Verify path ends with snaplingo/history.db
        assert!(path.to_string_lossy().contains("snaplingo"));
        assert!(path.file_name().unwrap() == "history.db");

        // Verify parent directory was created
        let parent = path.parent().unwrap();
        assert!(parent.exists(), "Config directory should be created");
    }
}
