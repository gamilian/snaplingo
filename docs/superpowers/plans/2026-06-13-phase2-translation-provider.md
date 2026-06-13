# Phase 2: Translation Provider Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate existing translation functionality to new architecture and add multiple translation providers with concurrent execution.

**Architecture:** Vertical slice for Translation Provider. Includes Trait definition, Registry (multi-select), Service (concurrent calls), and three implementations (Google Translate, DeepL, Baidu). Replaces old flat translate/ module.

**Tech Stack:** Rust, async-trait, tokio (concurrent execution), existing domain/infrastructure from Phase 1

**Duration:** 2-3 days

**Prerequisites:** Phase 1 (Infrastructure Layer) must be completed

---

## File Structure

### New Files to Create

**Application - Provider Common:**
- `src-tauri/src/application/mod.rs` - Application module exports
- `src-tauri/src/application/providers/mod.rs` - Providers module exports
- `src-tauri/src/application/providers/common/mod.rs` - Common provider exports
- `src-tauri/src/application/providers/common/provider.rs` - Base Provider trait

**Application - Translation Provider:**
- `src-tauri/src/application/providers/translation/mod.rs` - Translation module exports
- `src-tauri/src/application/providers/translation/trait_def.rs` - TranslationProvider trait
- `src-tauri/src/application/providers/translation/registry.rs` - TranslationRegistry (multi-select)
- `src-tauri/src/application/providers/translation/service.rs` - TranslationService
- `src-tauri/src/application/providers/translation/impls/mod.rs` - Implementations module
- `src-tauri/src/application/providers/translation/impls/google.rs` - Google Translate (migrated)
- `src-tauri/src/application/providers/translation/impls/deepl.rs` - DeepL
- `src-tauri/src/application/providers/translation/impls/baidu.rs` - Baidu Translation

**Commands:**
- `src-tauri/src/commands/translation_commands.rs` - Translation Tauri commands
- `src-tauri/src/commands/provider_commands.rs` - Provider management commands

**Tests:**
- `src-tauri/src/application/providers/translation/registry_test.rs` - Registry tests
- `src-tauri/src/application/providers/translation/service_test.rs` - Service tests
- `src-tauri/tests/translation_integration_test.rs` - Integration tests

### Files to Modify

- `src-tauri/src/lib.rs` - Update AppState with new Translation components
- `src-tauri/src/commands/mod.rs` - Add new command modules

### Files to Delete (after migration)

- `src-tauri/src/translate/` - Old translation module (delete after verification)

---

## Task 1: Provider Common Base

**Files:**
- Create: `src-tauri/src/application/mod.rs`
- Create: `src-tauri/src/application/providers/mod.rs`
- Create: `src-tauri/src/application/providers/common/mod.rs`
- Create: `src-tauri/src/application/providers/common/provider.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Create base Provider trait**

```rust
// src-tauri/src/application/providers/common/provider.rs

/// Base trait for all providers (OCR, Translation, TTS)
pub trait Provider: Send + Sync {
    /// Unique identifier (e.g., "google-translate", "deepl")
    fn id(&self) -> &str;
    
    /// Display name (e.g., "Google Translate", "DeepL")
    fn name(&self) -> &str;
    
    /// Whether this provider is ready to use
    fn is_configured(&self) -> bool;
    
    /// Whether this provider requires an API key
    fn requires_api_key(&self) -> bool;
}
```

- [ ] **Step 2: Create module structure**

```rust
// src-tauri/src/application/providers/common/mod.rs

mod provider;

pub use provider::Provider;
```

```rust
// src-tauri/src/application/providers/mod.rs

pub mod common;
pub mod translation;

pub use common::Provider;
```

```rust
// src-tauri/src/application/mod.rs

pub mod providers;

pub use providers::Provider;
```

- [ ] **Step 3: Add application module to lib.rs**

```rust
// src-tauri/src/lib.rs (add after infrastructure)

mod error;
mod domain;
mod infrastructure;
mod application;

pub use error::{AppError, Result};
pub use domain::*;
pub use infrastructure::*;
pub use application::*;
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check`
Expected: SUCCESS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/application/
git add src-tauri/src/lib.rs
git commit -m "feat(app): add Provider base trait for all provider types"
```

---

## Task 2: TranslationProvider Trait

**Files:**
- Create: `src-tauri/src/application/providers/translation/mod.rs`
- Create: `src-tauri/src/application/providers/translation/trait_def.rs`

- [ ] **Step 1: Define TranslationProvider trait**

```rust
// src-tauri/src/application/providers/translation/trait_def.rs

use crate::domain::translation::{TranslationRequest, TranslationResult};
use crate::application::providers::common::Provider;
use crate::Result;
use async_trait::async_trait;

#[async_trait]
pub trait TranslationProvider: Provider {
    /// Translate text from source language to target language
    async fn translate(&self, request: &TranslationRequest) -> Result<TranslationResult>;
}
```

