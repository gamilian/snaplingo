# Phase 1: Infrastructure Layer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build complete infrastructure layer providing storage, HTTP, and platform abstraction for the application layer.

**Architecture:** Four-layer architecture foundation. Infrastructure provides ConfigFile (JSON), Keychain (platform-adapted), HttpClient (abstracted), and system utilities (paths, screenshot, hotkey). All platform differences isolated here.

**Tech Stack:** Rust, thiserror, serde_json, keyring, reqwest, core-graphics (macOS), windows (Windows), xcb (Linux)

**Duration:** 2-3 days

---

## File Structure

### New Files to Create

**Error handling:**
- `src-tauri/src/error.rs` - Unified error type using thiserror

**Domain layer:**
- `src-tauri/src/domain/mod.rs` - Domain module exports
- `src-tauri/src/domain/translation.rs` - Translation domain models
- `src-tauri/src/domain/ocr.rs` - OCR domain models
- `src-tauri/src/domain/capture.rs` - Capture domain models
- `src-tauri/src/domain/config.rs` - Config domain models

**Infrastructure - Storage:**
- `src-tauri/src/infrastructure/mod.rs` - Infrastructure module exports
- `src-tauri/src/infrastructure/storage/mod.rs` - Storage module exports
- `src-tauri/src/infrastructure/storage/config_file.rs` - JSON config file operations
- `src-tauri/src/infrastructure/storage/keychain/mod.rs` - Keychain module exports
- `src-tauri/src/infrastructure/storage/keychain/backend.rs` - Keychain trait
- `src-tauri/src/infrastructure/storage/keychain/macos.rs` - macOS Keychain impl
- `src-tauri/src/infrastructure/storage/keychain/windows.rs` - Windows Credential Manager impl
- `src-tauri/src/infrastructure/storage/keychain/linux.rs` - Linux Secret Service impl

**Infrastructure - HTTP:**
- `src-tauri/src/infrastructure/http/mod.rs` - HTTP module exports
- `src-tauri/src/infrastructure/http/client.rs` - HttpClient trait
- `src-tauri/src/infrastructure/http/reqwest_impl.rs` - Reqwest implementation

**Infrastructure - System:**
- `src-tauri/src/infrastructure/system/mod.rs` - System module exports
- `src-tauri/src/infrastructure/system/paths.rs` - Platform-specific paths
- `src-tauri/src/infrastructure/system/screenshot/mod.rs` - Screenshot module exports
- `src-tauri/src/infrastructure/system/screenshot/backend.rs` - Screenshot trait
- `src-tauri/src/infrastructure/system/screenshot/macos.rs` - macOS screenshot impl
- `src-tauri/src/infrastructure/system/screenshot/windows.rs` - Windows screenshot impl
- `src-tauri/src/infrastructure/system/screenshot/linux.rs` - Linux screenshot impl

**Tests:**
- `src-tauri/src/infrastructure/storage/config_file_test.rs` - ConfigFile tests
- `src-tauri/src/infrastructure/http/client_test.rs` - HttpClient tests

### Files to Modify

- `src-tauri/src/lib.rs` - Add new module declarations
- `src-tauri/Cargo.toml` - Add dependencies (thiserror, keyring, image, chrono)

---

## Task 1: Error Handling Foundation

**Files:**
- Create: `src-tauri/src/error.rs`
- Modify: `src-tauri/src/lib.rs:1-10`

- [ ] **Step 1: Create unified error type**

```rust
// src-tauri/src/error.rs

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("Keychain error: {0}")]
    Keychain(#[from] keyring::Error),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Provider not found: {0}")]
    ProviderNotFound(String),
    
    #[error("Provider not configured: {0}")]
    ProviderNotConfigured(String),
    
    #[error("No active provider")]
    NoActiveProvider,
    
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, AppError>;

impl From<String> for AppError {
    fn from(s: String) -> Self {
        AppError::Other(s)
    }
}

impl From<&str> for AppError {
    fn from(s: &str) -> Self {
        AppError::Other(s.to_string())
    }
}
```

- [ ] **Step 2: Add error module to lib.rs**

```rust
// src-tauri/src/lib.rs (prepend to existing content)

mod error;

pub use error::{AppError, Result};

// ... existing module declarations
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check`
Expected: SUCCESS

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/error.rs src-tauri/src/lib.rs
git commit -m "feat(infra): add unified error type with thiserror"
```

---

## Task 2: Domain Layer - Basic Models

**Files:**
- Create: `src-tauri/src/domain/mod.rs`
- Create: `src-tauri/src/domain/translation.rs`
- Create: `src-tauri/src/domain/ocr.rs`
- Create: `src-tauri/src/domain/capture.rs`
- Create: `src-tauri/src/domain/config.rs`
- Modify: `src-tauri/src/lib.rs:2`

- [ ] **Step 1: Create domain module structure**

```rust
// src-tauri/src/domain/mod.rs

