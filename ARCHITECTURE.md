# SnapLingo 架构设计文档

> 最终版本 - 2026-06-13

## 📐 架构概览

SnapLingo 采用**分层架构 + 垂直切片**设计模式：

```
Commands Layer (接口层)
    ↓
Application Layer (应用层)
    ├─ Providers (垂直切片：OCR/Translation/TTS)
    └─ Services (其他业务服务)
    ↓
Domain Layer (领域层)
    ↓
Infrastructure Layer (基础设施层)
    ├─ Storage (存储抽象)
    ├─ System (系统集成 + 平台适配)
    └─ HTTP (网络客户端)
```

**核心原则：**
1. **依赖方向单向向内**（外层依赖内层，内层不知道外层）
2. **垂直切片按业务聚合**（OCR/Translation/TTS 各自独立）
3. **平台差异在 Infrastructure 隔离**（应用层无感知）
4. **依赖注入实现可测试性**（HttpClient, TtsBackend 等）

---

## 📁 目录结构

```
snaplingo/
├─ src-tauri/src/
│   ├─ main.rs                          # 入口 + 依赖注入
│   ├─ error.rs                         # 统一错误类型
│   │
│   ├─ commands/                        # ⭐ Commands Layer
│   │   ├─ config_commands.rs
│   │   ├─ provider_commands.rs
│   │   ├─ ocr_commands.rs
│   │   ├─ translation_commands.rs
│   │   └─ ...
│   │
│   ├─ application/                     # ⭐ Application Layer
│   │   ├─ providers/                   # Provider 垂直切片
│   │   │   ├─ common/                  # 共享基础
│   │   │   │   ├─ provider.rs          # Provider Trait
│   │   │   │   └─ registry.rs          # Registry 共享逻辑
│   │   │   │
│   │   │   ├─ ocr/                     # OCR Providers
│   │   │   │   ├─ trait.rs
│   │   │   │   ├─ registry.rs          # 单选
│   │   │   │   ├─ service.rs
│   │   │   │   └─ impls/               # 6 个实现
│   │   │   │
│   │   │   ├─ translation/             # Translation Providers
│   │   │   │   ├─ trait.rs
│   │   │   │   ├─ registry.rs          # 多选
│   │   │   │   ├─ service.rs
│   │   │   │   └─ impls/               # 9 个实现
│   │   │   │
│   │   │   └─ tts/                     # TTS Providers
│   │   │       └─ ...
│   │   │
│   │   └─ services/                    # 其他应用服务
│   │       ├─ hotkey_conflict.rs
│   │       └─ history_cleanup.rs
│   │
│   ├─ domain/                          # ⭐ Domain Layer
│   │   ├─ capture.rs                   # 纯数据结构
│   │   ├─ translation.rs
│   │   └─ ocr.rs
│   │
│   └─ infrastructure/                  # ⭐ Infrastructure Layer
│       ├─ storage/
│       │   ├─ config_file.rs
│       │   ├─ history_db.rs
│       │   └─ keychain/                # 平台适配 ⭐
│       │       ├─ mod.rs
│       │       ├─ macos.rs
│       │       ├─ windows.rs
│       │       └─ linux.rs
│       │
│       ├─ system/
│       │   ├─ paths.rs                 # 平台适配 ⭐
│       │   ├─ hotkey/                  # 平台适配 ⭐
│       │   ├─ tts/                     # 平台适配 ⭐
│       │   └─ ...
│       │
│       └─ http/
│           ├─ client.rs                # HttpClient Trait
│           └─ reqwest_impl.rs
│
└─ src/                                 # React Frontend
    ├─ components/
    │   ├─ SettingsWindow/              # 设置窗口（21 个页面）
    │   └─ ResultWindow/                # 结果窗口
    └─ stores/                          # Zustand 状态管理
```

---

## 🏗️ 分层架构详解

### 1. Commands Layer（接口层）

**职责：** Tauri 命令入口，纯编排逻辑

**特点：**
- ✅ 薄层，不包含业务逻辑
- ✅ 编排 Application + Infrastructure
- ✅ 错误转换（统一返回 `Result`）

