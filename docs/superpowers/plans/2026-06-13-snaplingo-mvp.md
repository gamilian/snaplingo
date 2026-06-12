# SnapLingo MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a cross-platform desktop app that unifies screenshot capture, OCR, and translation with multi-provider support

**Architecture:** Tauri 2.0 backend (Rust) + React frontend (TailwindCSS). Backend handles system integration (hotkeys, capture, clipboard), provider HTTP clients, and persistence. Frontend handles all UI rendering (canvas editor, result window, settings).

**Tech Stack:** Rust (Tauri 2.0, reqwest, image, arboard, enigo, lingua), React (Canvas API, TailwindCSS), SQLite (history), System credential stores (Keychain/Credential Manager/Secret Service)

---

## Project Structure

```
snaplingo/
├── src-tauri/          # Rust backend
│   ├── src/
│   │   ├── main.rs
│   │   ├── capture/    # Screenshot capture, hotkeys
│   │   ├── ocr/        # OCR provider implementations
│   │   ├── translate/  # Translation provider implementations
│   │   ├── config/     # Configuration management
│   │   ├── history/    # History persistence
│   │   ├── image/      # Image processing, layer composition
│   │   └── tts/        # Text-to-speech
│   └── Cargo.toml
├── src/                # React frontend
│   ├── components/
│   │   ├── ScreenshotEditor/
│   │   ├── ResultWindow/
│   │   ├── SettingsWindow/
│   │   └── HistoryWindow/
│   ├── hooks/
│   ├── stores/
│   └── main.tsx
└── package.json
```

---

### Task 1: Project Scaffold and Dependencies