pub mod translation;
pub mod ocr;
pub mod capture;
pub mod config;

pub use translation::{TranslationRequest, TranslationResult};
pub use ocr::{OcrRequest, OcrResult};
pub use capture::{CaptureMode, CaptureConfig, CaptureRegion, ImageFormat};
pub use config::AppConfig;
```

- [ ] **Step 2: Create translation domain models**

```rust
// src-tauri/src/domain/translation.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationRequest {
    pub text: String,
    pub source_lang: Option<String>,  // None = auto-detect
    pub target_lang: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationResult {
    pub provider_id: String,
    pub provider_name: String,
    pub text: String,
    pub detected_language: Option<String>,
}

impl TranslationRequest {
    pub fn new(text: String, target_lang: String) -> Self {
        Self {
            text,
            source_lang: None,
            target_lang,
        }
    }
    
    pub fn with_source_lang(mut self, lang: String) -> Self {
        self.source_lang = Some(lang);
        self
    }
}
```

- [ ] **Step 3: Create OCR domain models**

```rust
// src-tauri/src/domain/ocr.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrRequest {
    pub image: Vec<u8>,
    pub language_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResult {
    pub provider_id: String,
    pub text: String,
    pub confidence: Option<f32>,
    pub language: Option<String>,
}
```

- [ ] **Step 4: Create capture domain models**

```rust
// src-tauri/src/domain/capture.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CaptureMode {
    Screenshot,
    Ocr,
    OcrTranslation,
    SelectionTranslation,
    InputTranslation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureConfig {
    pub save_path: String,
    pub format: ImageFormat,
    pub quality: u8,
    pub auto_copy: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Webp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureRegion {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            save_path: "~/Pictures/SnapLingo".to_string(),
            format: ImageFormat::Png,
            quality: 90,
            auto_copy: false,
        }
    }
}
```

- [ ] **Step 5: Create config domain models**

```rust
// src-tauri/src/domain/config.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub active_translation_providers: Vec<String>,
    pub active_ocr_provider: Option<String>,
    pub capture: super::capture::CaptureConfig,
}
```

- [ ] **Step 6: Add domain module to lib.rs**

```rust
// src-tauri/src/lib.rs (after error module)

mod error;
mod domain;

pub use error::{AppError, Result};
```

- [ ] **Step 7: Verify compilation**

Run: `cargo check`
Expected: SUCCESS

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/domain/
git add src-tauri/src/lib.rs
git commit -m "feat(domain): add domain layer models for translation, OCR, and capture"
```

---

## Task 3: Infrastructure - ConfigFile

**Files:**
- Create: `src-tauri/src/infrastructure/mod.rs`
- Create: `src-tauri/src/infrastructure/storage/mod.rs`
- Create: `src-tauri/src/infrastructure/storage/config_file.rs`
- Create: `src-tauri/src/infrastructure/storage/config_file_test.rs`
- Modify: `src-tauri/src/lib.rs:3`
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Add dependencies to Cargo.toml**

Add to `[dev-dependencies]` section:
```toml
tempfile = "3.8"
```

- [ ] **Step 2: Create infrastructure module structure**

```rust
// src-tauri/src/infrastructure/mod.rs

pub mod storage;
pub mod http;
pub mod system;
```

```rust
// src-tauri/src/infrastructure/storage/mod.rs

pub mod config_file;

pub use config_file::ConfigFile;
```

- [ ] **Step 3: Write test for ConfigFile save/load**

```rust
// src-tauri/src/infrastructure/storage/config_file_test.rs

#[cfg(test)]
mod tests {
    use super::super::config_file::ConfigFile;
    use tempfile::tempdir;
    
    #[test]
    fn test_save_and_load() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.json");
        let config_file = ConfigFile::new(config_path);
        
        // Save
        config_file.save("test_key", &"test_value").unwrap();
        
        // Load
        let value: String = config_file.load("test_key").unwrap();
        assert_eq!(value, "test_value");
    }
    
    #[test]
    fn test_load_nonexistent_key() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.json");
        let config_file = ConfigFile::new(config_path);
        
        let result: crate::Result<String> = config_file.load("nonexistent");
        assert!(result.is_err());
    }
    
    #[test]
    fn test_save_multiple_keys() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.json");
        let config_file = ConfigFile::new(config_path);
        
        config_file.save("key1", &"value1").unwrap();
        config_file.save("key2", &42).unwrap();
        
        let value1: String = config_file.load("key1").unwrap();
        let value2: i32 = config_file.load("key2").unwrap();
        
        assert_eq!(value1, "value1");
        assert_eq!(value2, 42);
    }
}
```

