# SnapLingo - 更新后的产品需求文档 (2026-06-15)

## 项目概述

SnapLingo 是一个跨平台桌面应用（基于 Tauri 2.0），整合了截图、OCR 和翻译功能。

### 技术栈（已实现）
- **前端**: React 18 + TypeScript + TailwindCSS + Zustand (状态管理)
- **后端**: Rust (Tauri 2.0)
- **架构**: DDD 分层架构 (Domain/Application/Infrastructure/Commands)

---

## 核心功能模块

### 1. 五种捕获模式 (Capture Modes)

通过全局快捷键触发：

1. **Screenshot Mode** - `F1`
   - 框选截图 → 编辑（标注、形状、文字、模糊等）
   - 完成后：保存 / 复制 / 贴图（置顶浮窗）
   - 编辑器中可点击 OCR 按钮识别原图文字

2. **OCR Mode** - `Shift+Alt+S`
   - 框选截图 → 自动 OCR → 显示识别文本
   - 用户可手动点击"翻译"按钮

3. **OCR + Translation Mode** - `Ctrl+Shift+C`
   - 框选截图 → OCR → 自动翻译
   - 结果渐进显示：先文本，后翻译

4. **Selection Translation Mode** - `Ctrl+Shift+T`
   - 选中任意应用中的文字 → 按快捷键
   - 自动复制并翻译选中内容
   - **重要**: 无选中文本时快捷键穿透到系统

5. **Input Translation Mode** - `Ctrl+Shift+T`
   - 打开翻译窗口 → 手动输入文本 → 翻译

---

### 2. Provider 系统（已实现）

**架构模式**: Coordinator Pattern
- 所有 Provider 内置（非插件系统）
- 用户通过设置界面激活/配置

#### OCR Providers（单选）
- ✅ **Tesseract** (本地，免费)
- ✅ **Baidu OCR** (API Key)
- 🚧 PaddleOCR (计划中)
- 🚧 Tencent/Google/Azure OCR (计划中)

#### Translation Providers（多选，并发翻译）
- ✅ **Google Translate** (免费，无需 API Key)
- ✅ **DeepL** (API Key)
- ✅ **Baidu Translate** (App ID + Secret Key)
- ✅ **自定义 LLM Providers** (支持 OpenAI/Anthropic/Gemini 协议)
- 🚧 Youdao/Tencent/Azure (计划中)

**配置存储**:
- Provider Endpoint、Base URL、API Key 与其他应用设置：本机 SQLite (`snaplingo.db`)
- 当前小范围测试版本接受 API Key 在 SQLite 中明文保存

---

### 3. 主窗口结构（Settings Window）

采用**双层导航**结构：

#### 左侧主导航
- 📸 **截图 (Screenshot)**
- 🌐 **翻译 (Translation)**
- 🔍 **OCR**
- 🔌 **服务 (Services)** - 顶部标签切换（OCR/翻译/TTS）
- ⚙️ **通用 (General)**
- 🔧 **高级 (Advanced)**

#### 二级导航（部分主 tab 有）
**截图 tab**:
- 快捷键 / 保存设置 / 编辑器 / 收藏夹

**翻译 tab**:
- 快捷键 / 翻译设置 / 历史记录 / 收藏夹

**OCR tab**:
- 快捷键 / OCR 设置 / 历史记录 / 收藏夹

**服务 tab** (顶部标签):
- OCR 服务 / 翻译服务 / 语音合成

---

### 4. 翻译结果窗口（Result Window）

**统一的浮动窗口**，用于所有翻译场景：

#### 结构
1. **源文本区域** (可编辑)
   - 带朗读按钮（系统 TTS）
2. **语言选择控件**
   - 源语言下拉框（支持"自动检测"）
   - 目标语言下拉框
   - 交换按钮
3. **翻译按钮**
4. **翻译卡片列表**（垂直堆叠）
   - 每个激活的 Provider 一张卡片
   - 卡片显示：Provider 名称/图标 + 翻译结果
   - 每卡片独立：复制按钮 + TTS 按钮
   - 可折叠/展开

#### 窗口行为
- 位置：默认屏幕中央，记住最后拖动位置
- 关闭：ESC 键 / 点击窗口外（默认）
- "保持打开"开关：勾选后点击外部不关闭
- 持久性：窗口保持打开，允许编辑后重新翻译

---

### 5. 历史记录系统（已实现）

#### Translation History
- 记录内容：时间戳 / 操作类型 / 源文本 / 翻译结果 / 语言对
- 功能：过滤（划词/截图/输入）/ 搜索 / 删除 / 收藏
- 存储：SQLite 数据库

#### OCR History
- 记录内容：时间戳 / 操作类型 / 识别文本 / 原图缩略图（可选）
- 功能：过滤 / 搜索 / 删除 / 收藏

#### 清理策略
- 自动清理（可配置）：超过 30 天或 1000 条记录
- 收藏夹内容不受清理影响

---

### 6. 截图编辑器（计划中）

**Snipaste 同等功能**：

#### 绘图工具
- 矩形 / 椭圆 / 箭头 / 折线（Shift 约束）
- 画笔 / 马克笔（半透明高亮）
- 文字标注（字体/大小/颜色可调）
- 马赛克 / 高斯模糊 / 橡皮擦
- 撤销/重做栈

#### 双层架构
- **原始截图层**：用于 OCR（不含标注）
- **标注层**：用于显示和保存

