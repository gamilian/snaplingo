# Domain Language

## Project Name

**SnapLingo** - Snap（截图）+ Lingo（语言）

跨平台截图、OCR、翻译工具。

## Core Concepts

### Provider（提供者）
翻译或 OCR 服务的可插拔实现。每个 Provider 实现 `TranslationProvider` 或 `OcrProvider` trait，封装特定 API 的逻辑（认证、请求格式化、响应解析）。

**已实现的 Providers：**
- Translation: Google Translate、DeepL、Baidu Translation
- OCR: Tesseract（本地）、System OCR（macOS，本地）、Baidu OCR（远程）

### Coordinator（协调器）
管理 Providers 并协调其执行的核心模块。每种 Provider 类型有对应的 Coordinator：

- **TranslationCoordinator**：管理翻译 Providers，协调并发翻译
- **OcrCoordinator**：管理 OCR Providers，执行单个 OCR 识别

**职责：**
- Provider 注册和管理
- 激活状态管理（多选/单选）
- 配置持久化（自动保存/恢复）
- 执行协调（并发/单次调用）

**设计原则：**
- 使用内部细粒度锁（`Arc<Mutex<Vec<String>>>`）实现并发安全
- Builder 模式初始化：构建时可变（`&mut self`），使用时不可变（`Arc<Self>`）
- Provider 注册表由 Coordinator 管理；Provider 凭证可通过 Coordinator Interface 在运行时重配置
- `active` 运行时可变（需要锁），调用方不直接持有 Provider 的可变状态

### Frontend Tauri Adapter（前端 Tauri 适配器）
`src/tauri/*` 中的 TypeScript 模块，封装前端到 Tauri command 的调用细节。

**职责：**
- 集中维护 command 名称和 payload 形状
- 集中维护 Tauri event 名称、payload 解析和订阅清理（`src/tauri/appEvents.ts`）
- 为 UI、hooks、stores 提供 typed function，而不是让它们直接调用 `invoke()`
- 让前端/后端 seam 在代码导航时清晰可见

### Application Composition（应用组合）
`src-tauri/src/composition.rs` 和 `src-tauri/src/composition/*_runtime.rs` 中的运行时装配模块。

**职责：**
- `composition.rs` 保持为 AppState assembly shell，只创建共享基础设施并调用 builder
- 构建 Provider Coordinators 并注册内置 Provider
- 通过 Provider Configuration Module 恢复自定义 Translation Provider，并恢复 Provider 激活状态
- 构建 Capture、Selection、History 等运行时依赖组合
- 将 EventBus 注入 Coordinator，并在 Tauri runtime 就绪后订阅 HistoryService
- 让 `lib.rs` 保持为 Tauri builder/plugin setup、command 注册和启动模块调用的启动壳

### App State（应用状态）
`src-tauri/src/app_state.rs` 中的运行时状态形状。

**职责：**
- 聚合 ConfigFile、Keychain、HttpClient、Coordinators、Capture Runtime、History、EventBus 等运行时依赖
- 定义应用关闭时的清理顺序
- 不负责依赖构建；依赖构建由 Application Composition 完成

### Provider Activation（Provider 激活）
使 Provider 可用的过程。SnapLingo 支持两种激活模型：

- **多选模式（Translation）**：多个 Provider 可同时激活。请求会并发发送到所有活动 Provider 以便比较结果。
  ```
  状态：Vec<String> = ["google", "deepl", "baidu"]
  配置键：active_translation_providers
  实现：TranslationCoordinator 使用 tokio::spawn 并发调用
  ```

- **单选模式（OCR）**：同时只能激活一个 Provider。激活新 Provider 会替换之前的。
  ```
  状态：Option<String> = Some("tesseract")
  配置键：active_ocr_provider
  实现：OcrCoordinator 直接调用单个 Provider
  ```

这种区别反映了使用模式：用户需要比较多个翻译结果，但只需要一个 OCR 结果。

### Configuration Persistence（配置持久化）
Provider 激活状态自动保存到磁盘。Coordinator 模块内部处理持久化——Commands 层不知道存储机制。这确保：

- **局部性（Locality）**：状态管理和持久化在 Coordinator 中共同定位
- **原子性（Atomicity）**：激活和持久化作为一个操作成功或失败
- **配置清理**：无效 Provider（如会话间被删除）在恢复时自动清理