- [ ] **Step 2: Create translation module exports**

```rust
// src-tauri/src/application/providers/translation/mod.rs

mod trait_def;
pub mod registry;
pub mod service;
pub mod impls;

pub use trait_def::TranslationProvider;
pub use registry::TranslationRegistry;
pub use service::TranslationService;
```

- [ ] **Step 3: Update providers module**

```rust
// src-tauri/src/application/providers/mod.rs

pub mod common;
pub mod translation;

pub use common::Provider;
pub use translation::{TranslationProvider, TranslationRegistry, TranslationService};
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check`
Expected: SUCCESS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/application/providers/translation/trait_def.rs
git add src-tauri/src/application/providers/translation/mod.rs
git add src-tauri/src/application/providers/mod.rs
git commit -m "feat(app): add TranslationProvider trait definition"
```

---

## Task 3: TranslationRegistry (Multi-Select)

**Files:**
- Create: `src-tauri/src/application/providers/translation/registry.rs`
- Create: `src-tauri/src/application/providers/translation/registry_test.rs`

- [ ] **Step 1: Write test for TranslationRegistry**

```rust
// src-tauri/src/application/providers/translation/registry_test.rs

#[cfg(test)]
mod tests {
    use super::super::registry::TranslationRegistry;
    use super::super::TranslationProvider;
    use crate::application::providers::common::Provider;
    use crate::domain::translation::{TranslationRequest, TranslationResult};
    use crate::Result;
    use async_trait::async_trait;
    use std::sync::Arc;
    
    // Mock provider for testing
    struct MockProvider {
        id: String,
        name: String,
    }
    
    impl Provider for MockProvider {
        fn id(&self) -> &str { &self.id }
        fn name(&self) -> &str { &self.name }
        fn is_configured(&self) -> bool { true }
        fn requires_api_key(&self) -> bool { false }
    }
    
    #[async_trait]
    impl TranslationProvider for MockProvider {
        async fn translate(&self, _request: &TranslationRequest) -> Result<TranslationResult> {
            Ok(TranslationResult {
                provider_id: self.id.clone(),
                provider_name: self.name.clone(),
                text: "translated".to_string(),
                detected_language: None,
            })
        }
    }
    
    #[test]
    fn test_register_provider() {
        let mut registry = TranslationRegistry::new();
        let provider = Arc::new(MockProvider {
            id: "test".to_string(),
            name: "Test Provider".to_string(),
        });
        
        registry.register(provider);
        
        let all = registry.list_all();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id(), "test");
    }
    
    #[test]
    fn test_activate_multiple_providers() {
        let mut registry = TranslationRegistry::new();
        
        registry.register(Arc::new(MockProvider {
            id: "provider1".to_string(),
            name: "Provider 1".to_string(),
        }));
        registry.register(Arc::new(MockProvider {
            id: "provider2".to_string(),
            name: "Provider 2".to_string(),
        }));
        
        registry.activate("provider1").unwrap();
        registry.activate("provider2").unwrap();
        
        let active = registry.get_active();
        assert_eq!(active.len(), 2);
    }
    
    #[test]
    fn test_deactivate_provider() {
        let mut registry = TranslationRegistry::new();
        
        registry.register(Arc::new(MockProvider {
            id: "provider1".to_string(),
            name: "Provider 1".to_string(),
        }));
        
        registry.activate("provider1").unwrap();
        assert_eq!(registry.get_active().len(), 1);
        
        registry.deactivate("provider1");
        assert_eq!(registry.get_active().len(), 0);
    }
    
    #[test]
    fn test_activate_nonexistent_provider() {
        let mut registry = TranslationRegistry::new();
        
        let result = registry.activate("nonexistent");
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test translation::registry_test`
Expected: FAIL with "module not found"

- [ ] **Step 3: Implement TranslationRegistry**

```rust
// src-tauri/src/application/providers/translation/registry.rs

use super::TranslationProvider;
use crate::Result;
use std::collections::HashMap;
use std::sync::Arc;

pub struct TranslationRegistry {
    providers: HashMap<String, Arc<dyn TranslationProvider>>,
    active: Vec<String>,  // Multi-select
}

impl TranslationRegistry {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            active: Vec::new(),
        }
    }
    
    pub fn register(&mut self, provider: Arc<dyn TranslationProvider>) {
        self.providers.insert(provider.id().to_string(), provider);
    }
    
    pub fn activate(&mut self, id: &str) -> Result<()> {
        if !self.providers.contains_key(id) {
            return Err(crate::AppError::ProviderNotFound(id.to_string()));
        }
        if !self.active.contains(&id.to_string()) {
            self.active.push(id.to_string());
        }
        Ok(())
    }
    
    pub fn deactivate(&mut self, id: &str) {
        self.active.retain(|x| x != id);
    }
    
    pub fn get_active(&self) -> Vec<Arc<dyn TranslationProvider>> {
        self.active
            .iter()
            .filter_map(|id| self.providers.get(id).cloned())
            .collect()
    }
    
    pub fn list_all(&self) -> Vec<Arc<dyn TranslationProvider>> {
        self.providers.values().cloned().collect()
    }
    
    pub fn get(&self, id: &str) -> Option<Arc<dyn TranslationProvider>> {
        self.providers.get(id).cloned()
    }
}