- [ ] **Step 4: Run test to verify it fails**

Run: `cargo test config_file_test`
Expected: FAIL with "module not found"

- [ ] **Step 5: Implement ConfigFile**

```rust
// src-tauri/src/infrastructure/storage/config_file.rs

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use crate::Result;

pub struct ConfigFile {
    path: PathBuf,
    lock: Arc<Mutex<()>>,
}

impl ConfigFile {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            lock: Arc::new(Mutex::new(())),
        }
    }
    
    pub fn save<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let _guard = self.lock.lock().unwrap();
        
        let mut config = self.load_all()?;
        config[key] = serde_json::to_value(value)?;
        
        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let json = serde_json::to_string_pretty(&config)?;
        std::fs::write(&self.path, json)?;
        
        Ok(())
    }
    
    pub fn load<T: DeserializeOwned>(&self, key: &str) -> Result<T> {
        let config = self.load_all()?;
        let value = config.get(key)
            .ok_or_else(|| crate::AppError::Config(format!("Key not found: {}", key)))?;
        Ok(serde_json::from_value(value.clone())?)
    }
    
    fn load_all(&self) -> Result<Value> {
        if !self.path.exists() {
            return Ok(serde_json::json!({}));
        }
        let content = std::fs::read_to_string(&self.path)?;
        Ok(serde_json::from_str(&content)?)
    }
}

#[cfg(test)]
#[path = "config_file_test.rs"]
mod config_file_test;
```

- [ ] **Step 6: Add infrastructure module to lib.rs**

```rust
// src-tauri/src/lib.rs (after domain module)

mod error;
mod domain;
mod infrastructure;

pub use error::{AppError, Result};
```

- [ ] **Step 7: Run tests to verify they pass**

Run: `cargo test config_file_test`
Expected: All 3 tests PASS

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/infrastructure/
git add src-tauri/src/lib.rs
git add src-tauri/Cargo.toml
git commit -m "feat(infra): implement ConfigFile with JSON storage and tests"
```

---

## Task 4: Infrastructure - Keychain (Platform Abstraction)

**Files:**
- Create: `src-tauri/src/infrastructure/storage/keychain/mod.rs`
- Create: `src-tauri/src/infrastructure/storage/keychain/backend.rs`
- Create: `src-tauri/src/infrastructure/storage/keychain/macos.rs`
- Create: `src-tauri/src/infrastructure/storage/keychain/windows.rs`
- Create: `src-tauri/src/infrastructure/storage/keychain/linux.rs`
- Modify: `src-tauri/src/infrastructure/storage/mod.rs`

- [ ] **Step 1: Define KeychainBackend trait**

```rust
// src-tauri/src/infrastructure/storage/keychain/backend.rs

use crate::Result;

pub trait KeychainBackend: Send + Sync {
    fn save(&self, service: &str, account: &str, password: &str) -> Result<()>;
    fn load(&self, service: &str, account: &str) -> Result<String>;
    fn delete(&self, service: &str, account: &str) -> Result<()>;
}
```

- [ ] **Step 2: Implement macOS Keychain**

```rust
// src-tauri/src/infrastructure/storage/keychain/macos.rs

use super::backend::KeychainBackend;
use crate::Result;
use keyring::Entry;

pub struct MacOSKeychain;

impl MacOSKeychain {
    pub fn new() -> Self {
        Self
    }
}

impl KeychainBackend for MacOSKeychain {
    fn save(&self, service: &str, account: &str, password: &str) -> Result<()> {
        let entry = Entry::new(service, account)?;
        entry.set_password(password)?;
        Ok(())
    }
    
    fn load(&self, service: &str, account: &str) -> Result<String> {
        let entry = Entry::new(service, account)?;
        Ok(entry.get_password()?)
    }
    
    fn delete(&self, service: &str, account: &str) -> Result<()> {
        let entry = Entry::new(service, account)?;
        entry.delete_password()?;
        Ok(())
    }
}
```

- [ ] **Step 3: Implement Windows Keychain**

```rust
// src-tauri/src/infrastructure/storage/keychain/windows.rs