**实现细节：**
- Coordinator 拥有 `Arc<ConfigFile>`，在状态变更时立即调用 `config.save()`
- 启动时通过 `restore_from_config()` 恢复状态，跳过无效 Provider
- 持久化失败导致整个 `activate()` 操作失败（通过 `?` 运算符）

**并发安全：**
- `active` 状态用 `Arc<Mutex<Vec<String>>>` 或 `Arc<Mutex<Option<String>>>` 包装
- 短锁：只在修改 `active` 列表时锁定，读取后立即释放
- `translate()` 和 `recognize()` 可以并发调用，互不阻塞

### Provider Configuration Module（Provider 配置模块）
`src-tauri/src/application/providers/configuration.rs` 中的 `ProviderConfiguration` struct。

**职责：**
- 完整的自定义 Translation Provider 生命周期管理：add、update、remove
- 凭证管理：保存、加载、删除 keychain 条目
- 测试自定义 Provider 连接（test_custom_provider）
- Update 操作包含失败回滚：coordinator 替换失败时恢复旧定义和旧 API key
- 与 `LlmIntrospection` 配合进行 Provider 测试
- 与 Coordinator 的运行时重配置能力配合，使配置命令保存凭证后立即更新已注册 Provider，无需重启应用

**边界：**
- 不执行翻译/OCR 请求
- 不负责 Provider 内省操作（list_models/test），由 `LlmIntrospection` 负责

### LLM Introspection Module（LLM 内省模块）
`src-tauri/src/application/providers/llm_introspection.rs` 中的 `LlmIntrospection` struct。

**职责：**
- LLM provider 内省操作：list_models 和 test
- 集中 LLM 客户端构造逻辑，按 protocol 分发到正确的客户端
- 在保存配置前测试 API 端点连接
- 列举 API 端点可用模型供 UI 选择

**边界：**
- 不管理 Provider 生命周期（add/update/remove）
- 不保存凭证或配置
- 仅用于 UI 预览和验证

### Settings Configuration Module（设置配置模块）
`src-tauri/src/application/settings/configuration.rs` 中的 durable settings 模块。

**职责：**
- 拥有 `general`、`screenshot`、`translation` 三类持久设置的默认值
- 从 `ConfigFile` 读取和保存 sectioned settings snapshot
- 对截图保存路径做 backend-owned normalization
- 提供 section-specific update：`update_general`、`update_screenshot`、`update_translation`
- 一次性迁移旧前端 localStorage 中的 durable 设置，但不接管导航状态或热键注册生命周期

**边界：**
- Settings Configuration 不负责快捷键配置或注册；全局快捷键由 Hotkey Configuration / Runtime 管理
- Provider 配置仍由 Provider Configuration Module 和各 Coordinator 管理
- 前端不再直接持久化 general/screenshot/translation durable values

### Hotkey Configuration Module（快捷键配置模块）
`src-tauri/src/application/hotkeys/configuration.rs` 中的快捷键配置生命周期模块。

**职责：**
- 拥有 screenshot / translation / OCR 三类快捷键 snapshot 的读取、合并和保存
- 使用现有 `"hotkeys"` 配置 key，保持旧配置文件兼容
- 迁移旧前端 WebKit localStorage 中的 hotkeys，但忽略 durable settings 和导航状态
- 校验未知 category/action，并复用 display hotkey parser 过滤无效快捷键

### Hotkey Runtime（快捷键运行时）
`src-tauri/src/application/hotkeys/runtime.rs` 中的全局快捷键注册生命周期模块。

**职责：**
- 启动时从 Hotkey Configuration snapshot 注册全局快捷键
- 运行时更新时先注册/注销系统快捷键，再推进后端配置 snapshot
- 持有当前注册表，避免前端缓存、配置文件和系统注册状态互相漂移
- 将快捷键触发委托给 `startup_shortcuts.rs` 中的 action dispatch/timing 规则

### Durable Settings Store（持久设置 Store）
`src/stores/settingsConfigStore.ts` 中的前端 store。

**职责：**
- 通过 `src/tauri/settings.ts` hydrate 后端 settings snapshot
- 为 Settings、ScreenshotSession、PinnedImageWindow、ResultWindow 共享同一份 durable settings cache
- 调用 section update command 后用后端返回的 snapshot 更新本地 cache
- 清理已迁移的旧 durable localStorage keys，保留导航状态和 legacy hotkeys 给后端迁移

### Settings Navigation State（设置导航状态）
`src/components/SettingsWindow/settingsNavigationState.ts` 中的前端纯模型。