#[cfg(test)]
#[path = "registry_test.rs"]
mod registry_test;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test translation::registry_test`
Expected: All 4 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/application/providers/translation/registry.rs
git add src-tauri/src/application/providers/translation/registry_test.rs
git commit -m "feat(app): implement TranslationRegistry with multi-select and tests"
```

---

## Task 4: TranslationService (Concurrent Execution)

**Files:**
- Create: `src-tauri/src/application/providers/translation/service.rs`
- Create: `src-tauri/src/application/providers/translation/service_test.rs`
- Modify: `src-tauri/Cargo.toml` (if needed)

- [ ] **Step 1: Write test for TranslationService**

```rust
// src-tauri/src/application/providers/translation/service_test.rs

#[cfg(test)]
mod tests {
    use super::super::service::TranslationService;
    use super::super::registry::TranslationRegistry;
    use super::super::TranslationProvider;
    use crate::application::providers::common::Provider;
    use crate::domain::translation::{TranslationRequest, TranslationResult};
    use crate::Result;
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};
    
    // Mock provider
    struct MockProvider {
        id: String,
        name: String,
    }
    
    impl Provider for MockProvider {
        fn id(&self) -> &str { &self.id }
        fn name(&self) -> &str { &self.name }
        fn is_configured(&self) -> bool { true }
        fn requires_api_key(&self) -> bool { false }
    }
    
    #[async_trait]
    impl TranslationProvider for MockProvider {
        async fn translate(&self, request: &TranslationRequest) -> Result<TranslationResult> {
            Ok(TranslationResult {
                provider_id: self.id.clone(),
                provider_name: self.name.clone(),
                text: format!("{}_translated", request.text),
                detected_language: None,
            })
        }
    }
    
    #[tokio::test]
    async fn test_translate_with_single_provider() {
        let mut registry = TranslationRegistry::new();
        registry.register(Arc::new(MockProvider {
            id: "provider1".to_string(),
            name: "Provider 1".to_string(),
        }));
        registry.activate("provider1").unwrap();
        
        let service = TranslationService::new(Arc::new(Mutex::new(registry)));
        
        let request = TranslationRequest::new("hello".to_string(), "zh".to_string());
        let results = service.translate(&request).await.unwrap();
        
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].provider_id, "provider1");
        assert_eq!(results[0].text, "hello_translated");
    }
    
    #[tokio::test]
    async fn test_translate_with_multiple_providers() {
        let mut registry = TranslationRegistry::new();
        
        registry.register(Arc::new(MockProvider {
            id: "provider1".to_string(),
            name: "Provider 1".to_string(),
        }));
        registry.register(Arc::new(MockProvider {
            id: "provider2".to_string(),
            name: "Provider 2".to_string(),
        }));
        
        registry.activate("provider1").unwrap();
        registry.activate("provider2").unwrap();
        
        let service = TranslationService::new(Arc::new(Mutex::new(registry)));
        
        let request = TranslationRequest::new("hello".to_string(), "zh".to_string());
        let results = service.translate(&request).await.unwrap();
        
        assert_eq!(results.len(), 2);
        
        // Results should be from both providers
        let ids: Vec<_> = results.iter().map(|r| r.provider_id.as_str()).collect();
        assert!(ids.contains(&"provider1"));
        assert!(ids.contains(&"provider2"));
    }
    
    #[tokio::test]
    async fn test_translate_with_no_active_provider() {
        let registry = TranslationRegistry::new();
        let service = TranslationService::new(Arc::new(Mutex::new(registry)));
        
        let request = TranslationRequest::new("hello".to_string(), "zh".to_string());
        let result = service.translate(&request).await;
        
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test translation::service_test`
Expected: FAIL with "module not found"

- [ ] **Step 3: Implement TranslationService**