use super::backend::KeychainBackend;
use crate::Result;
use keyring::Entry;

pub struct WindowsKeychain;

impl WindowsKeychain {
    pub fn new() -> Self {
        Self
    }
}

impl KeychainBackend for WindowsKeychain {
    fn save(&self, service: &str, account: &str, password: &str) -> Result<()> {
        let entry = Entry::new(service, account)?;
        entry.set_password(password)?;
        Ok(())
    }
    
    fn load(&self, service: &str, account: &str) -> Result<String> {
        let entry = Entry::new(service, account)?;
        Ok(entry.get_password()?)
    }
    
    fn delete(&self, service: &str, account: &str) -> Result<()> {
        let entry = Entry::new(service, account)?;
        entry.delete_password()?;
        Ok(())
    }
}
```

- [ ] **Step 4: Implement Linux Keychain**

```rust
// src-tauri/src/infrastructure/storage/keychain/linux.rs

use super::backend::KeychainBackend;
use crate::Result;
use keyring::Entry;

pub struct LinuxKeychain;

impl LinuxKeychain {
    pub fn new() -> Self {
        Self
    }
}

impl KeychainBackend for LinuxKeychain {
    fn save(&self, service: &str, account: &str, password: &str) -> Result<()> {
        let entry = Entry::new(service, account)?;
        entry.set_password(password)?;
        Ok(())
    }
    
    fn load(&self, service: &str, account: &str) -> Result<String> {
        let entry = Entry::new(service, account)?;
        Ok(entry.get_password()?)
    }
    
    fn delete(&self, service: &str, account: &str) -> Result<()> {
        let entry = Entry::new(service, account)?;
        entry.delete_password()?;
        Ok(())
    }
}
```

- [ ] **Step 5: Create platform-adaptive Keychain**

```rust
// src-tauri/src/infrastructure/storage/keychain/mod.rs

mod backend;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "linux")]
mod linux;

pub use backend::KeychainBackend;

#[cfg(target_os = "macos")]
pub use macos::MacOSKeychain as PlatformKeychain;

#[cfg(target_os = "windows")]
pub use windows::WindowsKeychain as PlatformKeychain;

#[cfg(target_os = "linux")]
pub use linux::LinuxKeychain as PlatformKeychain;

use crate::Result;

pub struct Keychain {
    backend: PlatformKeychain,
}

impl Keychain {
    pub fn new() -> Self {
        Self {
            backend: PlatformKeychain::new(),
        }
    }
    
    pub fn save_provider_credential(&self, provider_id: &str, api_key: &str) -> Result<()> {
        self.backend.save("snaplingo", provider_id, api_key)
    }
    
    pub fn load_provider_credential(&self, provider_id: &str) -> Result<String> {
        self.backend.load("snaplingo", provider_id)
    }
    
    pub fn delete_provider_credential(&self, provider_id: &str) -> Result<()> {
        self.backend.delete("snaplingo", provider_id)
    }
}
```

- [ ] **Step 6: Update storage module exports**

```rust
// src-tauri/src/infrastructure/storage/mod.rs

pub mod config_file;
pub mod keychain;

pub use config_file::ConfigFile;
pub use keychain::Keychain;
```

- [ ] **Step 7: Verify compilation**

Run: `cargo check`
Expected: SUCCESS

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/infrastructure/storage/keychain/
git add src-tauri/src/infrastructure/storage/mod.rs
git commit -m "feat(infra): implement Keychain with platform abstraction (macOS/Windows/Linux)"
```

---

## Task 5: Infrastructure - HttpClient Abstraction

**Files:**
- Create: `src-tauri/src/infrastructure/http/mod.rs`
- Create: `src-tauri/src/infrastructure/http/client.rs`
- Create: `src-tauri/src/infrastructure/http/reqwest_impl.rs`
- Create: `src-tauri/src/infrastructure/http/client_test.rs`

- [ ] **Step 1: Define HttpClient trait**

```rust
// src-tauri/src/infrastructure/http/client.rs

use crate::Result;
use async_trait::async_trait;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub body: String,
}

#[async_trait]
pub trait HttpClient: Send + Sync {
    async fn post(&self, url: &str, json: Value) -> Result<Response>;
    async fn get(&self, url: &str) -> Result<Response>;
}
```

- [ ] **Step 2: Write test for HttpClient (using mockito)**