**职责：**
- 根据当前 Settings section 和持久化 secondary key 解析实际 active secondary item
- 当持久化 key 过期时回退到该 section 的第一个 secondary item
- 对用户点击的 secondary key 做 section 内合法性校验，避免 UI 调用方散落 switch 逻辑

### Settings UI Store（设置 UI Store）
`src/stores/settingsStore.ts` 中的前端 store。

**职责：**
- 保存 Settings Window 的 main tab / secondary tab UI 状态
- 不保存 durable general/screenshot/translation 设置；这些值必须走 Durable Settings Store
- 不保存 hotkey state；快捷键展示和录制必须走 Hotkey Config Store

### Hotkey Config Store（快捷键配置 Store）
`src/stores/hotkeyConfigStore.ts` 中的前端 store。

**职责：**
- 通过 `src/tauri/hotkeys.ts` hydrate 后端 hotkey snapshot
- 作为 Settings hotkey 页面缓存，不拥有默认值来源
- 更新、清空和重置快捷键时调用后端 `update_hotkey`，用返回的 snapshot 覆盖本地缓存
- 不做系统注册；注册副作用只发生在后端 Hotkey Runtime

### Capture Mode（捕获模式）
用户触发的五种独立功能入口：

- **Screenshot Mode（截图模式）**  
  快捷键触发框选截图，进行编辑（标注、箭头、文字等），最后保存或复制图片。不涉及 OCR 或翻译。
  
  **编辑工具**（参考 Snipaste）：
  - **矩形**：可转换为正方形（按住 Shift）
  - **椭圆**：可转换为圆形（按住 Shift）
  - **折线**：可转换为水平/竖直/45度线（按住 Shift）
  - **箭头**
  - **画笔**：自由绘制，可中途转换为直线（按住 Shift）
  - **马克笔**：半透明高亮，可转换为水平/竖直/45度线（按住 Shift）
  - **文字标注**：可调整字体、大小、颜色
  - **马赛克**
  - **高斯模糊**
  - **橡皮擦**
  - **撤销/重做**
  
  每个工具支持：
  - 颜色选择
  - 粗细/大小调整
  - 填充 vs 边框切换（几何图形）
  
  编辑完成后的操作选项：
  - **保存**：保存到配置的默认目录（如 `~/Pictures/SnapLingo`），文件名自动生成（时间戳）。提供"另存为"选项手动选择路径。
  - **复制**：复制图片到剪贴板，操作完成后窗口关闭。
  - **贴图**：将图片固定在屏幕最顶层（类似 Snipaste Paste 功能），编辑窗口关闭，图片变成独立的置顶窗口。
    - **窗口特性**：永远置顶，可拖动位置，可缩放（鼠标滚轮或拖拽边角）
    - **交互**：鼠标悬停显示工具栏（关闭、保存、复制、置顶开关）
    - **多贴图**：支持同时贴多张图
    - **持久化**：贴图不持久化，应用退出后消失
  - **取消**：关闭编辑窗口，不保存不复制。
  - **OCR 按钮**：编辑工具栏提供 OCR 按钮，可直接对当前编辑的图片执行 OCR，转入 OCR Mode 流程。OCR 识别的是**原始截图**（不包含标注），因为标注会干扰识别准确率。

- **OCR Mode（OCR 模式）**  
  快捷键触发框选截图，自动对图片执行 OCR 识别文本。识别完成后显示文本，用户可手动点击"翻译"按钮。

- **OCR + Translation Mode（OCR 并翻译模式）**  
  快捷键触发框选截图，自动对图片执行 OCR 识别文本。OCR 完成后立即显示文本，翻译在后台自动执行并追加结果。

- **Selection Translation Mode（划词翻译模式）**  
  用户在任意应用中选中文字，按快捷键触发，自动复制选中内容并翻译。
  
  **重要**：此快捷键仅在有文本被选中时生效。如果没有选中文本，快捷键会传递给系统（执行系统原本绑定的操作）。

- **Input Translation Mode（输入翻译模式）**  
  快捷键触发，弹出翻译窗口，用户直接在窗口中输入文字。按回车或点击"翻译"按钮执行翻译。

### Selected Text Acquisition（划词取词）
划词翻译中从当前前台应用获取用户选中文本的 workflow。

**核心模块：**
- `SelectedTextAcquirer`：Application 层 workflow，负责 method ordering、成功短路、失败诊断聚合
- `SelectionMethodRegistry`：按 `SelectionMethodKind` 找到具体 method
- `infrastructure/system/selection/*`：平台 method mechanics，包括 macOS 专用方法以及 Windows/Linux `ShortcutCopy`