#### 完成操作
- 保存到文件（PNG/JPG/WebP）
- 复制到剪贴板
- 贴图（创建置顶浮窗）
- OCR 按钮（识别原图）

---

## 已实现的核心架构

### DDD 分层

```rust
// Domain Layer (领域层)
- 核心实体：Translation, OCR, Hotkey, Config
- 事件系统：TranslationCompleted, OcrCompleted

// Application Layer (应用层)
- TranslationCoordinator (协调器模式)
- OcrCoordinator
- CaptureService, HotkeyService, HistoryService
- WorkflowService (编排五种 Capture Mode)

// Infrastructure Layer (基础设施层)
- SqliteConfigStore (配置存储)
- SqliteCredentialStore (Provider 凭据存储)
- HistoryDatabase (SQLite)
- HttpClient (reqwest)
- LLMClient (OpenAI/Anthropic/Gemini)
- EventBus (事件总线)

// Commands Layer (命令层)
- Tauri IPC Commands
```

### 前端状态管理（Zustand）

```typescript
// stores/appStore.ts - 应用全局状态
- resultWindowVisible
- sourceText, sourceLang, targetLang
- translations[], isTranslating

// stores/settingsStore.ts - 设置界面导航
- activeMainTab, screenshotSubTab, translationSubTab, ocrSubTab

// stores/providerStore.ts - Provider 状态
- ocrProviders[], translationProviders[]
- activeOcrProvider, activeTranslationProviders[]

// stores/historyStore.ts - 历史记录
- translationHistory[], ocrHistory[]
```

---

## 系统托盘（计划中）

常驻托盘菜单：
- 截图 (子菜单：普通/OCR/OCR+翻译)
- 快捷翻译
- 历史记录
- 打开主窗口
- 关于
- 退出

---

## 当前开发阶段

### ✅ 已完成
- [x] Rust 后端架构（DDD 分层）
- [x] Translation Coordinator + 3 个 Provider (Google/DeepL/Baidu)
- [x] OCR Coordinator + 2 个 Provider (Tesseract/Baidu)
- [x] 自定义 LLM Provider 支持
- [x] 历史记录系统（SQLite + 事件驱动）
- [x] 配置管理 + 系统密钥存储
- [x] 全局热键系统（基础实现）
- [x] Workflow Service（编排五种模式）
- [x] 前端设置窗口（双层导航结构）
- [x] 翻译结果窗口（Result Window）

### 🚧 进行中
- [ ] 截图编辑器（Canvas 绘图工具）
- [ ] 系统托盘集成
- [ ] 贴图窗口（置顶浮窗）
- [ ] 跨平台截图后端（Windows/Linux）

### 📋 计划中
- [ ] TTS 集成（系统 TTS）
- [ ] 收藏夹功能
- [ ] 代理配置
- [ ] 自动更新
- [ ] 应用打包和分发

---

## UI/UX 设计需求

### 当前状态
- ✅ 基础布局已实现（TailwindCSS）
- ✅ 设置窗口：左侧导航 + 二级导航 + 内容区
- ✅ 翻译结果窗口：模态浮窗 + 卡片式结果
- ❌ 缺少统一的视觉语言和设计系统
- ❌ 缺少品牌识别（图标/配色/字体）
- ❌ 截图编辑器 UI 未设计

### 设计目标
1. **建立视觉语言**：配色方案 / 字体系统 / 间距规范
2. **优化信息架构**：双层导航是否最优？
3. **改进交互体验**：Provider 配置流程 / 历史记录检索
4. **设计缺失界面**：截图编辑器 / 托盘菜单 / 贴图窗口
5. **暗色模式**：全局主题切换

---

## 技术约束

### 必须保持
- Tauri 2.0 窗口管理模型
- React + TailwindCSS 技术栈
- 现有状态管理结构（Zustand）

### 可以调整
- 配色方案
- 字体选择
- 组件样式
- 布局结构（在 Tauri 约束内）

---

## 参考应用

**功能对标**:
- Snipaste (截图编辑)
- Bob (macOS OCR+翻译)
- ShareX (Windows 截图工具)

**视觉参考**:
- macOS Big Sur 系统应用
- Linear (现代工具型应用)
- Figma (专业工具)

---

## 关键决策记录

1. **为什么不用插件系统？**
   - 简化实现，所有 Provider 内置
   - 自定义 Provider 通过标准 API 格式覆盖扩展性

2. **为什么多选翻译 Provider？**
   - 核心差异化功能
   - 翻译质量差异大，用户需要对比

3. **为什么双层导航？**
   - 功能模块多（截图/OCR/翻译/服务），需要清晰分组
   - 每个模块下有多个子功能（快捷键/设置/历史/收藏）

4. **为什么选择 Tauri 而非 Electron？**
   - 包体积小 (~10MB vs ~150MB)
   - 性能和内存占用更优
   - Rust 天然适合系统级操作（截图/热键）

---

## 下一步工作

### 短期（MVP）
1. 完成截图编辑器 UI 和交互
2. 实现系统托盘和菜单
3. 完善 Provider 配置流程
4. 优化历史记录检索体验

### 中期
1. 跨平台适配（Windows/Linux）
2. 暗色模式
3. 应用打包和自动更新
4. 性能优化和稳定性

### 长期
1. 高级 OCR（表格/公式识别）
2. 云端 TTS
3. 更多内置 Provider
4. 浏览器插件集成