**示例：**
```rust
#[tauri::command]
pub async fn activate_ocr_provider(
    provider_id: String,
    state: State<'_, AppState>
) -> Result<()> {
    // 1. 激活
    state.ocr_registry.lock().unwrap()
        .activate(&provider_id)?;
    
    // 2. 持久化
    state.config_file.save("active_ocr", &provider_id)?;
    
    Ok(())
}
```

---

### 2. Application Layer（应用层）

**职责：** 所有业务逻辑

#### 2.1 Providers（垂直切片）

**设计模式：** 每个 Provider 类型独立目录，包含完整子系统

**OCR Provider 结构：**
```
providers/ocr/
├─ trait.rs         # OcrProvider Trait 定义
├─ registry.rs      # OcrRegistry（管理激活状态，单选）
├─ service.rs       # OcrService（业务编排）
└─ impls/           # 具体实现
    ├─ tesseract.rs      # 本地
    ├─ paddleocr.rs      # 本地
    ├─ baidu_ocr.rs      # 远程（依赖 HttpClient）
    └─ ...
```

**职责划分：**

| 模块 | 职责 | 不负责 |
|------|------|--------|
| **Trait** | 定义 Provider 接口 | 不管理实例、不执行业务 |
| **Registry** | 管理 Provider 列表、激活状态 | 不执行 Provider、不记录历史 |
| **Service** | 调用 Provider + 记录历史 | 不管理激活状态 |
| **impls/** | 实现具体能力（OCR 识别） | 不管理自己的激活状态 |

**Translation Provider 特殊性：**
- Registry 是**多选**（可同时激活多个）
- Service **并发调用**所有激活的 Provider
- 返回 `Vec<TranslationResult>` 供用户对比

---

#### 2.2 Services（其他应用服务）

**示例：**
- `hotkey_conflict.rs`：快捷键冲突检测（跨 Provider 类型）
- `history_cleanup.rs`：历史记录自动清理

---

### 3. Domain Layer（领域层）

**职责：** 纯数据结构，零依赖

**示例：**
```rust
// domain/capture.rs
pub struct CaptureConfig {
    pub hotkeys: CaptureHotkeys,
    pub save_path: String,
    pub format: ImageFormat,
    pub quality: u8,
}
```

**特点：**
- ✅ 只包含数据结构
- ✅ 可以有 `impl` 块（`default()`, `validate()`）
- ❌ 不依赖任何其他模块
- ❌ 不包含业务逻辑

---

### 4. Infrastructure Layer（基础设施层）

**职责：** 与外部系统交互的技术实现

#### 4.1 Storage（存储抽象）

**ConfigFile：** JSON 配置文件读写
```rust
pub struct ConfigFile {
    path: PathBuf,
    lock: Arc<Mutex<()>>,  // 并发保护
}

impl ConfigFile {
    pub fn save<T: Serialize>(&self, key: &str, value: &T) -> Result<()>;
    pub fn load<T: DeserializeOwned>(&self, key: &str) -> Result<T>;
}
```

**HistoryDb：** SQLite 历史记录
```rust
pub struct HistoryDb {
    connection: SqliteConnection,
}

impl HistoryDb {
    pub fn add_translation_entry(&self, entry: TranslationHistoryEntry) -> Result<()>;
    pub fn query_translation_history(&self, filter: HistoryFilter) -> Result<Vec<...>>;
}
```

**Keychain：** 系统钥匙串（平台适配）⭐

---

#### 4.2 System（系统集成 + 平台适配）

**平台适配模式：** Trait 抽象 + `#[cfg]` 条件编译

**示例：Keychain**
```rust
// infrastructure/storage/keychain/mod.rs

pub trait KeychainBackend {
    fn save(&self, service: &str, account: &str, password: &str) -> Result<()>;
    fn load(&self, service: &str, account: &str) -> Result<String>;
}

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

**其他平台适配：**
- `paths.rs`：配置文件路径（`~/Library/` vs `%APPDATA%` vs `~/.config/`）
- `hotkey/`：全局快捷键注册（使用 global-hotkey crate）
- `tts/`：TTS 后端（macOS `say` vs Windows SAPI vs Linux espeak）

---

#### 4.3 HTTP（网络客户端）

**HttpClient Trait：** 抽象 HTTP 调用
```rust
pub trait HttpClient: Send + Sync {
    async fn post(&self, url: &str, json: Value) -> Result<Response>;
    async fn get(&self, url: &str) -> Result<Response>;
}

// Reqwest 实现
pub struct ReqwestHttpClient {
    client: reqwest::Client,
}
```

**为什么要抽象？**
- ✅ 可测试（mock HttpClient）
- ✅ 可替换（换 HTTP 库不改 Provider）
- ✅ 依赖注入（Provider 依赖抽象，不依赖具体实现）

---

## 🔄 数据流和交互

### 场景 1：用户激活 OCR Provider

```
用户点击"激活百度 OCR"
    ↓
React: invoke('activate_ocr_provider', {provider_id: 'baidu_ocr'})
    ↓ Tauri IPC
Commands: activate_ocr_provider(provider_id, State)
    ↓
Application: OcrRegistry.activate(provider_id)
    ↓ 持久化
Infrastructure: ConfigFile.save('active_ocr', provider_id)
    ↓
File System: ~/.snaplingo/config.json
```

---

### 场景 2：用户配置 Provider API Key

```
用户输入 API Key → 点击保存
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
            DeepLProvider.api_key = Some(api_key)
```

---

### 场景 3：用户执行翻译（多 Provider 并发）

```
用户输入文本 → 点击翻译
    ↓
React: invoke('translate_text', {text: 'hello', source: 'en', target: 'zh'})
    ↓
Commands: translate_text(text, source, target, State)
    ↓
Application: TranslationService.translate(request)
    ↓ 获取激活的 Providers
    TranslationRegistry.get_active_providers()
    → [DeepL, Google, 百度] (3 个激活)
    ↓ 并发调用 (tokio::join!)
    ┌─────────────┬─────────────┬─────────────┐
    ▼             ▼             ▼
DeepLProvider  GoogleProvider  BaiduProvider
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

---

## 🧩 核心设计模式

### 1. 策略模式（Strategy Pattern）

**Provider 是策略的实现：**
```rust
// 策略接口
trait OcrProvider {
    async fn recognize(&self, image: &[u8]) -> Result<OcrResult>;
}

// 具体策略
struct TesseractProvider { ... }
struct BaiduOcrProvider { ... }

// 上下文（运行时选择策略）
struct OcrService {
    registry: Arc<Mutex<OcrRegistry>>,  // 管理所有策略
}
```

---

### 2. 依赖注入（Dependency Injection）

**Provider 依赖抽象，不依赖具体实现：**
```rust
// ❌ 错误：直接依赖
struct BaiduOcrProvider {
    client: reqwest::Client,  // 直接依赖 reqwest
}

// ✅ 正确：依赖抽象
struct BaiduOcrProvider {
    http_client: Arc<dyn HttpClient>,  // 依赖抽象
}

// 在 main.rs 注入
let http_client = Arc::new(ReqwestHttpClient::new());
let provider = BaiduOcrProvider::new(Arc::clone(&http_client));
```

---

### 3. 注册表模式（Registry Pattern）

**集中管理 Provider 实例和激活状态：**
```rust
struct OcrRegistry {
    providers: HashMap<String, Arc<Mutex<dyn OcrProvider>>>,
    active: Option<String>,  // 单选
}

impl OcrRegistry {
    pub fn register(&mut self, provider: Arc<Mutex<dyn OcrProvider>>) { ... }
    pub fn activate(&mut self, id: &str) -> Result<()> { ... }
    pub fn get_active(&self) -> Option<Arc<Mutex<dyn OcrProvider>>> { ... }
}
```

---

### 4. 门面模式（Facade Pattern）

**Service 简化复杂子系统的使用：**
```rust
// 对外暴露简单接口
struct OcrService {
    registry: Arc<Mutex<OcrRegistry>>,
    history_db: Arc<HistoryDb>,
}

impl OcrService {
    pub async fn recognize(&self, image: &[u8]) -> Result<OcrResult> {
        // 隐藏复杂的 Registry 操作和历史记录逻辑
    }
}
```

---

## ⚖️ 架构权衡

### 权衡 1：垂直切片 vs 水平分层

| 方案 | 优点 | 缺点 |
|------|------|------|
| **垂直切片**（采用） | 功能内聚、易导航、独立演进 | 有重复代码（Registry 模式） |
| **水平分层** | 无重复 | Trait 和 impl 分离、跨目录查找 |

**决策：** 垂直切片，通过 `common/` 减少重复

---

### 权衡 2：Trait Object vs 泛型

| 方案 | 优点 | 缺点 |
|------|------|------|
| **Trait Object**（采用） | 可以在运行时选择实现 | 有虚函数调用开销 |
| **泛型** | 零成本抽象 | 编译时确定，无法运行时切换 |

**决策：** Trait Object，因为需要运行时切换 Provider

**性能评估：** OCR/翻译本身是重操作（网络/计算密集），虚函数开销可忽略

---

### 权衡 3：三个 Registry vs 泛型 Registry

| 方案 | 优点 | 缺点 |
|------|------|------|
| **三个独立**（采用） | 类型安全、激活逻辑独立 | 代码重复 |
| **泛型 Registry** | 无重复 | Rust 泛型 + dyn Trait 复杂 |

**决策：** 独立 Registry，通过 `common/registry.rs` 共享基础逻辑

---

## 📊 架构质量指标

### 可测试性 ✅

| 模块 | 如何测试 |
|------|---------|
| **Provider** | Mock HttpClient，单元测试 recognize() |
| **Registry** | 纯逻辑，单元测试激活/查询 |
| **Service** | Mock Registry + HistoryDb，集成测试 |
| **Commands** | Mock AppState，端到端测试 |

---

### 可扩展性 ✅

| 扩展场景 | 需要改动 |
|---------|---------|
| **加新 Provider** | 1. 实现 Trait<br>2. main.rs 注册 |
| **加新平台** | 1. 实现 Backend Trait<br>2. 添加 `#[cfg]` |
| **OCR 改为多选** | 只改 OcrRegistry，其他不变 |

---

### 依赖方向 ✅

**规则：** 外层 → 内层（单向）

```
Commands → Application → Domain
                ↓
           Infrastructure → External Systems
```

**验证：**
- ✅ Application 可以调用 Infrastructure
- ✅ Commands 可以调用 Application + Infrastructure
- ❌ Infrastructure 不能调用 Application
- ❌ Domain 不能调用任何内部模块

---

## 🚀 实现路线图

### Phase 1: Infrastructure（1-2 天）
- [ ] ConfigFile + 文件锁
- [ ] Keychain（macOS/Windows/Linux）
- [ ] HistoryDb
- [ ] HttpClient Trait + Reqwest
- [ ] 错误类型（thiserror）

### Phase 2: OCR Providers（3-4 天）
- [ ] OcrProvider Trait
- [ ] OcrRegistry（单选）
- [ ] OcrService
- [ ] Tesseract（本地）
- [ ] 百度 OCR（远程）
- [ ] Tauri Commands

### Phase 3: Translation Providers（3-4 天）
- [ ] TranslationProvider Trait
- [ ] TranslationRegistry（多选）
- [ ] TranslationService（并发）
- [ ] Google Translate
- [ ] DeepL
- [ ] OpenAI 兼容

### Phase 4: TTS + 集成（2-3 天）
- [ ] TtsProvider
- [ ] TtsBackend（平台适配）
- [ ] 集成测试

---

## 📚 参考资料

- **CONTEXT.md**：领域语言定义
- **ADR 0002**：主窗口架构（功能域独立）
- **ADR 0003**：Provider 架构设计
- **Clean Architecture**：依赖倒置原则
- **Strategy Pattern**：Provider 是策略模式的实现
