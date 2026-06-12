# Domain Language

## Project Name

**SnapLingo** - Snap（截图）+ Lingo（语言）

跨平台截图、OCR、翻译工具。

## Core Concepts

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
- PaddleOCR（免费，本地，中文优化）
- 百度 OCR（需 API Key）
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

## System Tray（系统托盘）

应用常驻系统托盘，没有传统主窗口，仅通过托盘菜单和各功能触发的浮动窗口交互。

### 托盘菜单
- **截图**（子菜单）
  - 普通截图
  - OCR 截图
  - OCR + 翻译
- **快捷翻译**：弹出输入翻译窗口（等同于 `Option+W` 快捷键）
- **历史记录**：显示历史记录窗口
- **设置**：打开设置窗口
- **关于**：显示应用版本、开源协议等信息
- **退出**：退出应用

## Settings（设置）

设置窗口采用左侧分类导航 + 右侧配置项的布局。

### 通用设置
- 开机自启
- 界面语言
- 主题（浅色/深色/跟随系统）

### 截图设置
- 默认保存路径
- 图片格式（PNG/JPG/WebP）
- 图片质量
- 默认工具颜色和粗细

### OCR 设置
- 选择和配置 OCR Provider
- OCR Provider 激活（单选）
- 各 Provider 的 API Key 和参数配置

### 翻译设置
- 选择和配置 Translation Provider
- Translation Provider 激活（多选）
- 各 Provider 的 API Key 和参数配置
- 自定义 Translation Provider 管理
- 默认目标语言

### 快捷键设置
- 自定义所有快捷键
- 冲突检测

### 历史记录设置
- 选择记录哪些内容（Screenshot / OCR / Translation）
- 自动清理配置（启用/禁用、天数、条数）

### 高级设置
- 网络代理设置
- 日志级别
- 清除所有数据

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

## History（历史记录）

### 记录范围
用户可配置记录哪些内容：
- Screenshot Mode 的保存操作（可选）
- OCR Mode 的识别结果（可选）
- OCR + Translation Mode 的识别和翻译结果（可选）
- Selection Translation Mode 的翻译结果（可选）

### 记录结构
每次操作生成一条历史记录，包含：
- 时间戳
- 操作类型（Screenshot / OCR / OCR+Translation / Selection Translation）
- 相关数据（图片缩略图、OCR 文本、翻译结果等，取决于操作类型）

### 清理策略
自动清理机制（用户可配置是否启用），满足任一条件时触发：
- 超过 30 天的记录（天数可配置）
- 记录总数超过 1000 条（数量可配置），删除最旧的

用户可在设置中手动清空历史记录。

## Hotkeys（快捷键）

### 默认快捷键
- **Screenshot Mode**: `F1` (与 Snipaste 一致)
- **OCR Mode**: `Option+A` (Mac) / `Alt+A` (Win/Linux)
- **OCR + Translation Mode**: `Option+S` (Mac) / `Alt+S` (Win/Linux)
- **Selection Translation Mode**: `Option+D` (Mac) / `Alt+D` (Win/Linux)
- **Input Translation Mode**: `Option+W` (Mac) / `Alt+W` (Win/Linux) - 弹出翻译框供用户直接输入文字翻译

### 冲突处理
- 快捷键注册失败时，在设置界面显示警告："快捷键已被占用，请重新设置"
- 该功能的快捷键显示为"未设置"状态，功能不可用直到用户设置有效快捷键
- 提供"检测冲突"按钮，列出系统中已占用的快捷键
- 用户可在设置中自定义所有快捷键