```rust
// src-tauri/src/infrastructure/http/client_test.rs

#[cfg(test)]
mod tests {
    use super::super::reqwest_impl::ReqwestHttpClient;
    use super::super::client::HttpClient;
    use mockito::{mock, server_url};
    
    #[tokio::test]
    async fn test_post_request() {
        let _m = mock("POST", "/translate")
            .with_status(200)
            .with_body(r#"{"result": "success"}"#)
            .create();
        
        let client = ReqwestHttpClient::new();
        let response = client.post(
            &format!("{}/translate", server_url()),
            serde_json::json!({"text": "hello"})
        ).await.unwrap();
        
        assert_eq!(response.status, 200);
        assert!(response.body.contains("success"));
    }
    
    #[tokio::test]
    async fn test_get_request() {
        let _m = mock("GET", "/status")
            .with_status(200)
            .with_body(r#"{"status": "ok"}"#)
            .create();
        
        let client = ReqwestHttpClient::new();
        let response = client.get(
            &format!("{}/status", server_url())
        ).await.unwrap();
        
        assert_eq!(response.status, 200);
        assert!(response.body.contains("ok"));
    }
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test http::client_test`
Expected: FAIL with "module not found"

- [ ] **Step 4: Implement ReqwestHttpClient**

```rust
// src-tauri/src/infrastructure/http/reqwest_impl.rs

use super::client::{HttpClient, Response};
use crate::Result;
use async_trait::async_trait;
use serde_json::Value;

pub struct ReqwestHttpClient {
    client: reqwest::Client,
}

impl ReqwestHttpClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl HttpClient for ReqwestHttpClient {
    async fn post(&self, url: &str, json: Value) -> Result<Response> {
        let resp = self.client
            .post(url)
            .json(&json)
            .send()
            .await?;
        
        Ok(Response {
            status: resp.status().as_u16(),
            body: resp.text().await?,
        })
    }
    
    async fn get(&self, url: &str) -> Result<Response> {
        let resp = self.client
            .get(url)
            .send()
            .await?;
        
        Ok(Response {
            status: resp.status().as_u16(),
            body: resp.text().await?,
        })
    }
}

#[cfg(test)]
#[path = "client_test.rs"]
mod client_test;
```

- [ ] **Step 5: Create HTTP module exports**

```rust
// src-tauri/src/infrastructure/http/mod.rs

pub mod client;
pub mod reqwest_impl;

pub use client::{HttpClient, Response};
pub use reqwest_impl::ReqwestHttpClient;
```

- [ ] **Step 6: Update infrastructure module exports**

```rust
// src-tauri/src/infrastructure/mod.rs

pub mod storage;
pub mod http;
pub mod system;

pub use http::{HttpClient, ReqwestHttpClient};
pub use storage::{ConfigFile, Keychain};
```

- [ ] **Step 7: Run tests to verify they pass**

Run: `cargo test http::client_test`
Expected: All 2 tests PASS

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/infrastructure/http/
git add src-tauri/src/infrastructure/mod.rs
git commit -m "feat(infra): implement HttpClient abstraction with Reqwest and tests"
```

---

## Task 6: Infrastructure - System Utilities (Paths)

**Files:**
- Create: `src-tauri/src/infrastructure/system/mod.rs`
- Create: `src-tauri/src/infrastructure/system/paths.rs`

- [ ] **Step 1: Implement platform-specific path utilities**

```rust
// src-tauri/src/infrastructure/system/paths.rs

use std::path::PathBuf;
use crate::Result;

pub fn get_config_dir() -> Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| crate::AppError::Config("Cannot find home directory".to_string()))?;
    
    #[cfg(target_os = "macos")]
    let config_dir = home.join("Library").join("Application Support").join("snaplingo");
    
    #[cfg(target_os = "windows")]
    let config_dir = home.join("AppData").join("Roaming").join("snaplingo");
    
    #[cfg(target_os = "linux")]
    let config_dir = home.join(".config").join("snaplingo");
    
    std::fs::create_dir_all(&config_dir)?;
    
    Ok(config_dir)
}

pub fn get_config_path() -> Result<PathBuf> {
    Ok(get_config_dir()?.join("config.json"))
}

pub fn get_history_db_path() -> Result<PathBuf> {
    Ok(get_config_dir()?.join("history.db"))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_get_config_path() {
        let path = get_config_path().unwrap();
        assert!(path.to_string_lossy().contains("snaplingo"));
        assert!(path.to_string_lossy().ends_with("config.json"));
    }
    
    #[test]
    fn test_get_history_db_path() {
        let path = get_history_db_path().unwrap();
        assert!(path.to_string_lossy().contains("snaplingo"));
        assert!(path.to_string_lossy().ends_with("history.db"));
    }
}
```

- [ ] **Step 2: Create system module exports**

```rust
// src-tauri/src/infrastructure/system/mod.rs

