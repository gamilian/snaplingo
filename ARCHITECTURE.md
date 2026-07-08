# SnapLingo 架构设计文档

> 当前版本 - 2026-07-08

## 📐 架构概览

SnapLingo 采用**分层架构 + 垂直切片**设计模式：

```
Commands Layer (接口层)
    ↓
Application Layer (应用层)
    ├─ Providers (垂直切片：OCR/Translation/TTS)
    └─ Application Modules (Capture Session、Pinned Image、History)
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

**运行时 seam：**
- `src/tauri/*` 是前端 Tauri Adapter seam，集中维护 command 名称和 payload 形状。
- `src/tauri/appEvents.ts` 是前端 Tauri event Adapter seam，集中维护主窗口 event 名称、payload 解析和订阅清理。
- `src/tauri/settings.ts` + `src/stores/settingsConfigStore.ts` 是前端 durable settings seam；Settings、Capture、Result、Pinned 窗口都从同一后端 snapshot hydrate。
- `src-tauri/src/commands/*` 是后端 Tauri command seam，负责把 IPC 请求转给 Application 层。
- `src-tauri/src/app_state.rs` 拥有 AppState 形状和关闭顺序。
- `src-tauri/src/composition.rs` 是应用组合入口；`src-tauri/src/composition/*_runtime.rs` 拆分 Provider、Capture、Selection、History 的构造策略。
- `src-tauri/src/startup_shortcuts.rs` 拥有启动期全局快捷键注册。
- `src-tauri/src/application/settings/configuration.rs` 是 Settings Configuration module，拥有 durable settings 默认值、路径归一化、section 更新和 legacy migration。
- `src-tauri/src/application/services/selected_text_acquirer.rs` 是 Selected Text acquisition workflow，拥有取词方法顺序和诊断；平台取词 mechanics 留在 `infrastructure/system/selection/*`。
- `application/services/capture_session_runtime.rs` 是 Capture Session Runtime，统一编排截图输出和 OCR。
- `src/components/ScreenshotSession/captureInteractionRuntime.ts` 是前端 Capture Interaction Runtime，负责纯 effect-plan 决策。

---

## 📁 目录结构

```
snaplingo/
├─ src/                                 # React/Vite Frontend Runtime
│   ├─ tauri/                           # ⭐ Frontend Tauri Adapter seam
│   │   ├─ appEvents.ts                  # 主窗口 Tauri event Adapter
│   │   ├─ settings.ts                   # durable settings command adapter
│   │   ├─ translation.ts
│   │   ├─ providers.ts
│   │   ├─ history.ts
│   │   ├─ captureSession.ts
│   │   └─ pinnedImage.ts
│   │
│   ├─ components/
│   │   ├─ SettingsWindow/              # 设置窗口 + 纯导航状态模型
│   │   ├─ ScreenshotSession/           # 截图会话 UI + 交互 runtime/model
│   │   ├─ ResultWindow/                # 结果窗口
│   │   └─ PinnedImageWindow/           # 贴图窗口
│   │
│   ├─ hooks/
│   └─ stores/                          # Zustand 状态管理
│       ├─ settingsConfigStore.ts        # 后端 snapshot backed durable settings
│       └─ settingsStore.ts              # Settings UI navigation + hotkey UI state
│
├─ src-tauri/src/
│   ├─ main.rs                          # Tauri binary 入口
│   ├─ lib.rs                           # Tauri builder/plugin setup + command 注册
│   ├─ app_state.rs                     # AppState 形状 + shutdown
│   ├─ composition.rs                   # ⭐ Runtime composition assembly shell
│   ├─ composition/                     # 构造策略 builders
│   │   ├─ provider_runtime.rs
│   │   ├─ capture_runtime.rs
│   │   ├─ selection_runtime.rs
│   │   └─ history_runtime.rs
│   ├─ startup_shortcuts.rs             # 启动期全局快捷键注册
│   ├─ error.rs                         # 统一错误类型
│   │
│   ├─ commands/                        # ⭐ Backend Tauri command seam
│   │   ├─ provider_commands.rs
│   │   ├─ ocr_commands.rs
│   │   ├─ translation_commands.rs
│   │   ├─ capture_session_commands.rs
│   │   ├─ pinned_image_commands.rs
│   │   ├─ history_commands.rs
│   │   └─ ...
│   │
│   ├─ application/                     # ⭐ Application Layer
│   │   ├─ settings/                    # durable user settings module
│   │   │   ├─ configuration.rs          # defaults, merge, path normalization, persistence
│   │   │   └─ mod.rs
│   │   ├─ providers/                   # Provider 垂直切片
│   │   │   ├─ common/                  # 共享基础
│   │   │   │   ├─ provider.rs          # Provider Trait
│   │   │   ├─ configuration.rs         # Provider 配置生命周期
│   │   │   │
│   │   │   ├─ ocr/                     # OCR Providers
│   │   │   │   ├─ trait_def.rs
│   │   │   │   ├─ coordinator.rs       # 单选 + 运行时重配置
│   │   │   │   └─ impls/               # 具体实现
│   │   │   │
│   │   │   ├─ translation/             # Translation Providers
│   │   │   │   ├─ trait_def.rs
│   │   │   │   ├─ coordinator.rs       # 多选 + 并发执行
│   │   │   │   └─ impls/               # 具体实现
│   │   │
│   │   └─ services/
│   │       ├─ selected_text_acquirer.rs # ⭐ Selected Text acquisition workflow
│   │       ├─ capture_session_runtime.rs # ⭐ Capture Session Runtime
│   │       ├─ capture_session_service.rs
│   │       ├─ image_composition_service.rs
│   │       ├─ capture_output_service.rs
│   │       ├─ pinned_image_service.rs
│   │       └─ history_service.rs
│   │
│   ├─ domain/                          # ⭐ Domain Layer
│   │   ├─ capture.rs                   # 纯数据结构
│   │   ├─ config.rs                    # durable settings snapshot types
│   │   ├─ selection.rs                 # selected-text method/source/result types
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
│       │   ├─ selection/               # Selected Text platform adapters ⭐
│       │   ├─ tts/                     # 平台适配 ⭐
│       │   └─ ...
│       │
│       └─ http/
│           ├─ client.rs                # HttpClient Trait
│           └─ reqwest_impl.rs
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
    state.ocr_coordinator.activate(&provider_id)
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
├─ trait_def.rs     # OcrProvider Trait 定义
├─ coordinator.rs   # OcrCoordinator（单选、持久化、执行、运行时重配置）
└─ impls/           # 具体实现
    ├─ tesseract.rs      # 本地
    ├─ baidu_ocr.rs      # 远程（依赖 HttpClient）
    └─ ...
```

**职责划分：**

| 模块 | 职责 | 不负责 |
|------|------|--------|
| **Trait** | 定义 Provider 接口 | 不管理实例、不执行业务 |
| **Coordinator** | 管理 Provider 列表、激活状态、持久化、执行协调和运行时重配置 | 不实现具体 OCR/翻译 API |
| **configuration.rs** | 校验凭证、自定义 Translation Provider 生命周期、构造自定义 LLM Provider | 不执行翻译/OCR 请求 |
| **impls/** | 实现具体能力（OCR 识别） | 不管理自己的激活状态 |

**Translation Provider 特殊性：**
- TranslationCoordinator 是**多选**（可同时激活多个）
- TranslationCoordinator **并发调用**所有激活的 Provider
- 返回 `Vec<TranslationResult>` 供用户对比

---

#### 2.2 Application Modules（应用模块）

**示例：**
- `settings/configuration.rs`：durable user settings 的默认值、读取、section 更新、路径归一化和 legacy migration
- `selected_text_acquirer.rs`：划词翻译取词 workflow，按 scheme 调用平台 method 并生成诊断
- `capture_session_runtime.rs`：统一编排截图会话渲染、输出、OCR
- `capture_session_service.rs`：创建和读取冻结桌面会话
- `image_composition_service.rs`：裁剪、标注、合成图像
- `capture_output_service.rs`：保存、复制、贴图输出
- `pinned_image_service.rs`：贴图状态和恢复
- `history_service.rs`：订阅领域事件并写入历史

**Settings Configuration 边界：**
- 后端 owns durable defaults：`general`、`screenshot`、`translation`
- 前端通过 `src/tauri/settings.ts` 调用 section update command，不直接写 durable localStorage
- `settingsConfigStore.ts` 只缓存后端 snapshot 并负责一次性 legacy migration
- `settingsStore.ts` 只保留 Settings UI navigation 和 hotkey UI state；热键注册生命周期仍由 `startup_shortcuts.rs` 拥有

**Selected Text acquisition 边界：**
- `SelectedTextAcquirer` 拥有取词方法顺序、成功短路和失败诊断格式
- `SelectionMethodRegistry` 只按 `SelectionMethodKind` 找 method，不做 workflow 决策
- macOS/Windows/Linux method 实现留在 `infrastructure/system/selection/*`
- Windows/Linux 目前通过 `ShortcutCopy` adapter 执行 `Ctrl+C` + clipboard transaction；macOS 仍保留 SelfWebview、Accessibility、BrowserScript、MenuCopy、ShortcutCopy 的原顺序

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
- `selection/`：Selected Text 平台取词 method；每个平台 provider 暴露 default scheme 和 method list
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
Frontend Adapter: activateOcrProvider('baidu-ocr')
    ↓
src/tauri/providers.ts: invoke('activate_ocr_provider', { providerId })
    ↓ Tauri IPC
Commands: activate_ocr_provider(provider_id, State)
    ↓
Application: OcrCoordinator.activate(provider_id)
    ↓ 持久化
Infrastructure: ConfigFile.save('active_ocr_provider', provider_id)
    ↓
File System: ~/.snaplingo/config.json
```

---

### 场景 2：用户配置 Provider API Key

```
用户输入 API Key → 点击保存
    ↓
Frontend Adapter: configureTranslationProviderCredentials(...)
    ↓
Commands: configure_translation_provider_credentials(provider_id, credentials, State)
    ↓ 分两路
    │
    ├─→ Infrastructure: Keychain.save_provider_credentials(provider_id, credentials)
    │       ↓ 平台适配
    │       ├─ macOS: Keychain API
    │       ├─ Windows: Credential Manager API
    │       └─ Linux: Secret Service API
    │
    └─→ Application: TranslationCoordinator.reconfigure_provider(provider_id, credentials)
            ↓
            已注册 Provider 立即更新运行时凭证
```

---

### 场景 3：用户修改 durable settings

```
用户修改 Settings 页面字段
    ↓
Settings page: useSettingsConfigStore.update*Settings(section)
    ↓
Frontend Adapter: src/tauri/settings.ts
    ↓
Commands: update_*_settings(input, State)
    ↓
Application: SettingsConfiguration.update_*(input)
    ↓
Infrastructure: ConfigFile.save("settings", snapshot)
    ↓
Frontend: all windows hydrate the same settings snapshot
```

---

### 场景 4：划词翻译取词

```
用户触发划词翻译快捷键
    ↓
Commands: open_selection_translation_window_for_state(...)
    ↓
Application: SelectedTextAcquirer.acquire()
    ↓
Selection scheme order
    ├─ macOS: SelfWebview → Accessibility → BrowserScript → MenuCopy → ShortcutCopy
    ├─ Windows: ShortcutCopy
    └─ Linux: ShortcutCopy
    ↓
Infrastructure: platform selection adapter
    ↓ 成功
Result Window opens with selected text
    ↓ 失败
one string error surface includes attempted method diagnostics
```

---

### 场景 5：用户执行翻译（多 Provider 并发）

```
用户输入文本 → 点击翻译
    ↓
Frontend Adapter: translateText(...)
    ↓
src/tauri/translation.ts: invoke('translate_text_v2', payload)
    ↓
Commands: translate_text_v2(request, State)
    ↓
Application: TranslationCoordinator.translate(request)
    ↓ 获取激活的 Providers
    TranslationCoordinator.get_active()
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
struct OcrCoordinator {
    providers: Mutex<HashMap<String, Arc<RwLock<dyn OcrProvider>>>>,
    active: Arc<Mutex<Option<String>>>,
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

// 在 composition/provider_runtime.rs 注入
let http_client = Arc::new(ReqwestHttpClient::new());
let provider = BaiduOcrProvider::new(Arc::clone(&http_client));
```

---

### 3. 协调器模式（Coordinator Pattern）

**集中管理 Provider 实例、激活状态、持久化和执行：**
```rust
struct OcrCoordinator {
    providers: Mutex<HashMap<String, Arc<RwLock<dyn OcrProvider>>>>,
    active: Arc<Mutex<Option<String>>>,  // 单选
    config: Arc<ConfigFile>,
}

impl OcrCoordinator {
    pub fn register<T: OcrProvider + 'static>(&self, provider: T) -> Result<()> { ... }
    pub fn activate(&self, id: &str) -> Result<()> { ... }
    pub fn get_active(&self) -> Option<Arc<RwLock<dyn OcrProvider>>> { ... }
    pub fn reconfigure_provider(&self, id: &str, credentials: &HashMap<String, String>) -> Result<()> { ... }
}
```

---

### 4. 运行时门面（Runtime Facade）

**Capture Session Runtime 简化截图会话输出路径：**
```rust
// 对外暴露简单接口
struct CaptureSessionRuntime {
    sessions: Arc<CaptureSessionService>,
    composer: Arc<ImageCompositionService>,
    output: Arc<CaptureOutputService>,
    ocr: Arc<OcrCoordinator>,
}

impl CaptureSessionRuntime {
    pub fn output_selection(&self, input: CaptureOutputInput) -> Result<CaptureOutputOutcome> { ... }
    pub async fn recognize_selection_text(&self, input: CaptureOcrInput) -> Result<String> { ... }
}
```

---

## ⚖️ 架构权衡

### 权衡 1：垂直切片 vs 水平分层

| 方案 | 优点 | 缺点 |
|------|------|------|
| **垂直切片**（采用） | 功能内聚、易导航、独立演进 | 有少量 Coordinator 结构重复 |
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

### 权衡 3：独立 Coordinator vs 泛型 Coordinator

| 方案 | 优点 | 缺点 |
|------|------|------|
| **独立 Coordinator**（采用） | 类型安全、激活逻辑独立、测试直接 | 少量重复 |
| **泛型 Coordinator** | 无重复 | Rust 泛型 + dyn Trait 复杂，接口更浅 |

**决策：** 独立 Coordinator。OCR 是单选，Translation 是多选且并发执行，合并成泛型会把差异推给调用者。

---

## 📊 架构质量指标

### 可测试性 ✅

| 模块 | 如何测试 |
|------|---------|
| **Provider** | Mock HttpClient，单元测试 recognize() |
| **Coordinator** | 单元测试激活、恢复、重配置和执行协调 |
| **Settings Configuration** | Rust 测 section defaults、partial update、path normalization、legacy migration |
| **SelectedTextAcquirer** | Rust 测 method ordering、success short-circuit、unsupported/unavailable/failed diagnostics |
| **Capture Session Runtime** | 通过一个 Interface 测试 render/output/OCR 编排 |
| **Capture Interaction Runtime** | Vitest 测选区完成后的 effect plan |
| **Frontend Tauri Adapter** | Vitest 测 command 名称和 payload 映射 |
| **Commands** | Mock AppState，端到端测试 |

---

### 可扩展性 ✅

| 扩展场景 | 需要改动 |
|---------|---------|
| **加新 Provider** | 1. 实现 Trait<br>2. 在 `composition/provider_runtime.rs` 注册<br>3. 如需凭证，接入 Provider Configuration Module |
| **加新平台** | 1. 实现 Backend Trait<br>2. 添加 `#[cfg]` |
| **OCR 改为多选** | 改 OcrCoordinator 激活模型和对应前端 adapter/store |

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

## 🚀 当前演进状态

- Frontend runtime 已通过 `src/tauri/*` 适配器集中调用 Tauri commands。
- 主窗口 Tauri events 已通过 `src/tauri/appEvents.ts` 集中订阅和解析。
- Backend runtime 已通过 `src-tauri/src/commands/*` 保持 Tauri command seam。
- Provider 当前由 `TranslationCoordinator` 和 `OcrCoordinator` 统一管理激活、持久化、执行和运行时重配置。
- Provider Configuration Module 负责凭证校验、自定义 Translation Provider 定义、运行时新增/注册/激活和回滚。
- Capture Session Runtime 已集中截图会话的 render/output/OCR 编排。
- AppState 形状位于 `src-tauri/src/app_state.rs`；`lib.rs` 只保留 Tauri builder/plugin setup、command 注册和启动模块调用。
- Application Composition 由 `src-tauri/src/composition.rs` assembly shell 和 `src-tauri/src/composition/*_runtime.rs` 构造策略 builders 组成。
- 启动期快捷键注册位于 `src-tauri/src/startup_shortcuts.rs`。
- Settings navigation state、Capture interaction runtime/model 是前端纯模块 seam，可用 Vitest 直接覆盖交互规则。

---

## 📚 参考资料

- **CONTEXT.md**：领域语言定义
- **ADR 0002**：主窗口架构（功能域独立）
- **ADR 0003**：Provider 架构设计（当前 Coordinator 结构）
- **ADR 0004**：Coordinator Consolidation
- **ADR 0005**：Runtime Provider Reconfiguration
- **Clean Architecture**：依赖倒置原则
- **Strategy Pattern**：Provider 是策略模式的实现
