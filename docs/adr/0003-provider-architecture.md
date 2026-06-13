# ADR 0003: Provider 架构设计

## Status
Accepted (2026-06-13)

## Context

SnapLingo 需要支持多种 OCR、翻译、TTS Provider，每种 Provider 有不同的实现方式（本地/远程、免费/付费、内置/自定义）。同时需要支持 macOS、Windows、Linux 三个平台，平台间存在系统 API 差异。

### 核心挑战

1. **多种类型的 Provider**
   - OCR: 6 个内置（Tesseract, PaddleOCR, 百度, 腾讯, Google, Azure），单选
   - Translation: 7 个内置 + 3 个自定义（OpenAI/Claude/Gemini 兼容），多选
   - TTS: 1 个内置（系统 TTS），单选

2. **平台差异**
   - 钥匙串：macOS Keychain / Windows Credential Manager / Linux Secret Service
   - 快捷键：不同 OS 的全局快捷键 API
   - TTS：macOS `say` / Windows SAPI / Linux espeak
   - 路径：macOS `~/Library/Application Support/` / Windows `%APPDATA%` / Linux `~/.config/`

3. **架构要求**
   - 类型安全（编译时检查）
   - 易扩展（加新 Provider 不改现有代码）
   - 可测试（依赖注入，可 mock）
   - 高内聚低耦合（Provider 独立演进）

## Decision

采用 **垂直切片 + 平台抽象** 架构：

### 1. Provider 按类型垂直切片

```
application/providers/
├─ common/              # 共享基础（Provider Trait, Registry 基础逻辑）
├─ ocr/                 # OCR Providers（完整子系统）
│   ├─ trait.rs         # OcrProvider Trait
│   ├─ registry.rs      # OcrRegistry（单选）
│   ├─ service.rs       # OcrService（业务编排）
│   └─ impls/           # 所有实现
├─ translation/         # Translation Providers（完整子系统）
│   ├─ trait.rs
│   ├─ registry.rs      # TranslationRegistry（多选）
│   ├─ service.rs       # TranslationService（并发调用）
│   └─ impls/
└─ tts/                 # TTS Providers（完整子系统）
```

**优点：**
- 功能内聚：一个 Provider 类型的所有代码在一个目录
- 独立演进：修改 OCR 不影响 Translation
- 易导航：目录即功能

### 2. Trait 分层抽象

**基础 Trait（通用能力）：**
```rust
trait Provider {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn is_configured(&self) -> bool;
    fn requires_api_key(&self) -> bool;
}
```

**特化 Trait（具体能力）：**
```rust
trait OcrProvider: Provider {
    async fn recognize(&self, image: &[u8]) -> Result<OcrResult>;
}

trait TranslationProvider: Provider {
    async fn translate(&self, request: TranslationRequest) -> Result<TranslationResult>;
}
```

**优点：**
- 类型安全（编译时检查）
- 共享通用逻辑（id, name 等）
- 各自独立扩展

### 3. Registry 设计模式

**分层设计（避免重复）：**
- **ProviderStore**（共享）：管理 Provider 列表
- **XxxRegistry**（特化）：管理激活逻辑（单选 vs 多选）

```rust
// OCR Registry（单选）
struct OcrRegistry {
    providers: HashMap<Id, Arc<Mutex<dyn OcrProvider>>>,
    active: Option<Id>,  // 单选
}

// Translation Registry（多选）
struct TranslationRegistry {
    providers: HashMap<Id, Arc<Mutex<dyn TranslationProvider>>>,
    active: Vec<Id>,  // 多选
}
```

**职责：**
- ✅ 注册 Provider
- ✅ 激活/停用
- ✅ 查询激活状态
- ❌ 不执行业务逻辑

### 4. Service 业务编排