pub mod paths;

pub use paths::{get_config_dir, get_config_path, get_history_db_path};
```

- [ ] **Step 3: Update infrastructure module exports**

```rust
// src-tauri/src/infrastructure/mod.rs

pub mod storage;
pub mod http;
pub mod system;

pub use http::{HttpClient, ReqwestHttpClient};
pub use storage::{ConfigFile, Keychain};
pub use system::{get_config_path, get_history_db_path};
```

- [ ] **Step 4: Run tests**

Run: `cargo test system::paths`
Expected: All 2 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/infrastructure/system/
git add src-tauri/src/infrastructure/mod.rs
git commit -m "feat(infra): add platform-specific path utilities"
```

---

## Task 7: Infrastructure - Screenshot Backend (Platform Abstraction)

**Files:**
- Create: `src-tauri/src/infrastructure/system/screenshot/mod.rs`
- Create: `src-tauri/src/infrastructure/system/screenshot/backend.rs`
- Create: `src-tauri/src/infrastructure/system/screenshot/macos.rs`
- Create: `src-tauri/src/infrastructure/system/screenshot/windows.rs`
- Create: `src-tauri/src/infrastructure/system/screenshot/linux.rs`
- Modify: `src-tauri/src/infrastructure/system/mod.rs`
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Add platform-specific dependencies**

Add to `src-tauri/Cargo.toml` after existing dependencies:

```toml
# Image processing
image = "0.25"

[target.'cfg(target_os = "macos")'.dependencies]
core-graphics = "0.23"
```

Existing Windows and Linux dependencies are already present.

- [ ] **Step 2: Define ScreenshotBackend trait**

```rust
// src-tauri/src/infrastructure/system/screenshot/backend.rs

use crate::Result;

pub trait ScreenshotBackend: Send + Sync {
    fn capture_full_screen(&self) -> Result<Vec<u8>>;
    fn capture_region(&self, x: i32, y: i32, width: u32, height: u32) -> Result<Vec<u8>>;
}
```

- [ ] **Step 3: Implement macOS screenshot backend**

```rust
// src-tauri/src/infrastructure/system/screenshot/macos.rs

use super::backend::ScreenshotBackend;
use crate::Result;

pub struct MacOSScreenshot;

impl MacOSScreenshot {
    pub fn new() -> Self {
        Self
    }
}

impl ScreenshotBackend for MacOSScreenshot {
    fn capture_full_screen(&self) -> Result<Vec<u8>> {
        // Placeholder implementation - will be refined with core-graphics
        // For now, return empty vec to allow compilation
        Ok(vec![])
    }
    
    fn capture_region(&self, _x: i32, _y: i32, _width: u32, _height: u32) -> Result<Vec<u8>> {
        // Placeholder implementation
        Ok(vec![])
    }
}
```

- [ ] **Step 4: Implement Windows screenshot backend**

```rust
// src-tauri/src/infrastructure/system/screenshot/windows.rs

use super::backend::ScreenshotBackend;
use crate::Result;

pub struct WindowsScreenshot;

impl WindowsScreenshot {
    pub fn new() -> Self {
        Self
    }
}

impl ScreenshotBackend for WindowsScreenshot {
    fn capture_full_screen(&self) -> Result<Vec<u8>> {
        // Placeholder implementation
        Ok(vec![])
    }
    
    fn capture_region(&self, _x: i32, _y: i32, _width: u32, _height: u32) -> Result<Vec<u8>> {
        // Placeholder implementation
        Ok(vec![])
    }
}
```

- [ ] **Step 5: Implement Linux screenshot backend**

```rust
// src-tauri/src/infrastructure/system/screenshot/linux.rs

use super::backend::ScreenshotBackend;
use crate::Result;

pub struct LinuxScreenshot;

impl LinuxScreenshot {
    pub fn new() -> Self {
        Self
    }
}

impl ScreenshotBackend for LinuxScreenshot {
    fn capture_full_screen(&self) -> Result<Vec<u8>> {
        // Placeholder implementation
        Ok(vec![])
    }
    
    fn capture_region(&self, _x: i32, _y: i32, _width: u32, _height: u32) -> Result<Vec<u8>> {
        // Placeholder implementation
        Ok(vec![])
    }
}
```