```rust
// src-tauri/src/application/providers/translation/service.rs

use super::TranslationRegistry;
use crate::domain::translation::{TranslationRequest, TranslationResult};
use crate::Result;
use std::sync::{Arc, Mutex};

pub struct TranslationService {
    registry: Arc<Mutex<TranslationRegistry>>,
}

impl TranslationService {
    pub fn new(registry: Arc<Mutex<TranslationRegistry>>) -> Self {
        Self { registry }
    }
    
    pub async fn translate(&self, request: &TranslationRequest) -> Result<Vec<TranslationResult>> {
        // 1. Get active providers
        let providers = self.registry.lock().unwrap().get_active();
        
        if providers.is_empty() {
            return Err(crate::AppError::NoActiveProvider);
        }
        
        // 2. Concurrent calls to all active providers
        let mut tasks = Vec::new();
        for provider in providers {
            let request = request.clone();
            tasks.push(tokio::spawn(async move {
                provider.translate(&request).await
            }));
        }
        
        // 3. Collect results
        let mut results = Vec::new();
        for task in tasks {
            if let Ok(Ok(result)) = task.await {
                results.push(result);
            }
        }
        
        Ok(results)
    }
}

#[cfg(test)]
#[path = "service_test.rs"]
mod service_test;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test translation::service_test`
Expected: All 3 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/application/providers/translation/service.rs
git add src-tauri/src/application/providers/translation/service_test.rs
git commit -m "feat(app): implement TranslationService with concurrent execution and tests"
```

---

## Task 5: Google Translate Provider (Migration)

**Files:**
- Create: `src-tauri/src/application/providers/translation/impls/mod.rs`
- Create: `src-tauri/src/application/providers/translation/impls/google.rs`
- Reference: `src-tauri/src/translate/google.rs` (existing implementation to migrate)

- [ ] **Step 1: Read existing Google Translate implementation**

Run: `cat src-tauri/src/translate/google.rs`
Review the existing implementation to understand the API structure.

- [ ] **Step 2: Create implementations module**

```rust
// src-tauri/src/application/providers/translation/impls/mod.rs

pub mod google;

pub use google::GoogleTranslateProvider;
```

- [ ] **Step 3: Implement GoogleTranslateProvider with new architecture**

```rust
// src-tauri/src/application/providers/translation/impls/google.rs

use crate::application::providers::common::Provider;
use crate::application::providers::translation::TranslationProvider;
use crate::domain::translation::{TranslationRequest, TranslationResult};
use crate::infrastructure::http::HttpClient;
use crate::Result;
use async_trait::async_trait;
use std::sync::Arc;

pub struct GoogleTranslateProvider {
    http_client: Arc<dyn HttpClient>,
}

impl GoogleTranslateProvider {
    pub fn new(http_client: Arc<dyn HttpClient>) -> Self {
        Self { http_client }
    }
}

impl Provider for GoogleTranslateProvider {
    fn id(&self) -> &str {
        "google-translate"
    }
    
    fn name(&self) -> &str {
        "Google Translate"
    }
    
    fn is_configured(&self) -> bool {
        true  // Free API, no configuration needed
    }
    
    fn requires_api_key(&self) -> bool {
        false
    }
}

