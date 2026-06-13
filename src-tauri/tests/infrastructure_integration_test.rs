use snaplingo_lib::storage::ConfigFile;
use snaplingo_lib::http::{HttpClient, ReqwestHttpClient};
use snaplingo_lib::system::{get_config_path, get_history_db_path};
use tempfile::TempDir;
use std::collections::HashMap;

#[test]
fn test_config_file_integration() {
    // Create a temporary directory for this test
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("test_config.json");

    let config = ConfigFile::new(config_path.clone());

    // Test saving multiple keys
    config.save("target_language", &"es".to_string()).expect("Failed to set target_language");
    config.save("source_language", &"auto".to_string()).expect("Failed to set source_language");
    config.save("provider", &"google-translate".to_string()).expect("Failed to set provider");

    // Test loading
    let loaded_config = ConfigFile::new(config_path.clone());
    assert_eq!(
        loaded_config.load::<String>("target_language").expect("Failed to get target_language"),
        "es"
    );
    assert_eq!(
        loaded_config.load::<String>("source_language").expect("Failed to get source_language"),
        "auto"
    );
    assert_eq!(
        loaded_config.load::<String>("provider").expect("Failed to get provider"),
        "google-translate"
    );

    // Test updating a key
    config.save("target_language", &"fr".to_string()).expect("Failed to update target_language");
    let reloaded_config = ConfigFile::new(config_path.clone());
    assert_eq!(
        reloaded_config.load::<String>("target_language").expect("Failed to get updated target_language"),
        "fr"
    );
}

#[tokio::test]
async fn test_http_client_integration() {
    let client = ReqwestHttpClient::new();

    // Test a real HTTP GET request to a reliable endpoint
    let headers: HashMap<String, String> = HashMap::new();
    let response = client
        .get("https://www.rust-lang.org/", headers)
        .await
        .expect("Failed to make HTTP request");

    // Verify response
    assert!(response.status >= 200 && response.status < 300, "Expected 2xx status code, got {}", response.status);
    assert!(!response.body.is_empty(), "Expected non-empty response body");

    // Verify it's HTML content
    assert!(
        response.body.contains("Rust") || response.body.contains("rust"),
        "Expected response to contain 'Rust'"
    );
}

#[test]
fn test_paths_integration() {
    // Test that get_config_path returns a valid path
    let config_path = get_config_path().expect("Failed to get config path");
    assert!(config_path.to_str().is_some(), "Config path should be valid UTF-8");
    assert!(
        config_path.to_str().unwrap().contains("snaplingo"),
        "Config path should contain 'snaplingo'"
    );
    assert!(
        config_path.to_str().unwrap().ends_with("config.json"),
        "Config path should end with 'config.json'"
    );

    // Test that get_history_db_path returns a valid path
    let history_path = get_history_db_path().expect("Failed to get history db path");
    assert!(history_path.to_str().is_some(), "History path should be valid UTF-8");
    assert!(
        history_path.to_str().unwrap().contains("snaplingo"),
        "History path should contain 'snaplingo'"
    );
    assert!(
        history_path.to_str().unwrap().ends_with("history.db"),
        "History path should end with 'history.db'"
    );
}