- [ ] **Step 6: Create platform-adaptive Screenshot**

```rust
// src-tauri/src/infrastructure/system/screenshot/mod.rs

mod backend;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "linux")]
mod linux;

pub use backend::ScreenshotBackend;

#[cfg(target_os = "macos")]
pub use macos::MacOSScreenshot as PlatformScreenshot;

#[cfg(target_os = "windows")]
pub use windows::WindowsScreenshot as PlatformScreenshot;

#[cfg(target_os = "linux")]
pub use linux::LinuxScreenshot as PlatformScreenshot;
```

- [ ] **Step 7: Update system module exports**

```rust
// src-tauri/src/infrastructure/system/mod.rs

pub mod paths;
pub mod screenshot;

pub use paths::{get_config_dir, get_config_path, get_history_db_path};
pub use screenshot::{ScreenshotBackend, PlatformScreenshot};
```

- [ ] **Step 8: Verify compilation**

Run: `cargo check`
Expected: SUCCESS

- [ ] **Step 9: Commit**

```bash
git add src-tauri/src/infrastructure/system/screenshot/
git add src-tauri/src/infrastructure/system/mod.rs
git add src-tauri/Cargo.toml
git commit -m "feat(infra): add Screenshot backend with platform abstraction (placeholder implementations)"
```

---

## Task 8: Update lib.rs Module Declarations

**Files:**
- Modify: `src-tauri/src/lib.rs:1-20`

- [ ] **Step 1: Update lib.rs with complete module structure**

```rust
// src-tauri/src/lib.rs

mod error;
mod domain;
mod infrastructure;

pub use error::{AppError, Result};
pub use domain::*;
pub use infrastructure::*;

// Existing modules (to be refactored in later phases)
mod commands;
mod config;
mod language;
mod ocr;
mod translate;
mod capture;
mod history;
mod utils;
mod hotkeys;

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::path::PathBuf;
use config::Config;
use translate::{GoogleTranslateProvider, TranslationProvider};
use language::LanguageDetector;
use hotkeys::HotkeyManager;
use tauri::Manager;

pub struct AppState {
    pub config: Arc<Mutex<Config>>,
    pub config_path: PathBuf,
    translation_providers: Arc<Mutex<HashMap<String, Arc<dyn TranslationProvider>>>>,
    pub language_detector: LanguageDetector,
    pub hotkey_manager: HotkeyManager,
}

impl AppState {
    pub fn new(config_path: PathBuf, app: tauri::AppHandle) -> Self {
        let config = Config::load_or_default(&config_path).unwrap_or_default();
        let mut providers: HashMap<String, Arc<dyn TranslationProvider>> = HashMap::new();

        providers.insert(
            "google-translate".to_string(),
            Arc::new(GoogleTranslateProvider::default()),
        );

        Self {
            config: Arc::new(Mutex::new(config)),
            config_path,
            translation_providers: Arc::new(Mutex::new(providers)),
            language_detector: LanguageDetector::new(),
            hotkey_manager: HotkeyManager::new(app),
        }
    }

    pub fn get_translation_provider(&self, id: &str) -> Option<Arc<dyn TranslationProvider>> {
        self.translation_providers.lock().unwrap().get(id).cloned()
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  let config_dir = dirs::home_dir()
      .unwrap()
      .join(".snaplingo");
  std::fs::create_dir_all(&config_dir).unwrap();
  let config_path = config_dir.join("config.json");

  tauri::Builder::default()
    .plugin(tauri_plugin_shell::init())
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }

      let app_state = AppState::new(config_path, app.handle().clone());
      app.manage(app_state);

      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      commands::translate_text,
      commands::detect_language,
      commands::get_config,
      commands::update_config,
      commands::open_result_window,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: SUCCESS (with possible warnings about unused exports)

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "refactor: update lib.rs module declarations for new infrastructure layer"
```

---

## Task 9: Integration Test - Infrastructure Layer

**Files:**
- Create: `src-tauri/tests/infrastructure_integration_test.rs`
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Enable integration tests in Cargo.toml**

Verify `[[test]]` section exists or add:

```toml
[[test]]
name = "infrastructure_integration_test"
path = "tests/infrastructure_integration_test.rs"
```

- [ ] **Step 2: Write integration test**

