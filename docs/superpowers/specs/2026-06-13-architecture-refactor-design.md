# SnapLingo 架构重构设计文档

> 日期：2026-06-13  
> 类型：全面重构  
> 策略：Infrastructure-First（基础设施优先）

## 1. 重构目标

将现有扁平模块结构重构为分层架构 + 垂直切片模式，符合 ARCHITECTURE.md 和 ADR 0003 定义的目标架构。

### 1.1 核心目标

1. **清晰的依赖方向**：Commands → Application → Domain → Infrastructure（单向依赖）
2. **高内聚低耦合**：Provider 按类型垂直切片（ocr/translation/tts 各自独立）
3. **平台无关性**：平台差异在 Infrastructure 层隔离，Application 层无感知
4. **可测试性**：依赖注入（HttpClient, KeychainBackend 等），可 mock
5. **可扩展性**：添加新 Provider 只需实现 Trait + 注册

### 1.2 重构范围

**包含：**
- ✅ 完整的四层架构（Commands/Application/Domain/Infrastructure）
- ✅ Translation Provider 垂直切片（迁移现有 Google Translate）
- ✅ OCR Provider 垂直切片（新增）
- ✅ 截图服务（Screenshot Mode 框选、编辑、保存、贴图）
- ✅ 平台适配（Keychain、快捷键、截图、TTS）
- ✅ 配置管理（ConfigFile）和历史记录（HistoryDb）

**不包含（后续迭代）：**
- ❌ 前端 UI 重构（保持现有 React 组件）
- ❌ 系统托盘（暂时保留 TODO）
- ❌ 自定义 Provider（OpenAI/Claude/Gemini 兼容，后续添加）

---

## 2. 架构设计

### 2.1 目录结构

```
src-tauri/src/
├── commands/              # Commands Layer（Tauri 接口层）
│   ├── mod.rs
│   ├── config_commands.rs
│   ├── provider_commands.rs
│   ├── translation_commands.rs
│   ├── ocr_commands.rs
│   └── capture_commands.rs
│
├── application/           # Application Layer（应用层）
│   ├── providers/
│   │   ├── common/
│   │   │   ├── mod.rs
│   │   │   └── provider.rs        # Provider trait
│   │   ├── translation/
│   │   │   ├── mod.rs
│   │   │   ├── trait.rs           # TranslationProvider trait
│   │   │   ├── registry.rs        # TranslationRegistry（多选）
│   │   │   ├── service.rs         # TranslationService（业务编排）
│   │   │   └── impls/
│   │   │       ├── mod.rs
│   │   │       ├── google.rs      # 迁移现有实现
│   │   │       ├── deepl.rs
│   │   │       └── baidu.rs
│   │   └── ocr/
│   │       ├── mod.rs
│   │       ├── trait.rs           # OcrProvider trait
│   │       ├── registry.rs        # OcrRegistry（单选）
│   │       ├── service.rs         # OcrService
│   │       └── impls/
│   │           ├── mod.rs
│   │           ├── tesseract.rs   # 本地
│   │           └── baidu_ocr.rs   # 远程
│   │
│   └── services/
│       ├── mod.rs
│       ├── capture_service.rs     # 截图服务
│       └── hotkey_service.rs      # 快捷键服务
│
├── domain/               # Domain Layer（领域层）
│   ├── mod.rs
│   ├── translation.rs   # TranslationRequest, TranslationResult
│   ├── ocr.rs          # OcrRequest, OcrResult
│   ├── capture.rs      # CaptureMode, CaptureConfig, EditTool
│   └── config.rs       # AppConfig, ProviderConfig
│
├── infrastructure/       # Infrastructure Layer（基础设施层）
│   ├── storage/
│   │   ├── mod.rs
│   │   ├── config_file.rs         # JSON 配置读写
│   │   ├── history_db.rs          # SQLite 历史记录
│   │   └── keychain/
│   │       ├── mod.rs
│   │       ├── backend.rs         # KeychainBackend trait
│   │       ├── macos.rs
│   │       ├── windows.rs
│   │       └── linux.rs
│   │
│   ├── http/
│   │   ├── mod.rs
│   │   ├── client.rs              # HttpClient trait
│   │   └── reqwest_impl.rs        # Reqwest 实现
│   │
│   └── system/
│       ├── mod.rs
│       ├── paths.rs               # 平台路径（~/Library/ vs %APPDATA%）
│       ├── screenshot/
│       │   ├── mod.rs
│       │   ├── backend.rs         # ScreenshotBackend trait
│       │   ├── macos.rs           # core-graphics
│       │   ├── windows.rs         # Windows GDI
│       │   └── linux.rs           # xcb
│       └── hotkey/
│           ├── mod.rs
│           ├── backend.rs         # HotkeyBackend trait
│           ├── macos.rs
│           ├── windows.rs
│           └── linux.rs
│
├── error.rs             # 统一错误类型（thiserror）
├── lib.rs              # 模块声明
└── main.rs             # 入口 + 依赖注入（AppState）
```

### 2.2 分层职责

#### Commands Layer（接口层）
**职责：** Tauri 命令入口，纯编排逻辑

**特点：**
- 薄层，不包含业务逻辑
- 编排 Application + Infrastructure
- 错误转换（统一返回 `Result<T, String>`）

**示例：**
```rust
#[tauri::command]
pub async fn translate_text(
    text: String,
    source_lang: String,
    target_lang: String,
    state: State<'_, AppState>
) -> Result<Vec<TranslationResult>, String> {
    let request = TranslationRequest {
        text,
        source_lang: Some(source_lang),
        target_lang,
    };
    
    state.translation_service
        .translate(request)
        .await
        .map_err(|e| e.to_string())
}
```

#### Application Layer（应用层）
**职责：** 所有业务逻辑