**平台策略：**
- macOS：保留 SelfWebview → Accessibility → BrowserScript → MenuCopy → ShortcutCopy 的多方法顺序
- Windows：使用 `ShortcutCopy`，通过 `Ctrl+C` 和 clipboard transaction 获取选中文本
- Linux：使用 `ShortcutCopy`，通过 `Ctrl+C` 和 clipboard transaction 获取选中文本，失败时保留明确的 clipboard / synthetic input 错误

**诊断规则：**
- 所有失败最终仍返回一个字符串 error surface 给 `open_selection_translation_window_for_state(...)`
- 诊断字符串包含尝试过的方法名，并区分 `unsupported`、`unavailable`、`failed` 和 empty text
- 平台适配器负责产出平台原因，`SelectedTextAcquirer` 只负责排序和聚合

### Capture Session（截图会话）
一次截图从快捷键触发到输出完成或取消的完整生命周期。Capture Session 不是简单的截图 API 调用，而是 SnapLingo 截图链路的核心领域对象。

**职责：**
- 创建时冻结当前桌面画面，保存每个显示器的截图、位置、尺寸和缩放信息
- 为前端提供可渲染的冻结画面和统一坐标元数据
- 接收用户选择的区域和标注命令
- 为 OCR 提供未标注的原始图像裁剪
- 为复制、保存、贴图提供合成后的输出图像
- 只有输出成功后，才把截图记为成功截图并写入历史

**坐标规则：**
- 前端交互使用 Logical Pixels
- 后端裁剪和图像处理使用 Physical Pixels
- 所有跨层传递的区域必须显式标注坐标空间，不能裸传 `x/y/width/height`

**与 Capture Mode 的关系：**
- Screenshot Mode：Capture Session → 选区/标注 → 复制/保存/贴图
- OCR Mode：Capture Session → 选区 → OCR
- OCR + Translation Mode：Capture Session → 选区 → OCR → 翻译
- Screenshot Mode 中的 OCR 按钮复用同一个 Capture Session 的原始图像

### Capture Session Runtime（截图会话运行时）
`CaptureSessionRuntime` 是 Application 层的深模块，统一编排 Capture Session 的输出和 OCR 路径。

**职责：**
- 调用 `CaptureSessionService` 读取冻结桌面和选区
- 调用 `ImageCompositionService` 渲染复制、保存、贴图所需的最终图像
- 调用 `CaptureOutputService` 处理剪贴板、文件输出和输出结果判断
- 调用 `OcrCoordinator` 对原始选区图像执行 OCR
- 让 Commands 层通过一个 Interface 完成 render/output/OCR，而不是了解多个服务的调用顺序

### Capture Interaction Runtime（截图交互运行时）
`src/components/ScreenshotSession/captureInteractionRuntime.ts` 中的前端纯运行时决策模块。

**职责：**
- 根据 Capture Mode 决定选区完成后的 flow（预览、OCR、OCR + 翻译）
- 根据完成动作生成有序 effect plan，决定是否记录成功截图、是否结束 session、OCR 结果进入哪个窗口
- 不调用 Tauri、DOM 或 React hooks；effect plan 的解释和副作用执行由 Capture Workspace host seam 绑定到 host adapter
- `captureInteractionModel.ts` 只保留兼容 facade 和 Capture Mode 到 flow 的纯规则

### Capture Workspace（截图工作区）
`src/components/ScreenshotSession/captureWorkspace*.ts` 和 `CaptureWorkspaceView.tsx` 组成前端截图工作区 seam，位于纯 plan 模块和 React shell 之间。

**职责：**
- 拥有前端截图工作区状态形状、patch/reset/load 规则和 ref-backed state 同步
- 把 host workflow（start/refresh/cancel/render/complete）绑定到 `captureHostRuntime` 和 Tauri adapter，但把 native command 细节留在 host adapter 外侧
- 把 keyboard、pointer 和 wheel 事件分发到 selection/editor/candidate 纯 plan 模块，避免 `ScreenshotSession/index.tsx` 直接持有大块交互分支
- 通过 `CaptureWorkspaceView.tsx` 渲染截图工作区；View 只接收状态、几何和 handler props，不启动 session、不读 localStorage、不调用 Tauri adapter、不读写 workflow refs
- 让 `ScreenshotSession/index.tsx` 保持为 composition shell：读取 settings，初始化 workspace state，计算 derived geometry，创建 host/keyboard/pointer actions，连接 hooks，渲染 View