```rust
// src-tauri/tests/infrastructure_integration_test.rs

use snaplingo_lib::{ConfigFile, Keychain, ReqwestHttpClient, HttpClient};
use snaplingo_lib::infrastructure::system::{get_config_path, get_history_db_path};
use tempfile::tempdir;

#[test]
fn test_config_file_integration() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.json");
    let config_file = ConfigFile::new(config_path);
    
    // Save multiple values
    config_file.save("active_providers", &vec!["google", "deepl"]).unwrap();
    config_file.save("ocr_provider", &"tesseract").unwrap();
    
    // Load and verify
    let providers: Vec<String> = config_file.load("active_providers").unwrap();
    assert_eq!(providers, vec!["google", "deepl"]);
    
    let ocr: String = config_file.load("ocr_provider").unwrap();
    assert_eq!(ocr, "tesseract");
}

#[tokio::test]
async fn test_http_client_integration() {
    let client = ReqwestHttpClient::new();
    
    // Test against a real API (httpbin.org for testing)
    let response = client.get("https://httpbin.org/status/200").await.unwrap();
    assert_eq!(response.status, 200);
}

#[test]
fn test_paths_integration() {
    let config_path = get_config_path().unwrap();
    let history_path = get_history_db_path().unwrap();
    
    // Verify paths contain expected components
    assert!(config_path.to_string_lossy().contains("snaplingo"));
    assert!(history_path.to_string_lossy().contains("snaplingo"));
    
    // Verify file names
    assert!(config_path.to_string_lossy().ends_with("config.json"));
    assert!(history_path.to_string_lossy().ends_with("history.db"));
}
```

- [ ] **Step 3: Run integration tests**

Run: `cargo test --test infrastructure_integration_test`
Expected: All 3 tests PASS

- [ ] **Step 4: Commit**

```bash
git add src-tauri/tests/infrastructure_integration_test.rs
git add src-tauri/Cargo.toml
git commit -m "test(infra): add integration tests for infrastructure layer"
```

---

## Task 10: Documentation and Summary

**Files:**
- Create: `src-tauri/src/infrastructure/README.md`

- [ ] **Step 1: Document infrastructure layer**

```markdown
# Infrastructure Layer

Platform-independent abstractions for storage, HTTP, and system operations.

## Components

### Storage
- **ConfigFile**: JSON configuration file operations with thread-safe access
- **Keychain**: Secure credential storage (platform-adapted: macOS Keychain, Windows Credential Manager, Linux Secret Service)

### HTTP
- **HttpClient**: Abstract HTTP client interface
- **ReqwestHttpClient**: Reqwest-based implementation

### System
- **Paths**: Platform-specific configuration and data paths
- **Screenshot**: Screenshot capture (placeholder - to be implemented in Phase 4)

## Platform Adaptation

Platform-specific code is isolated using:
- Trait abstractions (KeychainBackend, ScreenshotBackend)
- Conditional compilation (#[cfg(target_os = "...")])
- Type aliases (PlatformKeychain, PlatformScreenshot)

Application layer code remains platform-agnostic.

## Testing

- Unit tests: Individual component tests
- Integration tests: Cross-component tests in `tests/` directory
- Platform tests: Run on macOS/Windows/Linux to verify platform-specific implementations

## Usage Examples

### ConfigFile
```rust
let config = ConfigFile::new(config_path);
config.save("key", &value)?;
let loaded: ValueType = config.load("key")?;
```

### Keychain
```rust
let keychain = Keychain::new();
keychain.save_provider_credential("deepl", "api_key_value")?;
let api_key = keychain.load_provider_credential("deepl")?;
```

### HttpClient
```rust
let client = ReqwestHttpClient::new();
let response = client.post(url, json_body).await?;
```
```

- [ ] **Step 2: Final compilation check**

Run: `cargo build`
Expected: SUCCESS

- [ ] **Step 3: Run all tests**

Run: `cargo test`
Expected: All tests PASS

- [ ] **Step 4: Commit documentation**

```bash
git add src-tauri/src/infrastructure/README.md
git commit -m "docs(infra): add infrastructure layer documentation"
```

---

## Phase 1 Completion Checklist

- [ ] All infrastructure modules implemented
- [ ] Error handling with thiserror
- [ ] Domain layer models defined
- [ ] ConfigFile with JSON storage
- [ ] Keychain with platform abstraction (macOS/Windows/Linux)
- [ ] HttpClient abstraction with Reqwest
- [ ] System paths (platform-specific)
- [ ] Screenshot backend (placeholder)
- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Documentation complete
- [ ] Code compiles on current platform

**Next Phase:** Phase 2 - Translation Provider vertical slice (migrate Google Translate, add DeepL and Baidu)

**Estimated Time:** 2-3 days
