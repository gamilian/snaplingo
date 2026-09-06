# Easydict、Snapzy 与 SnapLingo 的截图实现比较

> 调研日期：2026-08-19。对比目标是为 SnapLingo 的 Tauri 实现选择截图架构，而不是复刻任一项目的 UI。

## 结论

推荐 SnapLingo 继续采用 **“原生后端一次冻结全部显示器像素 → Tauri/React 叠层负责选区与标注 → 后端从冻结原图裁剪并 OCR”** 的混合方案。它保留了原生截图的权限、DPI 和多屏可靠性，同时不把产品交互绑定到 macOS/AppKit。

不要采用“用户完成选择后再截图”的实现：桌面可能已经变化，且跨屏 DPI、负坐标和叠层自身进入画面的风险都会落在前端时序上。

## 对比依据

以下结论来自两个项目的第一方源码、固定在 2026-08-19 检出的提交：

- Easydict [`eaff88b4`](https://github.com/tisfeng/Easydict/tree/eaff88b4e981714c127b62aa8068f8ba5587cfcf)。
- Snapzy [`bf8ca39f`](https://github.com/duongductrong/Snapzy/tree/bf8ca39f8e13915b203b9aa3c73066b75f94284f)。

| 维度 | Easydict | Snapzy | 对 SnapLingo 的取舍 |
| --- | --- | --- | --- |
| 抓屏时机 | 叠层创建时为背景抓一次；用户松鼠标后再抓选区。 | 常规区域截图先冻结显示器快照，把它同时用作叠层背景和最终裁剪来源。 | 采用 Snapzy 的冻结语义，避免两次抓取间画面变化。 |
| 抓屏技术 | `CGDisplayCreateImage`，`NSScreen.backingScaleFactor` 把点坐标放大后裁剪。 | 主要用 ScreenCaptureKit；macOS 14+ 用 `SCScreenshotManager`，13 使用单帧 `SCStream`；快速路径可用 CoreGraphics。 | 保持按平台的 Rust 适配器；不把 ScreenCaptureKit 当跨平台公共接口。 |
| 多屏/DPI | 每个 `NSScreen` 一个叠层和状态，选区提交给单个屏幕。 | 每显示器冻结帧；裁剪时点坐标→像素对齐、翻转 Y，跨屏选区合成为一张图。 | SnapLingo 应继续以物理像素快照和显示器几何为真源。 |
| 叠层 | SwiftUI 覆盖在每个 `screenSaver` 级 AppKit 窗口上。 | 每屏一个非激活 `NSPanel`，`sharingType = .none`，`screenSaver` 级、全 Space；选择窗口不会写入捕获。 | Tauri 保留前端 Canvas/React 交互，macOS 窗口适配器应维持非激活、全 Space、不可被抓取的策略。 |
| 权限 | 首次触发时 `CGPreflightScreenCaptureAccess`，再单次 `CGRequestScreenCaptureAccess`；之后引导系统设置。 | 先预检；以 `SCShareableContent.current` 触发系统授权，失败回退 `CGRequestScreenCaptureAccess` 并打开设置。 | 借鉴 Snapzy 的分版本授权与可恢复引导；Accessibility 只为增强功能申请。 |
| OCR 图像 | 截图结果直接交给 Apple Vision OCR；识别前将输入写为 PNG 调试文件。 | OCR 在鼠标抬起时先锁定所触及显示器的像素、再关闭叠层；默认 Vision，亦可将 JPEG base64 发至用户配置的远端端点。 | OCR 默认使用未标注的冻结 PNG/位图；远端 OCR 必须明确告知会上传截图。 |

### Easydict：轻量的“选完再抓”实现

`Screenshot.startCapture` 先检查 Screen Recording 权限，随后为每个 `NSScreen` 创建全屏无边框窗口。选区 view 初始化时用 `screen.takeScreenshot()` 取得背景；拖动结束后，`performScreenshot` 重置状态、延迟 0.1 秒，并再次调用 `screen.takeScreenshot(rect:)`，最后才关闭叠层。参见 [`Screenshot.swift`](https://github.com/tisfeng/Easydict/blob/eaff88b4e981714c127b62aa8068f8ba5587cfcf/Easydict/Swift/Feature/Screenshot/Screenshot/Screenshot.swift)、[`ScreenshotOverlayView.swift`](https://github.com/tisfeng/Easydict/blob/eaff88b4e981714c127b62aa8068f8ba5587cfcf/Easydict/Swift/Feature/Screenshot/Screenshot/ScreenshotOverlayView.swift)。

实际抓屏通过 `CGDisplayCreateImage` 获取整块显示器图像，再按 `backingScaleFactor` 从点坐标裁剪；这是针对 Retina 的正确基础，但该路径的选区不会跨显示器合成。参见 [`NSScreen+Extention.swift`](https://github.com/tisfeng/Easydict/blob/eaff88b4e981714c127b62aa8068f8ba5587cfcf/Easydict/Swift/Feature/Screenshot/Screenshot/NSScreen%2BExtention.swift)。

这套方案代码短、对“截图后立即翻译”足够直接，但背景预览和最终图不是同一帧；动态内容、悬浮提示或窗口动画可造成所见与 OCR 输入不一致。它是 SnapLingo 不应复制的核心点。

### Snapzy：冻结会话与复杂原生叠层

`FrozenAreaCaptureSession` 保存每个显示器的 `CGImage`、屏幕 frame 和实际像素比例；它在裁剪时处理相对坐标、Y 轴翻转、像素对齐，并为跨屏选区合成输出。参见 [`FrozenAreaCaptureSession.swift`](https://github.com/duongductrong/Snapzy/blob/bf8ca39f8e13915b203b9aa3c73066b75f94284f/Snapzy/Services/Capture/FrozenAreaCaptureSession.swift)。

`ScreenCaptureManager.captureDisplaySnapshots` 并发抓取目标显示器，按实际返回图像尺寸重新计算比例；`CaptureViewModel.startFrozenAreaSelection` 将这些快照同时传给叠层背景和最终裁剪。这使选择期间的界面与保存结果严格对应。参见 [`ScreenCaptureManager.swift`](https://github.com/duongductrong/Snapzy/blob/bf8ca39f8e13915b203b9aa3c73066b75f94284f/Snapzy/Services/Capture/ScreenCaptureManager.swift) 与 [`CaptureViewModel.swift`](https://github.com/duongductrong/Snapzy/blob/bf8ca39f8e13915b203b9aa3c73066b75f94284f/Snapzy/Features/Capture/CaptureViewModel.swift)。

其 `AreaSelectionWindow` 是非激活 `NSPanel`，设为 `sharingType = .none`、`.screenSaver` 级、加入全部 Space；因此叠层既可覆盖全局，也不会烘焙进截图。其“实时穿透”模式用全局 event tap 保住原有 hover UI；此模式需要 Accessibility，未授权时退回普通窗口事件。参见 [`AreaSelectionWindow.swift`](https://github.com/duongductrong/Snapzy/blob/bf8ca39f8e13915b203b9aa3c73066b75f94284f/Snapzy/Services/Capture/AreaSelectionWindow.swift)。

Snapzy 的 OCR 是一个刻意不同的语义：为了捕获鼠标抬起时仍在屏幕上的 tooltip/popover，它在 mouse-up 同步冻结相关显示器，之后才关闭叠层并 OCR，而不是在热键触发时冻结。参见 [`CaptureViewModel.captureOCR`](https://github.com/duongductrong/Snapzy/blob/bf8ca39f8e13915b203b9aa3c73066b75f94284f/Snapzy/Features/Capture/CaptureViewModel.swift)。默认 OCR 会把 ScreenCaptureKit 图像标准化为 sRGB 后交给 Vision；可选远端提供方则上传 JPEG base64。参见 [`OCRService.swift`](https://github.com/duongductrong/Snapzy/blob/bf8ca39f8e13915b203b9aa3c73066b75f94284f/Snapzy/Services/Media/OCRService.swift) 与 [`RemoteOCRService.swift`](https://github.com/duongductrong/Snapzy/blob/bf8ca39f8e13915b203b9aa3c73066b75f94284f/Snapzy/Services/Media/OCR/RemoteOCRService.swift)。

## SnapLingo 已验证的实现与含义

SnapLingo 当前实现已具备推荐架构的关键部分：

- `CaptureSessionRuntime::open_capture_window_for_mode` 先创建可见桌面的会话，再打开捕获窗口；失败时会清理会话并结束展示生命周期。见 [`src-tauri/src/application/capture/runtime.rs`](../../src-tauri/src/application/capture/runtime.rs)。
- `CaptureSessions` 将每个显示器的快照保存在会话中；选区渲染、输出和 OCR 都按 `session_id + rect` 读取该会话，而不是重新抓屏。见 [`src-tauri/src/application/capture/session.rs`](../../src-tauri/src/application/capture/session.rs) 与 [`src-tauri/src/application/capture/runtime.rs`](../../src-tauri/src/application/capture/runtime.rs)。
- macOS 适配器先预检并请求 Screen Recording 权限，以 CoreGraphics 抓取显示器图像，做 BGRA → RGBA 转换，再编码 PNG。见 [`src-tauri/src/infrastructure/system/screenshot/macos.rs`](../../src-tauri/src/infrastructure/system/screenshot/macos.rs)。
- 选区叠层仍是 Tauri 窗口，但 macOS 侧会配置原生窗口层级、Space 行为、焦点和十字光标。见 [`src-tauri/src/infrastructure/system/capture_window/macos.rs`](../../src-tauri/src/infrastructure/system/capture_window/macos.rs)。

## 推荐的责任划分

1. **捕获适配器（Rust，按平台）**：请求/预检屏幕录制权限，抓取所有显示器的物理像素及显示器几何。
2. **Capture Session（Rust）**：保存冻结原图，完成逻辑坐标到物理坐标的映射，并提供裁剪、OCR 和输出接口。
3. **Overlay（Tauri/React）**：只处理选择、标注、快捷键和预览；它不拥有截图时序，也不持有 OCR 的权威图像。
4. **OCR**：默认输入未标注的原始选区 PNG/位图；标注图只服务于复制、保存、钉图或导出。PNG 避免 JPEG 有损压缩损害小字识别；用户导出可另行选 JPEG/WebP。若未来加入“捕获 tooltip”模式，应像 Snapzy 一样把它做成显式的 mouse-up 冻结模式，而不是改变默认热键的触发时刻。

## 权限与平台权衡

- macOS 的整屏/窗口像素捕获需要 Screen Recording 权限；拒绝后应给出可见且可恢复的指引。SnapLingo 已通过 `CGPreflightScreenCaptureAccess` / `CGRequestScreenCaptureAccess` 实现这一点。[macOS capture adapter](../../src-tauri/src/infrastructure/system/screenshot/macos.rs)
- 自由矩形选区不需要 Accessibility。仅“智能吸附窗口/控件”需要它，且应该是可降级的增强功能，不能阻断普通截图。
- 多显示器必须以物理像素快照为真源；前端使用显示器缩放信息换算坐标。不要让 DOM/CSS 像素直接成为裁剪坐标。

## 最终建议

不要二选一地“照搬 Easydict 或 Snapzy”。对 Tauri SnapLingo，采纳 **Snapzy 的冻结会话、跨屏像素合成和叠层排除原则**，但保持 SnapLingo 当前的后端跨平台适配器与 Web 前端选区；不要引入 Snapzy 为录屏、滚动截图、实时穿透和 Smart Element 所承担的大量 AppKit 状态机。

Easydict 的路径适合做 MVP，却会把“预览帧”和“OCR/输出帧”分开；SnapLingo 已有的冻结 Session 正好消除了这个问题。只有产品明确要捕获 hover/tooltip 时，才单独引入 Snapzy 式 mouse-up 冻结，并把 Accessibility 作为可选能力而非普通矩形截图的前置条件。