```rust
struct OcrService {
    registry: Arc<Mutex<OcrRegistry>>,
    history_db: Arc<HistoryDb>,
}

impl OcrService {
    async fn recognize(&self, image: &[u8]) -> Result<OcrResult> {
        // 1. 获取激活的 Provider
        let provider = self.registry.lock().unwrap().get_active()?;
        
        // 2. 调用 Provider
        let result = provider.lock().unwrap().recognize(image).await?;
        
        // 3. 记录历史
        self.history_db.add_ocr_entry(result.clone()).await?;
        
        Ok(result)
    }
}
```

**职责：**
- ✅ 调用 Registry 获取 Provider
- ✅ 调用 Provider 执行
- ✅ 记录历史
- ❌ 不管理 Provider 列表

### 5. 平台适配在 Infrastructure

**原则：** 平台差异用 Trait 抽象 + `#[cfg]` 条件编译

**示例：钥匙串**
```rust
// infrastructure/storage/keychain/

trait KeychainBackend {
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
    backend: PlatformKeychain,
}
```

**编译结果：**
- macOS 版本只包含 MacOSKeychain
- Windows 版本只包含 WindowsKeychain
- 应用层无感知平台差异

### 6. 依赖注入

**Provider 依赖 Infrastructure 抽象：**
```rust
// 远程 Provider 依赖 HttpClient（注入）
struct BaiduOcrProvider {
    http_client: Arc<dyn HttpClient>,
    api_key: Option<String>,
}

// 本地 Provider 无依赖
struct TesseractProvider {
    available: bool,
}
```

**好处：**
- 可测试（mock HttpClient）
- 可替换（换 HTTP 库不改 Provider）

## Consequences

### 优点

1. **类型安全**
   - Trait 抽象编译时检查
   - 不会把 OCR Provider 当成 Translation Provider

2. **高内聚低耦合**
   - 每个 Provider 类型独立目录
   - 修改 OCR 不影响 Translation

3. **易扩展**
   - 加新 Provider：实现 Trait + 注册
   - 加新平台：实现 Backend Trait + #[cfg]

4. **可测试**
   - 依赖注入（HttpClient, TtsBackend）
   - 可以 mock Infrastructure

5. **符合领域语言**
   - 代码结构反映 CONTEXT.md（Provider 概念）
   - 目录名即业务术语（ocr, translation, tts）

### 缺点

1. **代码量增加**
   - 三个 Provider 类型 = 三套 Registry + Service
   - 缓解：提取 common/ 共享逻辑

2. **学习曲线**
   - 需要理解 Trait 继承、Arc<Mutex<dyn>>
   - 缓解：文档完善 + 代码注释

3. **平台适配复杂度**
   - #[cfg] 条件编译增加编译复杂度
   - 缓解：Infrastructure 层统一封装

### 风险

1. **Trait Object 性能**
   - `Arc<Mutex<dyn OcrProvider>>` 有虚函数开销
   - 评估：OCR/翻译本身是重操作，虚函数开销可忽略

2. **依赖注入传递**
   - HttpClient 需要在 main.rs 创建后传递给 Provider
   - 缓解：AppState 统一管理依赖

## Implementation Plan

### Phase 1: Infrastructure 平台适配（1-2 天）
- [ ] Keychain（macOS/Windows/Linux）
- [ ] ConfigFile（JSON 读写）
- [ ] HistoryDb（SQLite）
- [ ] HttpClient（Reqwest 封装）
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
- [ ] OpenAI 兼容（自定义）

### Phase 4: TTS Providers（1-2 天）
- [ ] TtsProvider Trait
- [ ] TtsBackend（平台适配）
- [ ] SystemTtsProvider

### Phase 5: 集成测试（2-3 天）
- [ ] Unit Tests（各 Provider）
- [ ] Integration Tests（完整流程）
- [ ] 跨平台测试（macOS/Windows/Linux）

## References

- CONTEXT.md：领域语言定义
- ADR 0002：主窗口架构（功能域独立）
- Clean Architecture：依赖倒置原则
- Strategy Pattern：Provider 是策略的实现