### Provider（能力提供者）
实现某个能力的内置模块，不区分本地实现还是远程 API 调用。用户视角看到的是能力名称（如"DeepL 翻译"），而非技术实现细节。

Provider 类型：
- **OCR Provider** - 图像识别为文字
- **Translation Provider** - 文本翻译

**重要**：Provider 不是插件系统。所有 Provider 都是内置的，用户通过配置界面激活/停用并设置参数（如 API Key）。

**无付费选项**：应用本身不提供任何付费服务或订阅，所有 Provider 均需用户自行配置 API Key 或使用本地能力。

### 内置 Provider 列表

**OCR Provider**：
- Tesseract（免费，本地）
- System OCR（macOS，本地）
- 百度 OCR（需 API Key）

**未来候选 OCR Provider（未实现）**：
- PaddleOCR（免费，本地，中文优化）
- 腾讯云 OCR（需 API Key）
- Google Cloud Vision（需 API Key）
- Azure Computer Vision（需 API Key）

**Translation Provider**：
- Google Translate（免费 API）
- DeepL（需 API Key）
- 百度翻译（需 API Key）
- 有道翻译（需 API Key）
- 腾讯翻译君（需 API Key）
- OpenAI（需 API Key，支持 GPT 模型）
- Azure Translator（需 API Key）

### 自定义 Translation Provider
**仅支持自定义翻译 Provider**，不支持自定义 OCR Provider。

兼容以下 API 格式：
- **OpenAI 兼容格式**（如 OpenAI API、各类国内大模型、Ollama 等）
- **Claude 兼容格式**（Anthropic API）
- **Gemini 兼容格式**（Google AI Studio）

用户可配置：
- Provider 名称（显示名）
- API 格式类型（OpenAI / Claude / Gemini）
- API 端点 URL
- API Key
- 模型名称
- 自定义请求头（可选）

### Provider 激活规则
- **OCR Provider**：同一时间只能激活一个。用户尝试激活第二个时，系统警告并要求先停用当前激活的。
- **Translation Provider**：可以同时激活多个。翻译时并发调用所有已激活的，结果并排显示供用户对比。

### Provider 配置存储
- **敏感信息**（API Key、密钥等）：存储在系统级加密存储
  - macOS: Keychain
  - Windows: Credential Manager
  - Linux: Secret Service
- **非敏感配置**（语言偏好、是否激活、超时设置等）：存储在统一配置文件 `~/.snaplingo/config.json`
- **持久用户设置**（general/screenshot/translation）：通过 Settings Configuration Module 读写；前端只通过 `settingsConfigStore` hydrate 和 section update
- **快捷键配置**（screenshot/translation/OCR）：通过 Hotkey Configuration / Runtime 读写和注册；前端只通过 `hotkeyConfigStore` hydrate 和 update
- **配置持久性**：配置不随应用卸载删除，用户需在设置中主动"清除所有数据"才会删除

## Workflows

### OCR Mode 自动翻译流程
当用户启用"OCR 后自动翻译"配置时：
1. 用户框选完成
2. 开始 OCR 识别（显示进度）
3. OCR 完成，立即显示识别的文本
4. 翻译在后台执行
5. 翻译完成后更新界面，追加翻译结果

**设计原因**：渐进式反馈，避免用户等待 4-5 秒黑盒处理。用户先确认 OCR 正确性，再获得翻译。

### OCR 结果窗口
显示内容：
- OCR 识别的文本（可编辑，用户能修正错误）
- "复制"按钮
- "翻译"按钮（不管自动翻译开关状态如何，都提供手动触发）
- 翻译结果区域（如果触发了翻译）

### 多翻译结果展示
当同时激活多个 Translation Provider 时，翻译结果以**卡片式垂直布局**显示：
- 每个 Provider 一个独立卡片
- 卡片头部显示 Provider 名称和图标（如"DeepL X"、"Google 翻译"）
- 卡片可折叠/展开
- 卡片内显示翻译结果
- 每个卡片有独立的"朗读"和"复制"按钮
  - **朗读功能**：使用系统 TTS（macOS `say`、Windows SAPI、Linux `espeak`），点击播放，播放中按钮变"停止"
  - 朗读该卡片的翻译结果（目标语言）