**Files:**
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/tauri.conf.json`
- Create: `package.json`
- Create: `src/main.tsx`
- Create: `.gitignore`

- [ ] **Step 1: Initialize Tauri project**

```bash
cd /Users/gamilian/work/code/snaplingo
npm create tauri-app@latest . -- --template react-ts
```

Expected: Tauri 2.0 project scaffold created

- [ ] **Step 2: Add Rust dependencies to Cargo.toml**

```toml
[dependencies]
tauri = { version = "2.0", features = ["macos-private-api", "system-tray"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.11", features = ["json"] }
image = "0.24"
arboard = "3.2"
enigo = "0.1"
lingua = "1.5"
rusqlite = { version = "0.29", features = ["bundled"] }
keyring = "2.0"
dirs = "5.0"
anyhow = "1.0"
thiserror = "1.0"

[target.'cfg(target_os = "macos")'.dependencies]
core-graphics = "0.23"

[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.51", features = ["Win32_UI_Input_KeyboardAndMouse"] }

[target.'cfg(target_os = "linux")'.dependencies]
xcb = "1.2"
```

- [ ] **Step 3: Add frontend dependencies to package.json**

```bash
npm install react react-dom
npm install -D @types/react @types/react-dom
npm install tailwindcss postcss autoprefixer
npm install zustand
npm install @tauri-apps/api@^2.0.0
npm install -D vite @vitejs/plugin-react typescript
npx tailwindcss init -p
```

- [ ] **Step 4: Configure TailwindCSS**

Edit `tailwind.config.js`:

```javascript
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {},
  },
  plugins: [],
}
```

- [ ] **Step 5: Create .gitignore**

```
# Dependencies
node_modules/
target/

# Build outputs
dist/
src-tauri/target/

# Environment
.env
.env.local

# OS
.DS_Store
Thumbs.db

# IDE
.vscode/
.idea/
*.swp
*.swo

# User data
.snaplingo/
```

- [ ] **Step 6: Verify build**

```bash
npm install
npm run tauri build
```

Expected: Successful build, app opens with default Tauri window

- [ ] **Step 7: Commit**

```bash
git add .
git commit -m "feat: initialize Tauri 2.0 + React project scaffold"
```

---

### Task 2: Configuration Module (Rust)

**Files:**
- Create: `src-tauri/src/config/mod.rs`
- Create: `src-tauri/src/config/types.rs`
- Create: `src-tauri/src/config/storage.rs`
- Create: `src-tauri/src/config/tests.rs`

- [ ] **Step 1: Write failing test for config loading**

`src-tauri/src/config/tests.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
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
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd src-tauri
cargo test test_load_default_config
```

Expected: FAIL with "Config not found"

- [ ] **Step 3: Define config types**

`src-tauri/src/config/types.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub version: String,
    pub general: GeneralConfig,
    pub screenshot: ScreenshotConfig,
    pub ocr: OcrConfig,
    pub translation: TranslationConfig,
    pub hotkeys: HotkeysConfig,
    pub history: HistoryConfig,
    pub advanced: AdvancedConfig,
    pub custom_providers: Vec<CustomProvider>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub language: String,
    pub theme: String,
    pub start_on_boot: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotConfig {
    pub default_save_path: String,
    pub format: String,
    pub quality: u8,
    pub default_tool_color: String,
    pub default_stroke_width: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrConfig {
    pub active_provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationConfig {
    pub active_providers: Vec<String>,
    pub default_target_language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeysConfig {
    pub screenshot: String,
    pub ocr: String,
    pub ocr_translate: String,
    pub selection_translate: String,
    pub input_translate: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryConfig {
    pub record_screenshot: bool,
    pub record_ocr: bool,
    pub record_translation: bool,
    pub auto_cleanup_enabled: bool,
    pub max_age_days: u32,
    pub max_entries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedConfig {
    pub proxy_url: Option<String>,
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomProvider {
    pub id: String,
    pub name: String,
    pub provider_type: String,
    pub api_format: String,
    pub endpoint: String,
    pub model: String,
}
```

- [ ] **Step 4: Implement default config**

`src-tauri/src/config/mod.rs`:

```rust
mod types;
mod storage;

pub use types::*;
pub use storage::*;

use anyhow::Result;
use std::path::PathBuf;

impl Config {
    pub fn default() -> Self {
        Config {
            version: "1.0.0".to_string(),
            general: GeneralConfig {
                language: "en".to_string(),
                theme: "system".to_string(),
                start_on_boot: true,
            },
            screenshot: ScreenshotConfig {
                default_save_path: "~/Pictures/SnapLingo".to_string(),
                format: "png".to_string(),
                quality: 95,
                default_tool_color: "#FF0000".to_string(),
                default_stroke_width: 3,
            },
            ocr: OcrConfig {
                active_provider: "tesseract".to_string(),
            },
            translation: TranslationConfig {
                active_providers: vec!["google-translate".to_string()],
                default_target_language: "zh-CN".to_string(),
            },
            hotkeys: HotkeysConfig {
                screenshot: "F1".to_string(),
                ocr: "Option+A".to_string(),
                ocr_translate: "Option+S".to_string(),
                selection_translate: "Option+D".to_string(),
                input_translate: "Option+W".to_string(),
            },
            history: HistoryConfig {
                record_screenshot: false,
                record_ocr: true,
                record_translation: true,
                auto_cleanup_enabled: true,
                max_age_days: 30,
                max_entries: 1000,
            },
            advanced: AdvancedConfig {
                proxy_url: None,
                log_level: "info".to_string(),
            },
            custom_providers: vec![],
        }
    }
    
    pub fn load_or_default(path: &PathBuf) -> Result<Self> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }
}
```

- [ ] **Step 5: Run test to verify it passes**

```bash
cd src-tauri
cargo test test_load_default_config
```

Expected: PASS

- [ ] **Step 6: Write test for config persistence**

Add to `src-tauri/src/config/tests.rs`:

```rust
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
```

- [ ] **Step 7: Run test to verify it fails**

```bash
cargo test test_save_and_load_config
```

Expected: FAIL with "method not found: save"

- [ ] **Step 8: Implement config save**

Add to `src-tauri/src/config/mod.rs`:

```rust
impl Config {
    pub fn save(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
```

- [ ] **Step 9: Run test to verify it passes**

```bash
cargo test test_save_and_load_config
```

Expected: PASS

- [ ] **Step 10: Commit**

```bash
git add src-tauri/src/config/
git commit -m "feat: implement config module with load/save"
```

---

### Task 3: Provider Trait Definitions (Rust)

**Files:**
- Create: `src-tauri/src/ocr/mod.rs`
- Create: `src-tauri/src/ocr/provider.rs`
- Create: `src-tauri/src/translate/mod.rs`
- Create: `src-tauri/src/translate/provider.rs`

- [ ] **Step 1: Define OCR provider trait**

`src-tauri/src/ocr/provider.rs`:

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResult {
    pub text: String,
    pub confidence: Option<f32>,
    pub language: Option<String>,
}

#[async_trait]
pub trait OcrProvider: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    async fn recognize(&self, image_data: &[u8]) -> Result<OcrResult>;
    fn requires_api_key(&self) -> bool;
}
```

- [ ] **Step 2: Define translation provider trait**

`src-tauri/src/translate/provider.rs`:

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationResult {
    pub provider_id: String,
    pub text: String,
    pub detected_language: Option<String>,
}

#[async_trait]
pub trait TranslationProvider: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    async fn translate(
        &self,
        text: &str,
        from: &str,
        to: &str,
    ) -> Result<TranslationResult>;
    fn requires_api_key(&self) -> bool;
}
```

- [ ] **Step 3: Create module exports**

`src-tauri/src/ocr/mod.rs`:

```rust
mod provider;

pub use provider::{OcrProvider, OcrResult};
```

`src-tauri/src/translate/mod.rs`:

```rust
mod provider;

pub use provider::{TranslationProvider, TranslationResult};
```

- [ ] **Step 4: Add async_trait dependency**

Add to `src-tauri/Cargo.toml`:

```toml
async-trait = "0.1"
```

- [ ] **Step 5: Verify compilation**

```bash
cd src-tauri
cargo build
```

Expected: SUCCESS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/ocr/ src-tauri/src/translate/
git commit -m "feat: define OCR and translation provider traits"
```

---

### Task 4: Google Translate Provider Implementation

**Files:**
- Create: `src-tauri/src/translate/google.rs`
- Create: `src-tauri/src/translate/google_test.rs`

- [ ] **Step 1: Write failing test for Google Translate**

`src-tauri/src/translate/google_test.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{mock, server_url};

    #[tokio::test]
    async fn test_google_translate_success() {
        let _m = mock("POST", "/translate_a/single")
            .with_status(200)
            .with_body(r#"[[["Hello","你好",null,null,10]],null,"zh-CN"]"#)
            .create();

        let provider = GoogleTranslateProvider::new(server_url());
        let result = provider.translate("你好", "zh-CN", "en").await.unwrap();

        assert_eq!(result.text, "Hello");
        assert_eq!(result.provider_id, "google-translate");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test test_google_translate_success
```

Expected: FAIL with "GoogleTranslateProvider not found"

- [ ] **Step 3: Implement Google Translate provider**

`src-tauri/src/translate/google.rs`:

```rust
use super::provider::{TranslationProvider, TranslationResult};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

pub struct GoogleTranslateProvider {
    client: Client,
    base_url: String,
}

impl GoogleTranslateProvider {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }

    pub fn default() -> Self {
        Self::new("https://translate.googleapis.com".to_string())
    }
}

#[async_trait]
impl TranslationProvider for GoogleTranslateProvider {
    fn id(&self) -> &str {
        "google-translate"
    }

    fn name(&self) -> &str {
        "Google Translate"
    }

    async fn translate(&self, text: &str, from: &str, to: &str) -> Result<TranslationResult> {
        let url = format!(
            "{}/translate_a/single?client=gtx&sl={}&tl={}&dt=t&q={}",
            self.base_url,
            from,
            to,
            urlencoding::encode(text)
        );

        let response = self.client.get(&url).send().await?;
        let json: Value = response.json().await?;

        let translated = json[0][0][0]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid response format"))?
            .to_string();

        Ok(TranslationResult {
            provider_id: self.id().to_string(),
            text: translated,
            detected_language: json[2].as_str().map(String::from),
        })
    }

    fn requires_api_key(&self) -> bool {
        false
    }
}
```

- [ ] **Step 4: Add dependencies**

Add to `src-tauri/Cargo.toml`:

```toml
urlencoding = "2.1"
mockito = "1.2"
```

- [ ] **Step 5: Update translate module exports**

Edit `src-tauri/src/translate/mod.rs`:

```rust
mod provider;
mod google;

pub use provider::{TranslationProvider, TranslationResult};
pub use google::GoogleTranslateProvider;
```

- [ ] **Step 6: Run test to verify it passes**

```bash
cargo test test_google_translate_success
```

Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/translate/
git commit -m "feat: implement Google Translate provider"
```

---

### Task 5: Language Detection Module

**Files:**
- Create: `src-tauri/src/language/mod.rs`
- Create: `src-tauri/src/language/detector.rs`
- Create: `src-tauri/src/language/tests.rs`

- [ ] **Step 1: Write failing test for language detection**

`src-tauri/src/language/tests.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_chinese() {
        let detector = LanguageDetector::new();
        let lang = detector.detect("你好世界").unwrap();
        assert_eq!(lang, "zh-CN");
    }

    #[test]
    fn test_detect_english() {
        let detector = LanguageDetector::new();
        let lang = detector.detect("Hello world").unwrap();
        assert_eq!(lang, "en");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test test_detect_chinese
```

Expected: FAIL with "LanguageDetector not found"

- [ ] **Step 3: Implement language detector**

`src-tauri/src/language/detector.rs`:

```rust
use anyhow::Result;
use lingua::{Language, LanguageDetectorBuilder};

pub struct LanguageDetector {
    detector: lingua::LanguageDetector,
}

impl LanguageDetector {
    pub fn new() -> Self {
        let detector = LanguageDetectorBuilder::from_all_languages().build();
        Self { detector }
    }

    pub fn detect(&self, text: &str) -> Result<String> {
        let detected = self.detector.detect_language_of(text)
            .ok_or_else(|| anyhow::anyhow!("Unable to detect language"))?;

        let lang_code = match detected {
            Language::Chinese => "zh-CN",
            Language::English => "en",
            Language::Spanish => "es",
            Language::Japanese => "ja",
            Language::French => "fr",
            Language::German => "de",
            Language::Korean => "ko",
            Language::Russian => "ru",
            _ => "en",
        };

        Ok(lang_code.to_string())
    }

    pub fn smart_target(&self, source: &str) -> String {
        if source.starts_with("zh") {
            "en".to_string()
        } else {
            "zh-CN".to_string()
        }
    }
}
```

- [ ] **Step 4: Create module exports**

`src-tauri/src/language/mod.rs`:

```rust
mod detector;

pub use detector::LanguageDetector;
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cargo test test_detect_chinese && cargo test test_detect_english
```

Expected: PASS (both tests)

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/language/
git commit -m "feat: implement language detection with lingua"
```

---

### Task 6: Tauri Command Handlers (Backend API)

**Files:**
- Create: `src-tauri/src/commands/mod.rs`
- Create: `src-tauri/src/commands/translate.rs`
- Create: `src-tauri/src/commands/config.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Implement translate command**

`src-tauri/src/commands/translate.rs`:

```rust
use crate::translate::{TranslationProvider, TranslationResult};
use crate::language::LanguageDetector;
use anyhow::Result;

#[tauri::command]
pub async fn translate_text(
    text: String,
    from: String,
    to: String,
    provider_ids: Vec<String>,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<TranslationResult>, String> {
    let mut results = Vec::new();
    
    for provider_id in provider_ids {
        if let Some(provider) = state.get_translation_provider(&provider_id) {
            match provider.translate(&text, &from, &to).await {
                Ok(result) => results.push(result),
                Err(e) => eprintln!("Provider {} failed: {}", provider_id, e),
            }
        }
    }
    
    if results.is_empty() {
        Err("All providers failed".to_string())
    } else {
        Ok(results)
    }
}

#[tauri::command]
pub fn detect_language(
    text: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    state.language_detector.detect(&text)
        .map_err(|e| e.to_string())
}
```

- [ ] **Step 2: Implement config commands**

`src-tauri/src/commands/config.rs`:

```rust
use crate::config::Config;
use anyhow::Result;
use std::path::PathBuf;

#[tauri::command]
pub fn get_config(state: tauri::State<'_, AppState>) -> Result<Config, String> {
    Ok(state.config.lock().unwrap().clone())
}

#[tauri::command]
pub fn update_config(
    updates: serde_json::Value,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut config = state.config.lock().unwrap();
    
    // Merge updates into config
    let mut config_value = serde_json::to_value(&*config).unwrap();
    json_patch::merge(&mut config_value, &updates);
    *config = serde_json::from_value(config_value).map_err(|e| e.to_string())?;
    
    // Save to disk
    let config_path = state.config_path.clone();
    config.save(&config_path).map_err(|e| e.to_string())?;
    
    Ok(())
}
```

- [ ] **Step 3: Create commands module exports**

`src-tauri/src/commands/mod.rs`:

```rust
mod translate;
mod config;

pub use translate::*;
pub use config::*;
```

- [ ] **Step 4: Define AppState structure**

Add to `src-tauri/src/main.rs`:

```rust
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

pub struct AppState {
    config: Arc<Mutex<Config>>,
    config_path: PathBuf,
    translation_providers: Arc<Mutex<HashMap<String, Box<dyn TranslationProvider>>>>,
    language_detector: LanguageDetector,
}

impl AppState {
    pub fn new(config_path: PathBuf) -> Self {
        let config = Config::load_or_default(&config_path).unwrap_or_default();
        let mut providers: HashMap<String, Box<dyn TranslationProvider>> = HashMap::new();
        
        // Register built-in providers
        providers.insert(
            "google-translate".to_string(),
            Box::new(GoogleTranslateProvider::default()),
        );
        
        Self {
            config: Arc::new(Mutex::new(config)),
            config_path,
            translation_providers: Arc::new(Mutex::new(providers)),
            language_detector: LanguageDetector::new(),
        }
    }
    
    pub fn get_translation_provider(&self, id: &str) -> Option<Box<dyn TranslationProvider>> {
        self.translation_providers.lock().unwrap().get(id).cloned()
    }
}
```

- [ ] **Step 5: Wire commands into main.rs**

Update `src-tauri/src/main.rs`:

```rust
mod config;
mod translate;
mod language;
mod commands;

use config::Config;
use translate::GoogleTranslateProvider;
use language::LanguageDetector;
use std::path::PathBuf;

fn main() {
    let config_dir = dirs::home_dir()
        .unwrap()
        .join(".snaplingo");
    std::fs::create_dir_all(&config_dir).unwrap();
    let config_path = config_dir.join("config.json");
    
    let app_state = AppState::new(config_path);

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::translate_text,
            commands::detect_language,
            commands::get_config,
            commands::update_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 6: Add json-patch dependency**

Add to `src-tauri/Cargo.toml`:

```toml
json-patch = "1.2"
```

- [ ] **Step 7: Verify compilation**

```bash
cd src-tauri
cargo build
```

Expected: SUCCESS

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/commands/ src-tauri/src/main.rs
git commit -m "feat: implement Tauri command handlers for translation and config"
```

---

### Task 7: Frontend State Management (React)

**Files:**
- Create: `src/stores/appStore.ts`
- Create: `src/hooks/useTranslate.ts`

- [ ] **Step 1: Create Zustand store**

`src/stores/appStore.ts`:

```typescript
import { create } from 'zustand';

interface TranslationResult {
  provider_id: string;
  text: string;
  detected_language?: string;
}

interface AppState {
  sourceText: string;
  sourceLang: string;
  targetLang: string;
  translations: TranslationResult[];
  isTranslating: boolean;
  resultWindowVisible: boolean;
  
  setSourceText: (text: string) => void;
  setSourceLang: (lang: string) => void;
  setTargetLang: (lang: string) => void;
  setTranslations: (results: TranslationResult[]) => void;
  setTranslating: (value: boolean) => void;
  showResultWindow: () => void;
  hideResultWindow: () => void;
  reset: () => void;
}

export const useAppStore = create<AppState>((set) => ({
  sourceText: '',
  sourceLang: 'auto',
  targetLang: 'zh-CN',
  translations: [],
  isTranslating: false,
  resultWindowVisible: false,
  
  setSourceText: (text) => set({ sourceText: text }),
  setSourceLang: (lang) => set({ sourceLang: lang }),
  setTargetLang: (lang) => set({ targetLang: lang }),
  setTranslations: (results) => set({ translations: results }),
  setTranslating: (value) => set({ isTranslating: value }),
  showResultWindow: () => set({ resultWindowVisible: true }),
  hideResultWindow: () => set({ resultWindowVisible: false }),
  reset: () => set({
    sourceText: '',
    sourceLang: 'auto',
    targetLang: 'zh-CN',
    translations: [],
    isTranslating: false,
  }),
}));
```

- [ ] **Step 2: Create translation hook**

`src/hooks/useTranslate.ts`:

```typescript
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../stores/appStore';

export function useTranslate() {
  const { 
    sourceText, 
    sourceLang, 
    targetLang, 
    setTranslations, 
    setTranslating 
  } = useAppStore();

  const translate = async (text?: string, from?: string, to?: string) => {
    const textToTranslate = text || sourceText;
    const fromLang = from || sourceLang;
    const toLang = to || targetLang;

    if (!textToTranslate.trim()) return;

    setTranslating(true);
    
    try {
      let detectedFrom = fromLang;
      
      if (fromLang === 'auto') {
        detectedFrom = await invoke<string>('detect_language', { 
          text: textToTranslate 
        });
      }

      const results = await invoke<any[]>('translate_text', {
        text: textToTranslate,
        from: detectedFrom,
        to: toLang,
        providerIds: ['google-translate'],
      });

      setTranslations(results);
    } catch (error) {
      console.error('Translation failed:', error);
      setTranslations([]);
    } finally {
      setTranslating(false);
    }
  };

  return { translate };
}
```

- [ ] **Step 3: Verify TypeScript compilation**

```bash
npm run build
```

Expected: SUCCESS

- [ ] **Step 4: Commit**

```bash
git add src/stores/ src/hooks/
git commit -m "feat: implement frontend state management with Zustand"
```

---

### Task 8: Result Window Component

**Files:**
- Create: `src/components/ResultWindow/ResultWindow.tsx`
- Create: `src/components/ResultWindow/TranslationCard.tsx`
- Create: `src/components/ResultWindow/index.ts`

- [ ] **Step 1: Create TranslationCard component**

`src/components/ResultWindow/TranslationCard.tsx`:

```typescript
import React, { useState } from 'react';

interface TranslationCardProps {
  providerId: string;
  text: string;
  onCopy: () => void;
}

export function TranslationCard({ providerId, text, onCopy }: TranslationCardProps) {
  const [isCollapsed, setIsCollapsed] = useState(false);

  const providerNames: Record<string, string> = {
    'google-translate': 'Google Translate',
  };

  return (
    <div className="border rounded-lg p-4 mb-3 bg-white shadow-sm">
      <div className="flex items-center justify-between mb-2">
        <h3 className="font-semibold text-gray-700">
          {providerNames[providerId] || providerId}
        </h3>
        <div className="flex gap-2">
          <button
            onClick={() => setIsCollapsed(!isCollapsed)}
            className="text-sm text-gray-500 hover:text-gray-700"
          >
            {isCollapsed ? 'Expand' : 'Collapse'}
          </button>
          <button
            onClick={onCopy}
            className="px-3 py-1 text-sm bg-blue-500 text-white rounded hover:bg-blue-600"
          >
            Copy
          </button>
        </div>
      </div>
      {!isCollapsed && (
        <p className="text-gray-900 whitespace-pre-wrap">{text}</p>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Create ResultWindow component**

`src/components/ResultWindow/ResultWindow.tsx`:

```typescript
import React from 'react';
import { useAppStore } from '../../stores/appStore';
import { useTranslate } from '../../hooks/useTranslate';
import { TranslationCard } from './TranslationCard';

export function ResultWindow() {
  const {
    sourceText,
    sourceLang,
    targetLang,
    translations,
    isTranslating,
    resultWindowVisible,
    setSourceText,
    setSourceLang,
    setTargetLang,
    hideResultWindow,
  } = useAppStore();

  const { translate } = useTranslate();

  if (!resultWindowVisible) return null;

  const handleCopy = (text: string) => {
    navigator.clipboard.writeText(text);
  };

  const handleSwapLanguages = () => {
    if (sourceLang !== 'auto') {
      setSourceLang(targetLang);
      setTargetLang(sourceLang);
    }
  };

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-gray-50 rounded-lg shadow-xl w-[600px] max-h-[80vh] flex flex-col">
        <div className="p-4 border-b flex items-center justify-between">
          <h2 className="text-xl font-bold">Translation</h2>
          <button
            onClick={hideResultWindow}
            className="text-gray-500 hover:text-gray-700"
          >
            ✕
          </button>
        </div>

        <div className="p-4 overflow-y-auto flex-1">
          <textarea
            value={sourceText}
            onChange={(e) => setSourceText(e.target.value)}
            className="w-full p-3 border rounded-lg mb-4 resize-none"
            rows={4}
            placeholder="Enter text to translate..."
          />

          <div className="flex gap-2 mb-4 items-center">
            <select
              value={sourceLang}
              onChange={(e) => setSourceLang(e.target.value)}
              className="px-3 py-2 border rounded"
            >
              <option value="auto">Auto Detect</option>
              <option value="en">English</option>
              <option value="zh-CN">Chinese</option>
              <option value="ja">Japanese</option>
              <option value="es">Spanish</option>
            </select>

            <button
              onClick={handleSwapLanguages}
              className="px-3 py-2 text-gray-600 hover:text-gray-800"
            >
              ⇄
            </button>

            <select
              value={targetLang}
              onChange={(e) => setTargetLang(e.target.value)}
              className="px-3 py-2 border rounded"
            >
              <option value="en">English</option>
              <option value="zh-CN">Chinese</option>
              <option value="ja">Japanese</option>
              <option value="es">Spanish</option>
            </select>

            <button
              onClick={() => translate()}
              disabled={isTranslating || !sourceText.trim()}
              className="ml-auto px-6 py-2 bg-blue-500 text-white rounded hover:bg-blue-600 disabled:bg-gray-300"
            >
              {isTranslating ? 'Translating...' : 'Translate'}
            </button>
          </div>

          <div className="space-y-3">
            {translations.map((result) => (
              <TranslationCard
                key={result.provider_id}
                providerId={result.provider_id}
                text={result.text}
                onCopy={() => handleCopy(result.text)}
              />
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 3: Create index exports**

`src/components/ResultWindow/index.ts`:

```typescript
export { ResultWindow } from './ResultWindow';
export { TranslationCard } from './TranslationCard';
```

- [ ] **Step 4: Update main App component**

`src/main.tsx`:

```typescript
import React from 'react';
import ReactDOM from 'react-dom/client';
import { ResultWindow } from './components/ResultWindow';
import './index.css';

function App() {
  return (
    <div className="w-screen h-screen bg-gray-100">
      <ResultWindow />
      <div className="flex items-center justify-center h-full">
        <p className="text-gray-500">SnapLingo is running in the system tray</p>
      </div>
    </div>
  );
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
```

- [ ] **Step 5: Create global styles**

`src/index.css`:

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

body {
  margin: 0;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', 'Oxygen',
    'Ubuntu', 'Cantarell', 'Fira Sans', 'Droid Sans', 'Helvetica Neue',
    sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}
```

- [ ] **Step 6: Test the UI**

```bash
npm run tauri dev
```

Expected: App opens, shows "SnapLingo is running in the system tray" message

- [ ] **Step 7: Commit**

```bash
git add src/components/ src/main.tsx src/index.css
git commit -m "feat: implement ResultWindow UI component"
```

---

### Task 9: Input Translation Mode (Manual Entry)

**Files:**
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Add open_result_window command**

Add to `src-tauri/src/commands/mod.rs`:

```rust
#[tauri::command]
pub fn open_result_window(
    text: String,
    app: tauri::AppHandle,
) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        
        // Emit event to frontend with text
        window.emit("input-translation", text)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
```

- [ ] **Step 2: Register command in main.rs**

Update `src-tauri/src/main.rs` invoke_handler:

```rust
.invoke_handler(tauri::generate_handler![
    commands::translate_text,
    commands::detect_language,
    commands::get_config,
    commands::update_config,
    commands::open_result_window,
])
```

- [ ] **Step 3: Add event listener in frontend**

Update `src/main.tsx`:

```typescript
import { listen } from '@tauri-apps/api/event';
import { useAppStore } from './stores/appStore';

function App() {
  React.useEffect(() => {
    const unlisten = listen<string>('input-translation', (event) => {
      const { setSourceText, showResultWindow } = useAppStore.getState();
      setSourceText(event.payload);
      showResultWindow();
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  return (
    <div className="w-screen h-screen bg-gray-100">
      <ResultWindow />
      <div className="flex items-center justify-center h-full">
        <p className="text-gray-500">SnapLingo is running in the system tray</p>
      </div>
    </div>
  );
}
```

- [ ] **Step 4: Test manual input translation**

```bash
npm run tauri dev
```

In the Tauri dev console, call:
```javascript
invoke('open_result_window', { text: 'Hello world' })
```

Expected: Result window opens with "Hello world" pre-filled

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/ src-tauri/src/main.rs src/main.tsx
git commit -m "feat: implement input translation mode"
```

---

### Task 10: Global Hotkey Registration (macOS)

**Files:**
- Create: `src-tauri/src/hotkeys/mod.rs`
- Create: `src-tauri/src/hotkeys/macos.rs`
- Create: `src-tauri/src/hotkeys/types.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Define hotkey types**

`src-tauri/src/hotkeys/types.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum CaptureMode {
    Screenshot,
    Ocr,
    OcrTranslate,
    SelectionTranslate,
    InputTranslate,
}

#[derive(Debug, Clone)]
pub struct HotkeyConfig {
    pub mode: CaptureMode,
    pub keys: String,
}
```

- [ ] **Step 2: Implement macOS hotkey registration**

`src-tauri/src/hotkeys/macos.rs`:

```rust
#[cfg(target_os = "macos")]
use super::types::{CaptureMode, HotkeyConfig};
use anyhow::Result;
use tauri::AppHandle;

pub struct HotkeyManager {
    app: AppHandle,
}

impl HotkeyManager {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }

    pub fn register(&self, config: HotkeyConfig) -> Result<()> {
        // Placeholder: will implement platform-specific registration
        println!("Registering hotkey: {:?} - {}", config.mode, config.keys);
        Ok(())
    }

    pub fn unregister_all(&self) -> Result<()> {
        println!("Unregistering all hotkeys");
        Ok(())
    }
}
```

- [ ] **Step 3: Create hotkeys module exports**

`src-tauri/src/hotkeys/mod.rs`:

```rust
mod types;

#[cfg(target_os = "macos")]
mod macos;

pub use types::{CaptureMode, HotkeyConfig};

#[cfg(target_os = "macos")]
pub use macos::HotkeyManager;
```

- [ ] **Step 4: Add hotkey registration to AppState**

Update `src-tauri/src/main.rs`:

```rust
mod hotkeys;

use hotkeys::{HotkeyManager, HotkeyConfig, CaptureMode};

// Add to AppState
pub struct AppState {
    // ... existing fields ...
    #[cfg(target_os = "macos")]
    hotkey_manager: Arc<Mutex<HotkeyManager>>,
}

// Update AppState::new
impl AppState {
    pub fn new(config_path: PathBuf, app: AppHandle) -> Self {
        // ... existing code ...
        
        #[cfg(target_os = "macos")]
        let hotkey_manager = Arc::new(Mutex::new(HotkeyManager::new(app.clone())));
        
        Self {
            // ... existing fields ...
            #[cfg(target_os = "macos")]
            hotkey_manager,
        }
    }
}

// Update main function
fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let config_dir = dirs::home_dir()
                .unwrap()
                .join(".snaplingo");
            std::fs::create_dir_all(&config_dir).unwrap();
            let config_path = config_dir.join("config.json");
            
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

- [ ] **Step 5: Verify compilation**

```bash
cd src-tauri
cargo build
```

Expected: SUCCESS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/hotkeys/ src-tauri/src/main.rs
git commit -m "feat: add hotkey registration infrastructure (macOS placeholder)"
```

---

## Summary and Next Steps

This plan covers the foundation of SnapLingo:

**Completed (Tasks 1-10):**
- ✅ Project scaffold with Tauri 2.0 + React
- ✅ Configuration module with persistence
- ✅ Provider trait system
- ✅ Google Translate implementation
- ✅ Language detection
- ✅ Tauri command handlers
- ✅ Frontend state management
- ✅ Result Window UI
- ✅ Input translation mode
- ✅ Hotkey infrastructure

**Remaining for MVP (P0 + P1):**
- Screenshot capture (platform-specific)
- Screenshot editor with Canvas tools
- OCR providers (Tesseract, cloud options)
- Additional translation providers (DeepL, OpenAI)
- Selection translation with clipboard automation
- System tray integration
- Settings window
- History module with SQLite
- TTS integration
- Credential storage (Keychain/Credential Manager)

**Execution Options:**

1. **Subagent-Driven (recommended)** - Use superpowers:subagent-driven-development for task-by-task execution with fresh subagent per task

2. **Inline Execution** - Use superpowers:executing-plans for batch execution in this session

Which approach would you prefer?