**Provider 垂直切片：**
- **Trait**：定义 Provider 接口
- **Registry**：管理 Provider 列表和激活状态（单选 vs 多选）
- **Service**：业务编排（调用 Provider + 记录历史）
- **impls/**：具体实现（内置 + 自定义）

**其他服务：**
- `capture_service.rs`：截图业务（框选区域、保存路径、贴图管理）
- `hotkey_service.rs`：快捷键注册和冲突检测

#### Domain Layer（领域层）
**职责：** 纯数据结构，零依赖

**示例：**
```rust
// domain/translation.rs
pub struct TranslationRequest {
    pub text: String,
    pub source_lang: Option<String>,  // None = auto-detect
    pub target_lang: String,
}

pub struct TranslationResult {
    pub provider_id: String,
    pub text: String,
    pub detected_language: Option<String>,
}
```

**特点：**
- 只包含数据结构
- 可以有 `impl` 块（`default()`, `validate()`）
- 不依赖任何其他模块

#### Infrastructure Layer（基础设施层）
**职责：** 与外部系统交互的技术实现

**Storage（存储抽象）：**
- `ConfigFile`：JSON 配置文件读写（`~/.snaplingo/config.json`）
- `HistoryDb`：SQLite 历史记录（翻译/OCR）
- `Keychain`：系统钥匙串（平台适配）

**HTTP（网络客户端）：**
- `HttpClient` Trait：抽象 HTTP 调用
- `ReqwestHttpClient`：Reqwest 实现

**System（系统集成 + 平台适配）：**
- `paths.rs`：配置文件路径（平台差异）
- `screenshot/`：截图底层（调用系统 API）
- `hotkey/`：全局快捷键注册（平台差异）

---

## 3. 核心设计模式

### 3.1 策略模式（Strategy Pattern）

**Provider 是策略的实现：**
```rust
// 策略接口
trait TranslationProvider {
    async fn translate(&self, request: TranslationRequest) -> Result<TranslationResult>;
}

// 具体策略
struct GoogleTranslateProvider { ... }
struct DeepLProvider { ... }

// 上下文（运行时选择策略）
struct TranslationService {
    registry: Arc<Mutex<TranslationRegistry>>,
}
```

### 3.2 注册表模式（Registry Pattern）

**集中管理 Provider 实例和激活状态：**
```rust
struct TranslationRegistry {
    providers: HashMap<String, Arc<dyn TranslationProvider>>,
    active: Vec<String>,  // 多选
}

impl TranslationRegistry {
    pub fn register(&mut self, provider: Arc<dyn TranslationProvider>) { ... }
    pub fn activate(&mut self, id: &str) -> Result<()> { ... }
    pub fn get_active(&self) -> Vec<Arc<dyn TranslationProvider>> { ... }
}
```

**OCR Registry 特殊性：**
```rust
struct OcrRegistry {
    providers: HashMap<String, Arc<Mutex<dyn OcrProvider>>>,
    active: Option<String>,  // 单选
}
```

### 3.3 依赖注入（Dependency Injection）

**Provider 依赖抽象，不依赖具体实现：**
```rust
// ❌ 错误：直接依赖
struct DeepLProvider {
    client: reqwest::Client,  // 直接依赖 reqwest
}

// ✅ 正确：依赖抽象
struct DeepLProvider {
    http_client: Arc<dyn HttpClient>,  // 依赖抽象
    api_key: Option<String>,
}

// 在 main.rs 注入
let http_client = Arc::new(ReqwestHttpClient::new());
let provider = DeepLProvider::new(Arc::clone(&http_client));
```

### 3.4 平台适配模式

**Trait 抽象 + `#[cfg]` 条件编译：**
```rust
// infrastructure/storage/keychain/backend.rs

pub trait KeychainBackend {
    fn save(&self, service: &str, account: &str, password: &str) -> Result<()>;
    fn load(&self, service: &str, account: &str) -> Result<String>;
}

// infrastructure/storage/keychain/mod.rs

#[cfg(target_os = "macos")]
pub use macos::MacOSKeychain as PlatformKeychain;

#[cfg(target_os = "windows")]
pub use windows::WindowsKeychain as PlatformKeychain;

#[cfg(target_os = "linux")]
pub use linux::LinuxKeychain as PlatformKeychain;

pub struct Keychain {
    backend: PlatformKeychain,  // 编译时确定
}
```

**编译结果：**
- macOS 版本只包含 `MacOSKeychain`
- Windows 版本只包含 `WindowsKeychain`
- 应用层无感知平台差异

---

## 4. 数据流设计

### 4.1 翻译流程（多 Provider 并发）

```
用户输入文本 → 点击翻译
    ↓
React: invoke('translate_text', {text, source_lang, target_lang})
    ↓ Tauri IPC
Commands: translate_text(...)
    ↓
Application: TranslationService.translate(request)
    ↓ 获取激活的 Providers
    TranslationRegistry.get_active()
    → [Google, DeepL, 百度] (3 个激活)
    ↓ 并发调用 (tokio::join!)
    ┌─────────────┬─────────────┬─────────────┐
    ▼             ▼             ▼
GoogleProvider  DeepLProvider  BaiduProvider
    ↓ HTTP        ↓ HTTP        ↓ HTTP
Infrastructure: HttpClient (Reqwest)
    ↓             ↓             ↓
External APIs → Result 1, Result 2, Result 3
    │             │             │
    └─────────────┴─────────────┘
                  ↓ 收集
Application: 返回 Vec<TranslationResult>
    ↓ 异步记录历史
Infrastructure: HistoryDb.add_translation_entry(...)
    ↓
React: 显示 3 个翻译卡片
```

### 4.2 OCR 流程（单 Provider）

```
用户框选截图区域 → 完成框选
    ↓
React: invoke('ocr_recognize', {image_data})
    ↓
Commands: ocr_recognize(...)
    ↓
Application: OcrService.recognize(image)
    ↓
    OcrRegistry.get_active()
    → TesseractProvider (当前激活)
    ↓
TesseractProvider.recognize(image)
    ↓ 本地调用
Infrastructure: Tesseract CLI / Library
    ↓
返回 OcrResult {text, language}
    ↓ 记录历史
Infrastructure: HistoryDb.add_ocr_entry(...)
    ↓
React: 显示识别文本 + 翻译按钮
```

### 4.3 截图流程

```
用户按快捷键触发截图
    ↓
Infrastructure: HotkeyBackend 触发
    ↓
Commands: start_screenshot()
    ↓
Application: CaptureService.start_capture(CaptureMode::Screenshot)
    ↓
Infrastructure: ScreenshotBackend.capture_region()
    → macOS: core-graphics API
    → Windows: GDI API
    → Linux: xcb API
    ↓ 返回原始图片
React: 显示编辑器（全屏 Canvas）
    ↓ 用户编辑（标注、箭头、文字）
用户点击保存
    ↓
React: invoke('save_screenshot', {image_data, annotations})
    ↓
Commands: save_screenshot(...)
    ↓
Application: CaptureService.save(image, annotations)
    ↓ 合成标注层
    ↓ 生成文件名（时间戳）
Infrastructure: 保存到 ~/Pictures/SnapLingo/
```

### 4.4 Provider 配置流程

```
用户在设置界面输入 DeepL API Key → 点击保存
    ↓
React: invoke('configure_provider', {provider_id: 'deepl', api_key: 'xxx'})
    ↓
Commands: configure_provider(provider_id, api_key, State)
    ↓ 分两路
    │
    ├─→ Infrastructure: Keychain.save_provider_credential(provider_id, api_key)
    │       ↓ 平台适配
    │       ├─ macOS: Keychain API
    │       ├─ Windows: Credential Manager API
    │       └─ Linux: Secret Service API
    │
    └─→ Application: TranslationRegistry.configure_provider(provider_id, api_key)
            ↓
            DeepLProvider.set_api_key(api_key)
            ↓
            TranslationRegistry.activate(provider_id)
            ↓
    Infrastructure: ConfigFile.save('active_translation_providers', active_ids)
```

---

## 5. 详细设计

### 5.1 Infrastructure Layer

#### 5.1.1 ConfigFile（JSON 配置）

```rust
// infrastructure/storage/config_file.rs

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use serde_json::Value;
use anyhow::Result;

pub struct ConfigFile {
    path: PathBuf,
    lock: Arc<Mutex<()>>,  // 并发保护
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
        
        // 读取现有配置
        let mut config = self.load_all()?;
        
        // 更新指定 key
        config[key] = serde_json::to_value(value)?;
        
        // 写回文件
        let json = serde_json::to_string_pretty(&config)?;
        std::fs::write(&self.path, json)?;
        
        Ok(())
    }
    
    pub fn load<T: DeserializeOwned>(&self, key: &str) -> Result<T> {
        let config = self.load_all()?;
        let value = config.get(key)
            .ok_or_else(|| anyhow!("Key not found: {}", key))?;
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
```

**测试策略：**
- 单元测试：使用 `tempfile` 创建临时配置文件
- 测试并发读写（spawn 多线程）
- 测试错误处理（文件不存在、JSON 格式错误）

#### 5.1.2 Keychain（系统钥匙串）

```rust
// infrastructure/storage/keychain/backend.rs

use anyhow::Result;

pub trait KeychainBackend: Send + Sync {
    fn save(&self, service: &str, account: &str, password: &str) -> Result<()>;
    fn load(&self, service: &str, account: &str) -> Result<String>;
    fn delete(&self, service: &str, account: &str) -> Result<()>;
}

// infrastructure/storage/keychain/mod.rs

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
}

// infrastructure/storage/keychain/macos.rs

use keyring::Entry;
use anyhow::Result;

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

**实现注意事项：**
- 使用 `keyring` crate（跨平台封装）
- Windows/Linux 实现类似，只需替换底层 API
- 错误处理：密钥不存在、权限不足

#### 5.1.3 HttpClient（HTTP 抽象）

```rust
// infrastructure/http/client.rs

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

#[async_trait]
pub trait HttpClient: Send + Sync {
    async fn post(&self, url: &str, json: Value) -> Result<Response>;
    async fn get(&self, url: &str) -> Result<Response>;
}

pub struct Response {
    pub status: u16,
    pub body: String,
}

// infrastructure/http/reqwest_impl.rs

use super::client::{HttpClient, Response};
use anyhow::Result;
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
        let resp = self.client.get(url).send().await?;
        
        Ok(Response {
            status: resp.status().as_u16(),
            body: resp.text().await?,
        })
    }
}
```

**为什么要抽象？**
- ✅ 可测试（mock HttpClient）
- ✅ 可替换（换 HTTP 库不改 Provider）
- ✅ 可以添加通用逻辑（重试、超时、日志）

#### 5.1.4 ScreenshotBackend（截图平台适配）

```rust
// infrastructure/system/screenshot/backend.rs

use anyhow::Result;

pub trait ScreenshotBackend: Send + Sync {
    /// 捕获全屏
    fn capture_full_screen(&self) -> Result<Vec<u8>>;
    
    /// 捕获指定区域
    fn capture_region(&self, x: i32, y: i32, width: u32, height: u32) -> Result<Vec<u8>>;
}

// infrastructure/system/screenshot/macos.rs

use core_graphics::display::{CGDisplay, CGDisplayBounds};
use core_graphics::image::CGImageRef;
use anyhow::Result;

pub struct MacOSScreenshot;

impl MacOSScreenshot {
    pub fn new() -> Self {
        Self
    }
}

impl ScreenshotBackend for MacOSScreenshot {
    fn capture_full_screen(&self) -> Result<Vec<u8>> {
        // 使用 core-graphics API
        let display_id = CGDisplay::main().id;
        let bounds = CGDisplayBounds(display_id);
        let image: CGImageRef = CGDisplay::screenshot(
            bounds,
            kCGWindowListOptionOnScreenOnly,
            kCGNullWindowID,
            kCGWindowImageDefault,
        )?;
        
        // 转换为 PNG bytes
        let mut buffer = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut buffer);
        encoder.write_image(
            image.data(),
            image.width() as u32,
            image.height() as u32,
            image::ColorType::Rgba8,
        )?;
        
        Ok(buffer)
    }
    
    fn capture_region(&self, x: i32, y: i32, width: u32, height: u32) -> Result<Vec<u8>> {
        let display_id = CGDisplay::main().id;
        let rect = CGRect::new(
            CGPoint::new(x as f64, y as f64),
            CGSize::new(width as f64, height as f64),
        );
        let image: CGImageRef = CGDisplay::screenshot(
            rect,
            kCGWindowListOptionOnScreenOnly,
            kCGNullWindowID,
            kCGWindowImageDefault,
        )?;
        
        // 转换为 PNG bytes（同 capture_full_screen）
        let mut buffer = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut buffer);
        encoder.write_image(
            image.data(),
            image.width() as u32,
            image.height() as u32,
            image::ColorType::Rgba8,
        )?;
        
        Ok(buffer)
    }
}
```

**实现注意事项：**
- macOS: 使用 `core-graphics` crate
- Windows: 使用 `windows` crate（GDI API）
- Linux: 使用 `xcb` crate
- 返回 PNG/JPEG bytes，应用层不感知编码细节

---

### 5.2 Domain Layer

#### 5.2.1 Translation 领域模型

```rust
// domain/translation.rs

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

#### 5.2.2 OCR 领域模型

```rust
// domain/ocr.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrRequest {
    pub image: Vec<u8>,
    pub language_hint: Option<String>,  // 语言提示（如 "zh-CN", "en"）
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResult {
    pub provider_id: String,
    pub text: String,
    pub confidence: Option<f32>,  // 识别置信度 0.0-1.0
    pub language: Option<String>,  // 检测到的语言
}
```

#### 5.2.3 Capture 领域模型

```rust
// domain/capture.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CaptureMode {
    Screenshot,           // 截图模式
    Ocr,                 // OCR 模式
    OcrTranslation,      // OCR + 翻译模式
    SelectionTranslation, // 划词翻译模式
    InputTranslation,    // 输入翻译模式
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureConfig {
    pub save_path: String,
    pub format: ImageFormat,
    pub quality: u8,  // 1-100
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
```

---

### 5.3 Application Layer

#### 5.3.1 Provider 通用接口

```rust
// application/providers/common/provider.rs

pub trait Provider: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn is_configured(&self) -> bool;
    fn requires_api_key(&self) -> bool;
}
```

#### 5.3.2 Translation Provider

```rust
// application/providers/translation/trait.rs

use crate::domain::translation::{TranslationRequest, TranslationResult};
use crate::application::providers::common::Provider;
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait TranslationProvider: Provider {
    async fn translate(&self, request: &TranslationRequest) -> Result<TranslationResult>;
}

// application/providers/translation/registry.rs

use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;

pub struct TranslationRegistry {
    providers: HashMap<String, Arc<dyn TranslationProvider>>,
    active: Vec<String>,  // 多选
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
            return Err(anyhow!("Provider not found: {}", id));
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
}

// application/providers/translation/service.rs

use crate::domain::translation::{TranslationRequest, TranslationResult};
use crate::infrastructure::storage::HistoryDb;
use std::sync::{Arc, Mutex};
use anyhow::Result;

pub struct TranslationService {
    registry: Arc<Mutex<TranslationRegistry>>,
    history_db: Arc<HistoryDb>,
}

impl TranslationService {
    pub fn new(registry: Arc<Mutex<TranslationRegistry>>, history_db: Arc<HistoryDb>) -> Self {
        Self { registry, history_db }
    }
    
    pub async fn translate(&self, request: &TranslationRequest) -> Result<Vec<TranslationResult>> {
        // 1. 获取激活的 Providers
        let providers = self.registry.lock().unwrap().get_active();
        
        if providers.is_empty() {
            return Err(anyhow!("No translation provider activated"));
        }
        
        // 2. 并发调用所有激活的 Providers
        let mut tasks = Vec::new();
        for provider in providers {
            let request = request.clone();
            tasks.push(tokio::spawn(async move {
                provider.translate(&request).await
            }));
        }
        
        // 3. 收集结果
        let mut results = Vec::new();
        for task in tasks {
            if let Ok(Ok(result)) = task.await {
                results.push(result);
            }
        }
        
        // 4. 异步记录历史
        let history_db = Arc::clone(&self.history_db);
        let request_clone = request.clone();
        let results_clone = results.clone();
        tokio::spawn(async move {
            let _ = history_db.add_translation_entry(&request_clone, &results_clone).await;
        });
        
        Ok(results)
    }
}
```

#### 5.3.3 Google Translate Provider（迁移现有实现）

```rust
// application/providers/translation/impls/google.rs

use crate::application::providers::common::Provider;
use crate::application::providers::translation::TranslationProvider;
use crate::domain::translation::{TranslationRequest, TranslationResult};
use crate::infrastructure::http::HttpClient;
use std::sync::Arc;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

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
        true  // 免费 API，无需配置
    }
    
    fn requires_api_key(&self) -> bool {
        false
    }
}

#[async_trait]
impl TranslationProvider for GoogleTranslateProvider {
    async fn translate(&self, request: &TranslationRequest) -> Result<TranslationResult> {
        let url = format!(
            "https://translate.googleapis.com/translate_a/single?client=gtx&sl={}&tl={}&dt=t&q={}",
            request.source_lang.as_deref().unwrap_or("auto"),
            request.target_lang,
            urlencoding::encode(&request.text)
        );
        
        let response = self.http_client.get(&url).await?;
        
        // 解析 Google Translate API 响应
        let json: serde_json::Value = serde_json::from_str(&response.body)?;
        let translated_text = json[0][0][0]
            .as_str()
            .ok_or_else(|| anyhow!("Failed to parse translation result"))?
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
```

#### 5.3.4 OCR Provider

```rust
// application/providers/ocr/trait.rs

use crate::domain::ocr::{OcrRequest, OcrResult};
use crate::application::providers::common::Provider;
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait OcrProvider: Provider {
    async fn recognize(&self, request: &OcrRequest) -> Result<OcrResult>;
}

// application/providers/ocr/registry.rs

use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;

pub struct OcrRegistry {
    providers: HashMap<String, Arc<dyn OcrProvider>>,
    active: Option<String>,  // 单选
}

impl OcrRegistry {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            active: None,
        }
    }
    
    pub fn register(&mut self, provider: Arc<dyn OcrProvider>) {
        self.providers.insert(provider.id().to_string(), provider);
    }
    
    pub fn activate(&mut self, id: &str) -> Result<()> {
        if !self.providers.contains_key(id) {
            return Err(anyhow!("Provider not found: {}", id));
        }
        self.active = Some(id.to_string());
        Ok(())
    }
    
    pub fn get_active(&self) -> Option<Arc<dyn OcrProvider>> {
        self.active.as_ref()
            .and_then(|id| self.providers.get(id).cloned())
    }
    
    pub fn list_all(&self) -> Vec<Arc<dyn OcrProvider>> {
        self.providers.values().cloned().collect()
    }
}

// application/providers/ocr/service.rs

use crate::domain::ocr::{OcrRequest, OcrResult};
use crate::infrastructure::storage::HistoryDb;
use std::sync::{Arc, Mutex};
use anyhow::Result;

pub struct OcrService {
    registry: Arc<Mutex<OcrRegistry>>,
    history_db: Arc<HistoryDb>,
}

impl OcrService {
    pub fn new(registry: Arc<Mutex<OcrRegistry>>, history_db: Arc<HistoryDb>) -> Self {
        Self { registry, history_db }
    }
    
    pub async fn recognize(&self, request: &OcrRequest) -> Result<OcrResult> {
        // 1. 获取激活的 Provider
        let provider = self.registry.lock().unwrap().get_active()
            .ok_or_else(|| anyhow!("No OCR provider activated"))?;
        
        // 2. 调用 Provider
        let result = provider.recognize(request).await?;
        
        // 3. 异步记录历史
        let history_db = Arc::clone(&self.history_db);
        let result_clone = result.clone();
        tokio::spawn(async move {
            let _ = history_db.add_ocr_entry(&result_clone).await;
        });
        
        Ok(result)
    }
}
```

#### 5.3.5 Capture Service（截图服务）

```rust
// application/services/capture_service.rs

use crate::domain::capture::{CaptureMode, CaptureConfig, CaptureRegion};
use crate::infrastructure::system::screenshot::ScreenshotBackend;
use std::sync::Arc;
use anyhow::Result;

pub struct CaptureService {
    screenshot_backend: Arc<dyn ScreenshotBackend>,
    config: CaptureConfig,
}

impl CaptureService {
    pub fn new(screenshot_backend: Arc<dyn ScreenshotBackend>, config: CaptureConfig) -> Self {
        Self {
            screenshot_backend,
            config,
        }
    }
    
    pub fn capture_region(&self, region: &CaptureRegion) -> Result<Vec<u8>> {
        self.screenshot_backend.capture_region(
            region.x,
            region.y,
            region.width,
            region.height
        )
    }
    
    pub fn save(&self, image: Vec<u8>, filename: Option<String>) -> Result<String> {
        let filename = filename.unwrap_or_else(|| {
            // 生成时间戳文件名
            let now = chrono::Local::now();
            format!("screenshot_{}.png", now.format("%Y%m%d_%H%M%S"))
        });
        
        let path = std::path::Path::new(&self.config.save_path).join(filename);
        std::fs::write(&path, image)?;
        
        Ok(path.to_string_lossy().to_string())
    }
}
```

---

### 5.4 Commands Layer

```rust
// commands/translation_commands.rs

use crate::domain::translation::{TranslationRequest, TranslationResult};
use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn translate_text(
    text: String,
    source_lang: Option<String>,
    target_lang: String,
    state: State<'_, AppState>
) -> Result<Vec<TranslationResult>, String> {
    let request = TranslationRequest {
        text,
        source_lang,
        target_lang,
    };
    
    state.translation_service
        .translate(&request)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_translation_providers(
    state: State<'_, AppState>
) -> Result<Vec<ProviderInfo>, String> {
    let providers = state.translation_registry
        .lock()
        .unwrap()
        .list_all();
    
    let info: Vec<_> = providers.iter().map(|p| ProviderInfo {
        id: p.id().to_string(),
        name: p.name().to_string(),
        is_configured: p.is_configured(),
        requires_api_key: p.requires_api_key(),
    }).collect();
    
    Ok(info)
}

#[tauri::command]
pub async fn activate_translation_provider(
    provider_id: String,
    state: State<'_, AppState>
) -> Result<(), String> {
    state.translation_registry
        .lock()
        .unwrap()
        .activate(&provider_id)
        .map_err(|e| e.to_string())?;
    
    // 持久化
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

// commands/ocr_commands.rs

use crate::domain::ocr::{OcrRequest, OcrResult};
use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn ocr_recognize(
    image: Vec<u8>,
    language_hint: Option<String>,
    state: State<'_, AppState>
) -> Result<OcrResult, String> {
    let request = OcrRequest {
        image,
        language_hint,
    };
    
    state.ocr_service
        .recognize(&request)
        .await
        .map_err(|e| e.to_string())
}

// commands/capture_commands.rs

use crate::domain::capture::CaptureRegion;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn capture_screenshot(
    region: CaptureRegion,
    state: State<'_, AppState>
) -> Result<Vec<u8>, String> {
    state.capture_service
        .capture_region(&region)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_screenshot(
    image: Vec<u8>,
    filename: Option<String>,
    state: State<'_, AppState>
) -> Result<String, String> {
    state.capture_service
        .save(image, filename)
        .map_err(|e| e.to_string())
}
```

---

## 6. 依赖注入和初始化

### 6.1 AppState 设计

```rust
// main.rs

use std::sync::{Arc, Mutex};

pub struct AppState {
    // Infrastructure
    pub config_file: Arc<ConfigFile>,
    pub history_db: Arc<HistoryDb>,
    pub keychain: Arc<Keychain>,
    pub http_client: Arc<dyn HttpClient>,
    
    // Application - Providers
    pub translation_registry: Arc<Mutex<TranslationRegistry>>,
    pub translation_service: Arc<TranslationService>,
    pub ocr_registry: Arc<Mutex<OcrRegistry>>,
    pub ocr_service: Arc<OcrService>,
    
    // Application - Services
    pub capture_service: Arc<CaptureService>,
    pub hotkey_service: Arc<HotkeyService>,
}

impl AppState {
    pub fn new(app: tauri::AppHandle) -> Result<Self> {
        // 1. Infrastructure
        let config_path = get_config_path()?;
        let config_file = Arc::new(ConfigFile::new(config_path.clone()));
        
        let db_path = config_path.parent().unwrap().join("history.db");
        let history_db = Arc::new(HistoryDb::new(db_path)?);
        
        let keychain = Arc::new(Keychain::new());
        let http_client: Arc<dyn HttpClient> = Arc::new(ReqwestHttpClient::new());
        
        // 2. Translation Providers
        let mut translation_registry = TranslationRegistry::new();
        
        // 注册 Google Translate
        translation_registry.register(Arc::new(
            GoogleTranslateProvider::new(Arc::clone(&http_client))
        ));
        
        // 注册 DeepL
        let deepl_api_key = keychain.load_provider_credential("deepl").ok();
        translation_registry.register(Arc::new(
            DeepLProvider::new(Arc::clone(&http_client), deepl_api_key)
        ));
        
        // 恢复激活状态
        if let Ok(active_ids) = config_file.load::<Vec<String>>("active_translation_providers") {
            for id in active_ids {
                let _ = translation_registry.activate(&id);
            }
        }
        
        let translation_registry = Arc::new(Mutex::new(translation_registry));
        let translation_service = Arc::new(TranslationService::new(
            Arc::clone(&translation_registry),
            Arc::clone(&history_db),
        ));
        
        // 3. OCR Providers
        let mut ocr_registry = OcrRegistry::new();
        
        ocr_registry.register(Arc::new(TesseractProvider::new()));
        
        let baidu_api_key = keychain.load_provider_credential("baidu_ocr").ok();
        ocr_registry.register(Arc::new(
            BaiduOcrProvider::new(Arc::clone(&http_client), baidu_api_key)
        ));
        
        // 恢复激活状态
        if let Ok(active_id) = config_file.load::<String>("active_ocr_provider") {
            let _ = ocr_registry.activate(&active_id);
        }
        
        let ocr_registry = Arc::new(Mutex::new(ocr_registry));
        let ocr_service = Arc::new(OcrService::new(
            Arc::clone(&ocr_registry),
            Arc::clone(&history_db),
        ));
        
        // 4. Capture Service
        let screenshot_backend: Arc<dyn ScreenshotBackend> = Arc::new(PlatformScreenshot::new());
        let capture_config = config_file.load("capture_config")
            .unwrap_or_default();
        let capture_service = Arc::new(CaptureService::new(
            screenshot_backend,
            capture_config,
        ));
        
        // 5. Hotkey Service
        let hotkey_service = Arc::new(HotkeyService::new(app));
        
        Ok(Self {
            config_file,
            history_db,
            keychain,
            http_client,
            translation_registry,
            translation_service,
            ocr_registry,
            ocr_service,
            capture_service,
            hotkey_service,
        })
    }
}
```

---

## 7. 测试策略

### 7.1 单元测试

#### Infrastructure Layer
```rust
// infrastructure/storage/config_file.rs 测试

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_save_and_load() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.json");
        let config_file = ConfigFile::new(config_path);
        
        // 保存
        config_file.save("test_key", &"test_value").unwrap();
        
        // 加载
        let value: String = config_file.load("test_key").unwrap();
        assert_eq!(value, "test_value");
    }
    
    #[test]
    fn test_concurrent_writes() {
        // 测试并发写入不会导致数据损坏
        // ...
    }
}

// infrastructure/http/reqwest_impl.rs 测试

#[cfg(test)]
mod tests {
    use super::*;
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
    }
}
```

#### Application Layer
```rust
// application/providers/translation/service.rs 测试

#[cfg(test)]
mod tests {
    use super::*;
    
    // Mock Provider
    struct MockTranslationProvider {
        id: String,
        result: TranslationResult,
    }
    
    #[async_trait]
    impl TranslationProvider for MockTranslationProvider {
        async fn translate(&self, _request: &TranslationRequest) -> Result<TranslationResult> {
            Ok(self.result.clone())
        }
    }
    
    impl Provider for MockTranslationProvider {
        fn id(&self) -> &str { &self.id }
        fn name(&self) -> &str { &self.id }
        fn is_configured(&self) -> bool { true }
        fn requires_api_key(&self) -> bool { false }
    }
    
    #[tokio::test]
    async fn test_translate_with_multiple_providers() {
        // 创建 mock registry
        let mut registry = TranslationRegistry::new();
        registry.register(Arc::new(MockTranslationProvider {
            id: "provider1".to_string(),
            result: TranslationResult {
                provider_id: "provider1".to_string(),
                provider_name: "Provider 1".to_string(),
                text: "result1".to_string(),
                detected_language: None,
            },
        }));
        registry.activate("provider1").unwrap();
        
        // 创建 mock history_db（空实现，测试时忽略历史记录）
        let mock_history_db = Arc::new(MockHistoryDb::new());
        
        // 测试
        let service = TranslationService::new(
            Arc::new(Mutex::new(registry)),
            Arc::new(mock_history_db),
        );
        
        let request = TranslationRequest::new("hello".to_string(), "zh".to_string());
        let results = service.translate(&request).await.unwrap();
        
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].text, "result1");
    }
}
```

### 7.2 集成测试

```rust
// tests/integration_test.rs

#[tokio::test]
async fn test_full_translation_flow() {
    // 1. 初始化 AppState
    let app_state = AppState::new_for_test().unwrap();
    
    // 2. 激活 Google Translate
    app_state.translation_registry
        .lock().unwrap()
        .activate("google-translate").unwrap();
    
    // 3. 调用翻译
    let request = TranslationRequest::new("hello".to_string(), "zh".to_string());
    let results = app_state.translation_service.translate(&request).await.unwrap();
    
    // 4. 验证结果
    assert!(!results.is_empty());
    assert_eq!(results[0].provider_id, "google-translate");
    
    // 5. 验证历史记录
    // ...
}
```

### 7.3 端到端测试

使用 Tauri 的测试工具测试前后端集成：

```rust
// tests/e2e_test.rs

#[test]
fn test_translate_text_command() {
    tauri::test::mock_builder()
        .invoke_handler(tauri::generate_handler![
            commands::translate_text,
        ])
        .build(tauri::generate_context!())
        .unwrap()
        .test(|app| {
            let window = app.get_window("main").unwrap();
            
            // 调用命令
            let result: Vec<TranslationResult> = tauri::test::sync_invoke(
                &window,
                "translate_text",
                json!({
                    "text": "hello",
                    "sourceLang": null,
                    "targetLang": "zh"
                })
            );
            
            // 验证
            assert!(!result.is_empty());
        });
}
```

---

## 8. 迁移策略

### 8.1 Phase 1: Infrastructure Layer（2-3 天）

**目标：** 构建完整的基础设施层

**任务清单：**
1. ✅ 创建目录结构 `infrastructure/`
2. ✅ 实现 `error.rs`（统一错误类型，thiserror）
3. ✅ 实现 `infrastructure/storage/config_file.rs` + 单元测试
4. ✅ 实现 `infrastructure/storage/keychain/` (3 平台) + 单元测试
5. ✅ 实现 `infrastructure/http/client.rs` + `reqwest_impl.rs` + 单元测试
6. ✅ 实现 `infrastructure/system/paths.rs`
7. ✅ 实现 `infrastructure/system/screenshot/` (3 平台) + 单元测试
8. ✅ 实现 `infrastructure/system/hotkey/` (3 平台)
9. ✅ 创建 `domain/` 目录和基础数据结构

**验收标准：**
- 所有 Infrastructure 模块有单元测试
- 测试覆盖率 > 80%
- 在 macOS/Windows/Linux 上编译通过

**注意事项：**
- 此阶段应用不可运行（正常，预期行为）
- 专注于基础设施质量，不着急恢复应用功能
- 每个模块完成后立即写测试验证

---

### 8.2 Phase 2: Translation Provider（2-3 天）

**目标：** 迁移现有 Translation 功能到新架构，恢复应用可运行状态

**任务清单：**
1. ✅ 创建 `application/providers/common/provider.rs`
2. ✅ 创建 `application/providers/translation/` 目录结构
3. ✅ 实现 `TranslationProvider` Trait
4. ✅ 实现 `TranslationRegistry`（多选）+ 单元测试
5. ✅ 实现 `TranslationService` + 单元测试
6. ✅ 迁移 `GoogleTranslateProvider` 到 `impls/google.rs`
7. ✅ 实现 `DeepLProvider` (新增)
8. ✅ 实现 `BaiduTranslationProvider` (新增)
9. ✅ 更新 `commands/translation_commands.rs`
10. ✅ 更新 `main.rs` 的 AppState 初始化
11. ✅ 删除旧的 `translate/` 模块
12. ✅ 集成测试

**验收标准：**
- 前端翻译功能恢复正常
- 可以激活/停用多个 Translation Provider
- 翻译结果并发返回
- 历史记录正常保存

**注意事项：**
- 先迁移 Google Translate（已有实现），快速恢复功能
- 再添加 DeepL 和百度翻译
- 保持前端代码不变（只改后端）

---

### 8.3 Phase 3: OCR Provider（3-4 天）

**目标：** 添加 OCR 功能（新增）

**任务清单：**
1. ✅ 创建 `application/providers/ocr/` 目录结构
2. ✅ 实现 `OcrProvider` Trait
3. ✅ 实现 `OcrRegistry`（单选）+ 单元测试
4. ✅ 实现 `OcrService` + 单元测试
5. ✅ 实现 `TesseractProvider`（本地）
6. ✅ 实现 `BaiduOcrProvider`（远程）
7. ✅ 实现 `commands/ocr_commands.rs`
8. ✅ 更新 `main.rs` 的 AppState
9. ✅ 前端：OCR 结果窗口
10. ✅ 集成测试

**验收标准：**
- OCR Mode 可以识别截图中的文字
- 支持切换 Tesseract / 百度 OCR
- OCR 后可以点击"翻译"按钮
- 历史记录正常保存

---

### 8.4 Phase 4: Capture Service（2-3 天）

**目标：** 实现截图功能（Screenshot Mode）

**任务清单：**
1. ✅ 完善 `domain/capture.rs`
2. ✅ 实现 `application/services/capture_service.rs`
3. ✅ 实现 `commands/capture_commands.rs`
4. ✅ 更新 `main.rs` 的 AppState
5. ✅ 前端：截图编辑器（Canvas）
6. ✅ 前端：贴图窗口
7. ✅ 集成快捷键触发
8. ✅ 集成测试

**验收标准：**
- 按 F1 触发截图框选
- 编辑工具正常（矩形、箭头、文字等）
- 保存到配置的路径
- 贴图功能正常

---

### 8.5 Phase 5: HistoryDb + 收尾（2 天）

**目标：** 完善历史记录和剩余功能

**任务清单：**
1. ✅ 实现 `infrastructure/storage/history_db.rs`
2. ✅ 集成到 TranslationService 和 OcrService
3. ✅ 前端：历史记录界面
4. ✅ 实现历史记录清理（自动 + 手动）
5. ✅ 完善错误处理和日志
6. ✅ 跨平台测试（macOS/Windows/Linux）
7. ✅ 性能测试和优化
8. ✅ 文档更新

**验收标准：**
- 翻译/OCR 历史正常记录和查询
- 历史记录可以搜索、过滤、删除
- 自动清理机制正常工作
- 在三个平台上功能正常

---

## 9. 风险和缓解

### 9.1 平台适配风险

**风险：** macOS/Windows/Linux 的系统 API 差异导致功能不一致

**缓解：**
- 优先在 macOS 上实现和测试（开发环境）
- 使用成熟的跨平台 crate（`keyring`, `core-graphics`, `windows`, `xcb`）
- 为每个平台适配层编写单元测试
- Phase 5 专门做跨平台测试

### 9.2 性能风险

**风险：** Trait Object (`Arc<dyn Provider>`) 有虚函数开销

**缓解：**
- OCR/翻译本身是重操作（网络/计算密集），虚函数开销可忽略
- Phase 5 进行性能测试，验证性能符合预期
- 如果发现瓶颈，可以针对性优化（如使用泛型替代 Trait Object）

### 9.3 测试覆盖风险

**风险：** Infrastructure 层难以测试（依赖系统 API）

**缓解：**
- 抽象为 Trait（`KeychainBackend`, `ScreenshotBackend`），可 mock
- 单元测试覆盖业务逻辑（Registry, Service）
- 集成测试覆盖完整流程
- 端到端测试验证用户场景

### 9.4 迁移风险

**风险：** Phase 1 期间应用不可运行，可能遗漏需求

**缓解：**
- Phase 1 每完成一个模块立即写测试验证
- Phase 2 优先恢复核心功能（Google Translate）
- 保持频繁提交，方便回滚
- 与用户保持沟通，确认功能符合预期

---

## 10. 成功标准

### 10.1 架构质量

- ✅ 依赖方向单向（Commands → Application → Domain, Infrastructure）
- ✅ Provider 按类型垂直切片（ocr/translation/tts 各自独立）
- ✅ 平台差异在 Infrastructure 隔离
- ✅ 依赖注入实现（HttpClient, KeychainBackend 等）
- ✅ 测试覆盖率 > 80%

### 10.2 功能完整性

- ✅ Translation：支持 Google/DeepL/百度，可多选，并发调用
- ✅ OCR：支持 Tesseract/百度 OCR，单选
- ✅ Screenshot：框选、编辑、保存、贴图
- ✅ 历史记录：翻译/OCR 历史可查询、搜索、删除
- ✅ 配置管理：API Key 保存到系统钥匙串

### 10.3 跨平台支持

- ✅ macOS/Windows/Linux 编译通过
- ✅ 核心功能在三个平台上正常工作
- ✅ 平台适配层（Keychain, Screenshot, Hotkey）正常

### 10.4 性能

- ✅ 翻译响应时间 < 2 秒（网络正常）
- ✅ OCR 识别时间 < 3 秒（Tesseract 本地）
- ✅ 截图触发延迟 < 100ms
- ✅ 历史记录查询 < 100ms

---

## 11. 后续迭代

重构完成后的后续工作（不在本次重构范围）：

1. **TTS Provider**
   - 实现 TtsProvider Trait + Registry（单选）
   - 平台适配（macOS `say`, Windows SAPI, Linux espeak）
   - 集成到 ResultWindow（朗读按钮）

2. **自定义 Translation Provider**
   - OpenAI 兼容（支持 Ollama 等本地模型）
   - Claude 兼容（Anthropic API）
   - Gemini 兼容（Google AI Studio）

3. **更多内置 Provider**
   - OCR: PaddleOCR, 腾讯 OCR, Google Cloud Vision, Azure
   - Translation: 有道翻译, 腾讯翻译君, Azure Translator

4. **系统托盘和全局快捷键**
   - 托盘菜单（截图/翻译/历史记录）
   - 完善快捷键冲突检测

5. **前端 UI 优化**
   - Settings Window（21 个页面）
   - 历史记录界面优化
   - 收藏夹功能

---

## 12. 总结

这次重构的核心价值：

1. **清晰的架构**：四层分层 + 垂直切片，职责明确，易导航
2. **高内聚低耦合**：Provider 独立演进，修改 OCR 不影响 Translation
3. **平台无关性**：应用层无需关心平台差异
4. **可测试性**：依赖注入，可 mock，高测试覆盖率
5. **可扩展性**：添加新 Provider 只需实现 Trait + 注册

**预期收益：**
- 开发效率提升：添加新 Provider 从 2 天降到 0.5 天
- 代码质量提升：测试覆盖率从 0% 提升到 80%+
- 维护成本降低：清晰的架构减少理解成本
- 平台支持增强：统一的平台适配层，易于添加新平台

**工作量估算：**
- Phase 1 (Infrastructure): 2-3 天
- Phase 2 (Translation): 2-3 天
- Phase 3 (OCR): 3-4 天
- Phase 4 (Capture): 2-3 天
- Phase 5 (收尾): 2 天

**总计：11-15 天（2-3 周）**