### Result Window（结果窗口）
统一的结果展示窗口，用于所有翻译和 OCR 场景：
- **OCR Mode**：显示 OCR 识别文本（可编辑）+ "翻译"按钮
- **OCR + Translation Mode**：显示 OCR 识别文本（可编辑）+ 翻译卡片列表
- **Selection Translation Mode**：显示原文（可编辑）+ 翻译卡片列表
- **Input Translation Mode**：显示用户输入的文本（可编辑）+ 翻译卡片列表

**窗口结构**：
- 顶部：可编辑文本区域（根据来源不同，可能是 OCR 结果、剪贴板内容、或空白输入框）
  - 文本区域右侧提供"朗读"按钮，朗读原文（源语言）
- 语言选择控件：源语言下拉框（支持"自动检测"）、目标语言下拉框、交换按钮
- 翻译按钮
- 下方：翻译卡片列表（垂直布局，每个激活的 Translation Provider 一个卡片）

**语言检测和翻译方向**：
- **源语言**：默认"自动检测"，使用本地轻量库（如 `lingua` 或 `franc`）检测
- **目标语言**：智能选择
  - 检测到中文 → 翻译成英文
  - 检测到非中文 → 翻译成中文
- **用户控制**：Result Window 顶部提供语言选择下拉框，用户可手动调整源语言和目标语言，调整后点击"翻译"按钮重新翻译

**窗口行为**：
- **位置**：默认屏幕中央，用户可拖动，记住最后拖动的位置
- **关闭方式**：
  - 按 ESC 键关闭
  - 点击窗口外关闭（默认行为）
  - 窗口内提供"保持打开"开关，勾选后点击窗口外不关闭
- **持久性**：窗口保持打开，允许用户编辑文本后重新点击"翻译"按钮

**重要**：所有模式共用同一个窗口组件，仅数据源和初始状态不同。

## Main Window（主窗口）

应用有一个传统主窗口（Settings Window），左侧导航栏包含以下一级标签页：

### 📸 截图 tab（二级导航）
- **快捷键**：截屏、截屏并自动复制、自定义截屏、贴图、贴图管理等
- **保存设置**：默认路径、格式、质量、文件命名规则
- **编辑器**：默认工具颜色、粗细、字体、贴图窗口设置
- **收藏夹**：收藏的截图（用户主动收藏，永久保存）

### 🌐 翻译 tab（二级导航）
- **快捷键**：划词翻译、截图翻译、输入翻译、显示翻译窗口
- **翻译设置**：源/目标语言默认值、多服务显示顺序、自动复制译文
- **历史记录**：自动记录所有翻译操作（可过滤：全部/划词/截图/输入）
- **收藏夹**：收藏的翻译（带标签、笔记）

### 🔍 OCR tab（二级导航）
- **快捷键**：截图 OCR、静默截图 OCR、访问选图 OCR、显示 OCR 窗口
- **OCR 设置**：识别语言优先级、结果窗口位置、自动复制识别文本
- **历史记录**：自动记录所有 OCR 操作（带原图缩略图）
- **收藏夹**：收藏的 OCR 结果（带原图）

### 🔌 服务 tab（顶部标签切换）
- **OCR 服务**：OCR Provider 配置和管理（当前激活、卡片列表、添加自定义）
- **翻译服务**：Translation Provider 配置和管理（已激活列表、优先级排序、添加自定义）
- **语音合成**：TTS Provider 配置和管理（当前激活、卡片列表、添加自定义）

### ⚙️ 通用 tab（二级导航）
- **界面**：语言、主题、开机自启
- **应用快捷键**：显示主窗口、退出应用
- **关于**：版本信息、更新检查、开源协议

### 🔧 高级 tab（二级导航）
- **网络**：代理设置、超时时间
- **日志**：日志级别、查看日志文件
- **数据管理**：导出/导入配置、清空历史记录、清除所有数据

## System Tray（系统托盘）

应用同时常驻系统托盘，用户可以通过托盘菜单快速访问功能。

### 托盘菜单
- **截图**（子菜单）
  - 普通截图
  - OCR 截图
  - OCR + 翻译
- **快捷翻译**：弹出输入翻译窗口（等同于 `Option+W` 快捷键）
- **历史记录**：显示历史记录窗口
- **打开主窗口**：显示主窗口
- **关于**：显示应用版本、开源协议等信息
- **退出**：退出应用


## Technical Architecture（技术架构）

### 技术栈
- **框架**：Tauri 2.0（Rust + Web）
- **前端**：React + TailwindCSS
- **后端**：Rust

