use mockito::Server;
use snaplingo_lib::http::{HttpClient, ReqwestHttpClient};
use snaplingo_lib::storage::{Database, SqliteConfigStore};
use snaplingo_lib::system::get_database_path;
use snaplingo_lib::SettingsSnapshot;
use std::collections::HashMap;
use std::net::{Ipv4Addr, TcpListener};
use std::sync::Arc;
use tempfile::TempDir;

#[test]
fn test_sqlite_settings_integration() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let database_path = temp_dir.path().join("snaplingo.db");
    let database = Arc::new(Database::open(&database_path).unwrap());
    let config = SqliteConfigStore::new(database);
    let mut snapshot = SettingsSnapshot::default();
    snapshot.general.language = "es".to_string();

    config.save_settings(&snapshot).unwrap();

    let reloaded_database = Arc::new(Database::open(&database_path).unwrap());
    let reloaded_config = SqliteConfigStore::new(reloaded_database);
    assert_eq!(reloaded_config.load_settings().unwrap(), snapshot);

    snapshot.general.language = "fr".to_string();
    reloaded_config.save_settings(&snapshot).unwrap();
    assert_eq!(reloaded_config.load_settings().unwrap(), snapshot);
}

#[tokio::test]
async fn test_http_client_integration() {
    if skip_when_local_tcp_listener_unavailable("test_http_client_integration") {
        return;
    }

    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body("<html>Rust</html>")
        .create_async()
        .await;

    let reqwest_client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .expect("Failed to build no-proxy HTTP client");
    let client = ReqwestHttpClient::with_client(reqwest_client);

    let headers: HashMap<String, String> = HashMap::new();
    let response = client
        .get(&server.url(), headers)
        .await
        .expect("Failed to make HTTP request");

    mock.assert_async().await;

    // Verify response
    assert!(
        response.status >= 200 && response.status < 300,
        "Expected 2xx status code, got {}",
        response.status
    );
    assert!(
        !response.body.is_empty(),
        "Expected non-empty response body"
    );

    // Verify it's HTML content
    assert!(
        response.body.contains("Rust") || response.body.contains("rust"),
        "Expected response to contain 'Rust'"
    );
}

fn skip_when_local_tcp_listener_unavailable(test_name: &str) -> bool {
    match TcpListener::bind((Ipv4Addr::LOCALHOST, 0)) {
        Ok(listener) => {
            drop(listener);
            false
        }
        Err(err) => {
            eprintln!("skipping {test_name}: local TCP listener unavailable for mockito ({err})");
            true
        }
    }
}

#[test]
fn test_paths_integration() {
    let database_path = get_database_path().expect("Failed to get database path");
    assert!(
        database_path.to_str().is_some(),
        "Database path should be valid UTF-8"
    );
    assert!(
        database_path.to_str().unwrap().contains("snaplingo"),
        "Database path should contain 'snaplingo'"
    );
    assert!(
        database_path.to_str().unwrap().ends_with("snaplingo.db"),
        "Database path should end with 'snaplingo.db'"
    );
}