#[async_trait]
impl TranslationProvider for GoogleTranslateProvider {
    async fn translate(&self, request: &TranslationRequest) -> Result<TranslationResult> {
        let source_lang = request.source_lang.as_deref().unwrap_or("auto");
        let url = format!(
            "https://translate.googleapis.com/translate_a/single?client=gtx&sl={}&tl={}&dt=t&q={}",
            source_lang,
            request.target_lang,
            urlencoding::encode(&request.text)
        );
        
        let response = self.http_client.get(&url).await?;
        
        // Parse Google Translate API response
        let json: serde_json::Value = serde_json::from_str(&response.body)?;
        
        let translated_text = json[0][0][0]
            .as_str()
            .ok_or_else(|| crate::AppError::Other("Failed to parse translation result".to_string()))?
            .to_string();
        
        let detected_lang = json[2]
            .as_str()
            .map(|s| s.to_string());
        
        Ok(TranslationResult {
            provider_id: self.id().to_string(),
            provider_name: self.name().to_string(),
            text: translated_text,
            detected_language: detected_lang,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::http::ReqwestHttpClient;
    
    #[tokio::test]
    #[ignore] // Requires network
    async fn test_google_translate_real_api() {
        let http_client = Arc::new(ReqwestHttpClient::new());
        let provider = GoogleTranslateProvider::new(http_client);
        
        let request = TranslationRequest::new("hello".to_string(), "zh".to_string());
        let result = provider.translate(&request).await.unwrap();
        
        assert_eq!(result.provider_id, "google-translate");
        assert!(!result.text.is_empty());
    }
}
```

- [ ] **Step 4: Update translation module exports**

```rust
// src-tauri/src/application/providers/translation/mod.rs

mod trait_def;
pub mod registry;
pub mod service;
pub mod impls;

pub use trait_def::TranslationProvider;
pub use registry::TranslationRegistry;
pub use service::TranslationService;
pub use impls::GoogleTranslateProvider;
```

- [ ] **Step 5: Verify compilation**

Run: `cargo check`
Expected: SUCCESS

- [ ] **Step 6: Test with real API (optional)**

Run: `cargo test google::tests::test_google_translate_real_api --ignored`
Expected: PASS (requires network)

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/application/providers/translation/impls/
git add src-tauri/src/application/providers/translation/mod.rs
git commit -m "feat(app): migrate Google Translate to new architecture with HttpClient injection"
```

---

## Task 6: DeepL Provider

**Files:**
- Create: `src-tauri/src/application/providers/translation/impls/deepl.rs`
- Modify: `src-tauri/src/application/providers/translation/impls/mod.rs`

- [ ] **Step 1: Implement DeepL Provider**

```rust
// src-tauri/src/application/providers/translation/impls/deepl.rs

use crate::application::providers::common::Provider;
use crate::application::providers::translation::TranslationProvider;
use crate::domain::translation::{TranslationRequest, TranslationResult};
use crate::infrastructure::http::HttpClient;
use crate::Result;
use async_trait::async_trait;
use std::sync::Arc;

pub struct DeepLProvider {
    http_client: Arc<dyn HttpClient>,
    api_key: Option<String>,
}

impl DeepLProvider {
    pub fn new(http_client: Arc<dyn HttpClient>, api_key: Option<String>) -> Self {
        Self { http_client, api_key }
    }
    
    pub fn set_api_key(&mut self, api_key: String) {
        self.api_key = Some(api_key);
    }
}

impl Provider for DeepLProvider {
    fn id(&self) -> &str {
        "deepl"
    }
    
    fn name(&self) -> &str {
        "DeepL"
    }
    
    fn is_configured(&self) -> bool {
        self.api_key.is_some()
    }
    
    fn requires_api_key(&self) -> bool {
        true
    }
}

#[async_trait]
impl TranslationProvider for DeepLProvider {
    async fn translate(&self, request: &TranslationRequest) -> Result<TranslationResult> {
        let api_key = self.api_key.as_ref()
            .ok_or_else(|| crate::AppError::ProviderNotConfigured(self.id().to_string()))?;
        
        let source_lang = request.source_lang.as_deref().unwrap_or("");
        
        let mut json_body = serde_json::json!({
            "text": [request.text],
            "target_lang": request.target_lang.to_uppercase(),
        });
        
        if !source_lang.is_empty() && source_lang != "auto" {
            json_body["source_lang"] = serde_json::json!(source_lang.to_uppercase());
        }
        
        let url = "https://api-free.deepl.com/v2/translate";
        
        // DeepL uses form data with auth header, need to adapt
        let response = self.http_client.post(url, json_body).await?;
        
        let json: serde_json::Value = serde_json::from_str(&response.body)?;
        
        let translated_text = json["translations"][0]["text"]
            .as_str()
            .ok_or_else(|| crate::AppError::Other("Failed to parse DeepL response".to_string()))?
            .to_string();
        
        let detected_lang = json["translations"][0]["detected_source_language"]
            .as_str()
            .map(|s| s.to_lowercase());
        
        Ok(TranslationResult {
            provider_id: self.id().to_string(),
            provider_name: self.name().to_string(),
            text: translated_text,
            detected_language: detected_lang,
        })
    }
}
```

- [ ] **Step 2: Update implementations module**

```rust
// src-tauri/src/application/providers/translation/impls/mod.rs

pub mod google;
pub mod deepl;

pub use google::GoogleTranslateProvider;
pub use deepl::DeepLProvider;
```

- [ ] **Step 3: Update translation module exports**

```rust
// src-tauri/src/application/providers/translation/mod.rs

mod trait_def;
pub mod registry;
pub mod service;
pub mod impls;

pub use trait_def::TranslationProvider;
pub use registry::TranslationRegistry;
pub use service::TranslationService;
pub use impls::{GoogleTranslateProvider, DeepLProvider};
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check`
Expected: SUCCESS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/application/providers/translation/impls/deepl.rs
git add src-tauri/src/application/providers/translation/impls/mod.rs
git add src-tauri/src/application/providers/translation/mod.rs
git commit -m "feat(app): add DeepL translation provider with API key support"
```

---

## Task 7: Baidu Translation Provider

**Files:**
- Create: `src-tauri/src/application/providers/translation/impls/baidu.rs`
- Modify: `src-tauri/src/application/providers/translation/impls/mod.rs`
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Add MD5 dependency for Baidu API signature**

Add to `src-tauri/Cargo.toml` dependencies:

```toml
md5 = "0.7"
```

- [ ] **Step 2: Implement Baidu Translation Provider**

```rust
// src-tauri/src/application/providers/translation/impls/baidu.rs

use crate::application::providers::common::Provider;
use crate::application::providers::translation::TranslationProvider;
use crate::domain::translation::{TranslationRequest, TranslationResult};
use crate::infrastructure::http::HttpClient;
use crate::Result;
use async_trait::async_trait;
use std::sync::Arc;

pub struct BaiduTranslationProvider {
    http_client: Arc<dyn HttpClient>,
    app_id: Option<String>,
    secret_key: Option<String>,
}

impl BaiduTranslationProvider {
    pub fn new(http_client: Arc<dyn HttpClient>, app_id: Option<String>, secret_key: Option<String>) -> Self {
        Self { http_client, app_id, secret_key }
    }
    
    pub fn configure(&mut self, app_id: String, secret_key: String) {
        self.app_id = Some(app_id);
        self.secret_key = Some(secret_key);
    }
    
    fn generate_sign(&self, query: &str, salt: &str) -> String {
        let app_id = self.app_id.as_ref().unwrap();
        let secret_key = self.secret_key.as_ref().unwrap();
        
        let sign_str = format!("{}{}{}{}", app_id, query, salt, secret_key);
        format!("{:x}", md5::compute(sign_str.as_bytes()))
    }
}

impl Provider for BaiduTranslationProvider {
    fn id(&self) -> &str {
        "baidu-translate"
    }
    
    fn name(&self) -> &str {
        "Baidu Translate"
    }
    
    fn is_configured(&self) -> bool {
        self.app_id.is_some() && self.secret_key.is_some()
    }
    
    fn requires_api_key(&self) -> bool {
        true
    }
}

#[async_trait]
impl TranslationProvider for BaiduTranslationProvider {
    async fn translate(&self, request: &TranslationRequest) -> Result<TranslationResult> {
        if !self.is_configured() {
            return Err(crate::AppError::ProviderNotConfigured(self.id().to_string()));
        }
        
        let app_id = self.app_id.as_ref().unwrap();
        let salt = format!("{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
        let sign = self.generate_sign(&request.text, &salt);
        
        let source_lang = request.source_lang.as_deref().unwrap_or("auto");
        
        let url = format!(
            "https://fanyi-api.baidu.com/api/trans/vip/translate?q={}&from={}&to={}&appid={}&salt={}&sign={}",
            urlencoding::encode(&request.text),
            source_lang,
            &request.target_lang,
            app_id,
            salt,
            sign
        );
        
        let response = self.http_client.get(&url).await?;
        
        let json: serde_json::Value = serde_json::from_str(&response.body)?;
        
        // Check for error
        if let Some(error_code) = json["error_code"].as_str() {
            return Err(crate::AppError::Other(format!("Baidu API error: {}", error_code)));
        }
        
        let translated_text = json["trans_result"][0]["dst"]
            .as_str()
            .ok_or_else(|| crate::AppError::Other("Failed to parse Baidu response".to_string()))?
            .to_string();
        
        Ok(TranslationResult {
            provider_id: self.id().to_string(),
            provider_name: self.name().to_string(),
            text: translated_text,
            detected_language: None,
        })
    }
}
```

- [ ] **Step 3: Update implementations module**

```rust
// src-tauri/src/application/providers/translation/impls/mod.rs

pub mod google;
pub mod deepl;
pub mod baidu;

pub use google::GoogleTranslateProvider;
pub use deepl::DeepLProvider;
pub use baidu::BaiduTranslationProvider;
```

- [ ] **Step 4: Update translation module exports**

```rust
// src-tauri/src/application/providers/translation/mod.rs

mod trait_def;
pub mod registry;
pub mod service;
pub mod impls;

pub use trait_def::TranslationProvider;
pub use registry::TranslationRegistry;
pub use service::TranslationService;
pub use impls::{GoogleTranslateProvider, DeepLProvider, BaiduTranslationProvider};
```

- [ ] **Step 5: Verify compilation**

Run: `cargo check`
Expected: SUCCESS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/application/providers/translation/impls/baidu.rs
git add src-tauri/src/application/providers/translation/impls/mod.rs
git add src-tauri/src/application/providers/translation/mod.rs
git add src-tauri/Cargo.toml
git commit -m "feat(app): add Baidu translation provider with signature authentication"
```

---

## Task 8: Translation Commands

**Files:**
- Create: `src-tauri/src/commands/translation_commands.rs`
- Create: `src-tauri/src/commands/provider_commands.rs`
- Modify: `src-tauri/src/commands/mod.rs`

- [ ] **Step 1: Implement translation commands**

```rust
// src-tauri/src/commands/translation_commands.rs

use crate::domain::translation::{TranslationRequest, TranslationResult};
use tauri::State;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct TranslateTextRequest {
    pub text: String,
    pub source_lang: Option<String>,
    pub target_lang: String,
}

#[tauri::command]
pub async fn translate_text(
    request: TranslateTextRequest,
    state: State<'_, crate::AppState>,
) -> Result<Vec<TranslationResult>, String> {
    let translation_request = TranslationRequest {
        text: request.text,
        source_lang: request.source_lang,
        target_lang: request.target_lang,
    };
    
    state.translation_service
        .translate(&translation_request)
        .await
        .map_err(|e| e.to_string())
}
```

- [ ] **Step 2: Implement provider management commands**

```rust
// src-tauri/src/commands/provider_commands.rs

use tauri::State;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub is_configured: bool,
    pub requires_api_key: bool,
    pub is_active: bool,
}

#[tauri::command]
pub async fn list_translation_providers(
    state: State<'_, crate::AppState>,
) -> Result<Vec<ProviderInfo>, String> {
    let registry = state.translation_registry.lock().unwrap();
    let all_providers = registry.list_all();
    let active = registry.get_active();
    let active_ids: Vec<_> = active.iter().map(|p| p.id().to_string()).collect();
    
    let info: Vec<_> = all_providers.iter().map(|p| ProviderInfo {
        id: p.id().to_string(),
        name: p.name().to_string(),
        is_configured: p.is_configured(),
        requires_api_key: p.requires_api_key(),
        is_active: active_ids.contains(&p.id().to_string()),
    }).collect();
    
    Ok(info)
}

#[tauri::command]
pub async fn activate_translation_provider(
    provider_id: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state.translation_registry
        .lock()
        .unwrap()
        .activate(&provider_id)
        .map_err(|e| e.to_string())?;
    
    // Persist to config
    let active_ids: Vec<_> = state.translation_registry
        .lock()
        .unwrap()
        .get_active()
        .iter()
        .map(|p| p.id().to_string())
        .collect();
    
    state.config_file
        .save("active_translation_providers", &active_ids)
        .map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
pub async fn deactivate_translation_provider(
    provider_id: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    state.translation_registry
        .lock()
        .unwrap()
        .deactivate(&provider_id);
    
    // Persist to config
    let active_ids: Vec<_> = state.translation_registry
        .lock()
        .unwrap()
        .get_active()
        .iter()
        .map(|p| p.id().to_string())
        .collect();
    
    state.config_file
        .save("active_translation_providers", &active_ids)
        .map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
pub async fn configure_translation_provider(
    provider_id: String,
    api_key: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    // Save to keychain
    state.keychain
        .save_provider_credential(&provider_id, &api_key)
        .map_err(|e| e.to_string())?;
    
    // Update provider configuration
    // Note: This requires provider to support set_api_key, which would need
    // interior mutability or a different pattern. For now, providers are
    // recreated on startup with credentials from keychain.
    
    Ok(())
}
```

- [ ] **Step 3: Update commands module**

```rust
// src-tauri/src/commands/mod.rs

mod translate;
mod config;
mod translation_commands;
mod provider_commands;

pub use translate::*;
pub use config::*;
pub use translation_commands::*;
pub use provider_commands::*;
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check`
Expected: SUCCESS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/translation_commands.rs
git add src-tauri/src/commands/provider_commands.rs
git add src-tauri/src/commands/mod.rs
git commit -m "feat(commands): add translation and provider management commands"
```

---

## Task 9: Update AppState with Translation Components

**Files:**
- Modify: `src-tauri/src/lib.rs:30-100`

- [ ] **Step 1: Update AppState structure**

```rust
// src-tauri/src/lib.rs (replace AppState)

use std::sync::{Arc, Mutex};
use application::providers::translation::{TranslationRegistry, TranslationService, GoogleTranslateProvider, DeepLProvider, BaiduTranslationProvider};
use infrastructure::{ConfigFile, Keychain, HttpClient, ReqwestHttpClient, get_config_path};

pub struct AppState {
    // Infrastructure
    pub config_file: Arc<ConfigFile>,
    pub keychain: Arc<Keychain>,
    pub http_client: Arc<dyn HttpClient>,
    
    // Translation
    pub translation_registry: Arc<Mutex<TranslationRegistry>>,
    pub translation_service: Arc<TranslationService>,
    
    // Old components (to be removed in later phases)
    pub language_detector: language::LanguageDetector,
    pub hotkey_manager: hotkeys::HotkeyManager,
}

impl AppState {
    pub fn new(app: tauri::AppHandle) -> crate::Result<Self> {
        // 1. Infrastructure
        let config_path = get_config_path()?;
        let config_file = Arc::new(ConfigFile::new(config_path));
        let keychain = Arc::new(Keychain::new());
        let http_client: Arc<dyn HttpClient> = Arc::new(ReqwestHttpClient::new());
        
        // 2. Translation Providers
        let mut translation_registry = TranslationRegistry::new();
        
        // Register Google Translate
        translation_registry.register(Arc::new(
            GoogleTranslateProvider::new(Arc::clone(&http_client))
        ));
        
        // Register DeepL
        let deepl_api_key = keychain.load_provider_credential("deepl").ok();
        translation_registry.register(Arc::new(
            DeepLProvider::new(Arc::clone(&http_client), deepl_api_key)
        ));
        
        // Register Baidu Translate
        let baidu_app_id = keychain.load_provider_credential("baidu_translate_app_id").ok();
        let baidu_secret = keychain.load_provider_credential("baidu_translate_secret").ok();
        translation_registry.register(Arc::new(
            BaiduTranslationProvider::new(Arc::clone(&http_client), baidu_app_id, baidu_secret)
        ));
        
        // Restore active providers from config
        if let Ok(active_ids) = config_file.load::<Vec<String>>("active_translation_providers") {
            for id in active_ids {
                let _ = translation_registry.activate(&id);
            }
        } else {
            // Default: activate Google Translate
            let _ = translation_registry.activate("google-translate");
        }
        
        let translation_registry = Arc::new(Mutex::new(translation_registry));
        let translation_service = Arc::new(TranslationService::new(
            Arc::clone(&translation_registry),
        ));
        
        // Old components
        let language_detector = language::LanguageDetector::new();
        let hotkey_manager = hotkeys::HotkeyManager::new(app);
        
        Ok(Self {
            config_file,
            keychain,
            http_client,
            translation_registry,
            translation_service,
            language_detector,
            hotkey_manager,
        })
    }
}
```

- [ ] **Step 2: Update run() function to use new AppState**

```rust
// src-tauri/src/lib.rs (update run function)

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
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

      let app_state = AppState::new(app.handle().clone())
          .expect("Failed to initialize AppState");
      app.manage(app_state);

      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      commands::translate_text,
      commands::list_translation_providers,
      commands::activate_translation_provider,
      commands::deactivate_translation_provider,
      commands::configure_translation_provider,
      commands::detect_language,
      commands::get_config,
      commands::update_config,
      commands::open_result_window,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check`
Expected: SUCCESS

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "refactor(app): update AppState with new Translation architecture"
```

---

## Task 10: Integration Test and Frontend Verification

**Files:**
- Create: `src-tauri/tests/translation_integration_test.rs`
- Test frontend: Verify existing React UI still works

- [ ] **Step 1: Write integration test**

```rust
// src-tauri/tests/translation_integration_test.rs

use snaplingo_lib::{AppState, Result};
use tempfile::tempdir;

#[tokio::test]
async fn test_translation_full_flow() -> Result<()> {
    // Note: This test uses a simplified AppState setup
    // In real test, you'd need to mock tauri::AppHandle
    
    // For now, test individual components
    use snaplingo_lib::application::providers::translation::{
        TranslationRegistry, TranslationService, GoogleTranslateProvider
    };
    use snaplingo_lib::infrastructure::http::ReqwestHttpClient;
    use snaplingo_lib::domain::translation::TranslationRequest;
    use std::sync::{Arc, Mutex};
    
    let http_client = Arc::new(ReqwestHttpClient::new());
    let mut registry = TranslationRegistry::new();
    
    registry.register(Arc::new(GoogleTranslateProvider::new(http_client)));
    registry.activate("google-translate")?;
    
    let service = TranslationService::new(Arc::new(Mutex::new(registry)));
    
    let request = TranslationRequest::new("hello".to_string(), "zh".to_string());
    let results = service.translate(&request).await?;
    
    assert!(!results.is_empty());
    assert_eq!(results[0].provider_id, "google-translate");
    
    Ok(())
}
```

- [ ] **Step 2: Run integration test**

Run: `cargo test --test translation_integration_test`
Expected: PASS

- [ ] **Step 3: Build application**

Run: `cargo build`
Expected: SUCCESS

- [ ] **Step 4: Manual frontend test**

Run: `cargo tauri dev`

Test in frontend:
1. Open translation window
2. Input "hello"
3. Click translate
4. Verify translation appears

- [ ] **Step 5: Commit integration test**

```bash
git add src-tauri/tests/translation_integration_test.rs
git commit -m "test(integration): add translation integration test"
```

---

## Task 11: Delete Old Translation Module

**Files:**
- Delete: `src-tauri/src/translate/` (entire directory)
- Modify: `src-tauri/src/lib.rs` (remove old translate module)

- [ ] **Step 1: Verify new implementation works**

Run all tests:
```bash
cargo test
```

Expected: All tests PASS

- [ ] **Step 2: Remove old translate module from lib.rs**

```rust
// src-tauri/src/lib.rs (remove these lines)

// DELETE:
// mod translate;
// use translate::{GoogleTranslateProvider, TranslationProvider};
```

- [ ] **Step 3: Delete old translate directory**

```bash
rm -rf src-tauri/src/translate/
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check`
Expected: SUCCESS

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "refactor: remove old translate module, migration complete"
```

---

## Phase 2 Completion Checklist

- [ ] Provider base trait defined
- [ ] TranslationProvider trait defined
- [ ] TranslationRegistry (multi-select) implemented with tests
- [ ] TranslationService (concurrent execution) implemented with tests
- [ ] Google Translate migrated to new architecture
- [ ] DeepL provider added
- [ ] Baidu Translation provider added
- [ ] Translation commands implemented
- [ ] Provider management commands implemented
- [ ] AppState updated with new components
- [ ] Integration test passes
- [ ] Frontend verification complete
- [ ] Old translate module deleted
- [ ] All tests pass
- [ ] Application runs successfully

**Next Phase:** Phase 3 - OCR Provider vertical slice (Tesseract, Baidu OCR)

**Estimated Time:** 2-3 days