### 职责分层

**前端（React）**：
- 所有 UI 渲染（截图编辑器、Result Window、设置界面、历史记录等）
- Canvas 绘图（截图编辑工具）
- 用户交互处理

**后端（Rust）**：
- 系统调用（截图、全局快捷键、剪贴板操作）
- 图片处理（合成标注层和原图、保存、格式转换）
- Provider 调用（HTTP 请求到 OCR/翻译 API）
- 语言检测
- 配置管理（读写 `~/.snaplingo/config.json`、系统密钥存储）
- 历史记录存储
- TTS 调用（系统 TTS API）

### Infrastructure Layer（基础设施层）

**LLM 客户端（`infrastructure/llm/`）：**
- `LLMClient` trait：定义 `generate()` 方法，用于生成翻译
- `LlmModelLister` trait：定义 `list_models()` 方法，用于列举端点可用模型
- Interface segregation：`LLMTranslationProvider` 只依赖 `LLMClient`，不需要 `list_models()` 能力
- 实现：`OpenAILLMClient`、`AnthropicLLMClient`、`GeminiLLMClient`
- 每个客户端封装自己的协议细节：请求头构造、错误处理、响应解析

**Keychain（`infrastructure/storage/keychain/`）：**
- 使用 `Box<dyn KeychainBackend>` trait object 实现平台抽象
- 提供 `with_backend()` 测试构造函数支持依赖注入
- `StubKeychainBackend` 用于单元测试，避免真实 keychain 操作
- 平台实现：macOS (Security framework)、Windows (Credential Manager)、Linux (libsecret)

**HttpClient（`infrastructure/http/`）：**
- `HttpClient` trait：定义 GET/POST 方法
- `ReqwestHttpClient`：基于 reqwest 的生产实现
- 支持 mock 实现用于测试

## History（历史记录）

历史记录按功能域独立管理，分别在"截图"、"翻译"、"OCR"标签页下。

### 翻译历史记录
位置：主窗口 → 翻译 tab → 历史记录

记录内容：
- 时间戳
- 操作类型（划词翻译 / 截图翻译 / 输入翻译）
- 源文本
- 翻译结果（所有激活的 Provider 的结果）
- 源语言和目标语言

功能：
- 过滤：全部 / 划词 / 截图 / 输入
- 搜索：按源文本或译文搜索
- 删除：单条删除或批量删除
- 收藏：将历史记录添加到收藏夹

### OCR 历史记录
位置：主窗口 → OCR tab → 历史记录

记录内容：
- 时间戳
- 操作类型（截图 OCR / 静默 OCR / 访问选图 OCR）
- 识别文本
- 原图缩略图（可选，用户可在设置中关闭）
- 识别语言

功能：
- 过滤：全部 / 截图OCR / 访问选图OCR
- 搜索：按识别文本搜索
- 删除：单条删除或批量删除
- 收藏：将历史记录添加到收藏夹

### 截图历史记录
截图功能**不记录历史**，因为截图要么保存到文件，要么复制到剪贴板，不需要应用内历史记录。

用户如需保留特定截图，可以：
1. 保存到文件系统（通过"保存设置"配置的默认路径）
2. 主动添加到收藏夹

### 清理策略
自动清理机制（用户可在各自 tab 的设置中配置是否启用），满足任一条件时触发：
- 超过配置的天数（默认 30 天）
- 记录总数超过配置的数量（默认 1000 条），删除最旧的

用户可在各自 tab 的二级导航中手动清空历史记录。

收藏夹内容不受自动清理影响。

## Hotkeys（快捷键）

### 快捷键分类

快捷键按功能域分组配置，分别在主窗口的"截图"、"翻译"、"OCR"标签页下的"快捷键"二级导航中设置。

#### 截图相关（全局快捷键）
- **截屏**: `F1` - 普通截图，进入编辑模式
- **截屏并自动复制**: `⌘F1` - 截图后自动复制到剪贴板
- **自定义截屏**: `⇧F1` - 自定义截图区域
- **贴图**: `F3` - 将剪贴板内容创建为置顶浮动窗口
- **隐藏/显示所有贴图**: `⇧F3` - 切换所有贴图的显示状态
- **切换到另一贴图组**: `⌘F3` - 在多个贴图组之间切换

