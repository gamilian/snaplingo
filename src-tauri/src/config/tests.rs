#[cfg(test)]
mod tests {
    use super::super::*;
    use tempfile::TempDir;

    #[test]
    fn test_load_default_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let config = Config::load_or_default(&config_path).unwrap();

        assert_eq!(config.version, "1.0.0");
        assert_eq!(config.general.language, "en");
        assert!(config.general.start_on_boot);
    }

    #[test]
    fn test_save_and_load_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let mut config = Config::default();
        config.general.language = "zh-CN".to_string();

        config.save(&config_path).unwrap();
        let loaded = Config::load_or_default(&config_path).unwrap();

        assert_eq!(loaded.general.language, "zh-CN");
    }
}