#### 截图编辑器内快捷键（仅在编辑器活跃时）
这些快捷键在截图编辑器全屏覆盖时生效，不需要修饰键：
- **矩形**: `R`
- **椭圆**: `O`
- **箭头**: `A`
- **画笔**: `P`
- **文字**: `T`
- **马赛克**: `M`
- **撤销**: `⌘Z` / `Ctrl+Z`
- **完成截图**: `Enter` / `Space`

#### 翻译相关（全局快捷键）
- **划词翻译**: `⌥D` (Mac) / `Alt+D` (Win) - 选中文字后触发翻译（默认值，可自定义）
- **截图翻译**: `⌥S` (Mac) / `Alt+S` (Win) - 截图区域 → OCR → 自动翻译（默认值，可自定义）
- **输入翻译**: `⌥A` (Mac) / `Alt+A` (Win) - 清空翻译窗口内容并显示，用于手动输入（默认值，可自定义）
- **显示翻译窗口**: 默认未设置 - 直接显示翻译窗口，用于查看之前的翻译结果

#### Result Window 内快捷键（仅在翻译窗口打开时）
- **关闭窗口**: `ESC`
- **复制原文**: `⌘C` / `Ctrl+C`
- **复制译文**: `⌘⇧C` / `Ctrl+Shift+C`
- **语言交换**: `⌘E` / `Ctrl+E`
- **重新翻译**: `⌘R` / `Ctrl+R`

#### OCR 相关（全局快捷键）
- **截图 OCR**: `⇧⌥S` (Mac) / `Shift+Alt+S` (Win) - 截图区域 → 自动 OCR → 显示识别结果（默认值，可自定义）
- **静默截图 OCR**: 默认未设置 - 后台识别图片，自动将 OCR 结果拷贝到剪切板，不显示 OCR 窗口
- **访问选图 OCR**: 默认未设置 - 通过文件选择器选择图片进行 OCR
- **显示 OCR 窗口**: 默认未设置 - 直接显示 OCR 窗口

#### 应用相关（全局快捷键）
- **显示主窗口**: `⌘,` (Mac) / `Ctrl+,` (Win) - 打开设置窗口
- **退出应用**: `⌘Q` (Mac) / `Alt+F4` (Win)

### 快捷键录制和冲突检测
- 每个快捷键输入框旁有"录制"按钮，点击后弹出录制对话框，用户按下新的快捷键组合确认
- 提供"检测冲突"按钮，列出与系统或其他应用冲突的快捷键，并给出建议
- 快捷键注册失败时，在设置界面显示警告："快捷键已被占用，请重新设置"
- 该功能的快捷键显示为"未设置"状态，功能不可用直到用户设置有效快捷键
- 用户可在设置中自定义所有快捷键

## Architecture Decisions

### TranslationRequest 不包含 provider 字段
Translation 请求不指定具体的 Provider。所有请求都发送到当前激活的所有 Providers 以便用户比较结果。这是设计决策，不是技术限制。

**理由：**
- 用户需求是"比较多个翻译"，而非"选择一个翻译"
- 并发调用所有 Providers 符合使用场景
- 如果以后需要单 Provider 路由，应该作为新的 API 设计（例如 `translate_with_provider()`），而非在现有请求中添加可选字段

**历史：** 2026-06-13 删除了未使用的 `provider: String` 字段（从未被读取，造成混淆）。

## Architecture Patterns

### Coordinator Pattern（协调器模式）
用于管理和协调多个同类 Provider 的架构模式。

**结构：**
```rust
pub struct TranslationCoordinator {
    providers: HashMap<String, Arc<dyn TranslationProvider>>, // 不可变，无锁
    active: Arc<Mutex<Vec<String>>>,  // 可变，细粒度锁
    config: Arc<ConfigFile>,           // 持久化
}
```

**生命周期：**
1. **构建阶段**（`&mut self`）：
   - 调用 `register()` 注册所有 Providers
   - 条件性注册（根据 API key 是否存在）
   - 调用 `restore_from_config()` 恢复激活状态

2. **使用阶段**（`Arc<Self>`）：
   - 包装为 `Arc` 后变为不可变共享
   - 所有方法使用 `&self`
   - 内部通过 `Mutex` 保护可变状态

**并发特性：**
- `translate()` 可以并发调用（多个请求同时处理）
- `activate()` 可以并发调用（不同用户同时修改配置）
- 锁粒度小（只锁 `active` 列表，不锁整个 Coordinator）

**适用场景：**
- 需要管理多个同类实现（Provider）
- 需要动态激活/停用（运行时配置）
- 需要并发执行或并发配置
- 需要持久化状态
