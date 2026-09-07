# Easydict、Snapzy 与 SnapLingo 的截图、OCR、翻译实现对比

> 调研日期：2026-09-07。上游源码固定于 Easydict `dev` 提交 [`0bbbd4f7`](https://github.com/tisfeng/Easydict/tree/0bbbd4f7cdc40f160e5e54ceee9e200ae757b05e) 和 Snapzy `master` 提交 [`628a9f77`](https://github.com/duongductrong/Snapzy/tree/628a9f777830abee1c8a7391855fc7f4b22f5e61)。结论只基于上游源码、上游仓库文档和当前工作区源码，不修改业务代码。

## 结论摘要

- **截图时序应以 Snapzy 为基准。** Snapzy 使用冻结显示器快照、跨屏像素裁剪和 mouse-up frame lock；Easydict 在选区结束后再次抓屏，背景预览帧与最终 OCR/输出帧可能不一致。SnapLingo 已有 Capture Session 的冻结方向，应继续把“选区 UI”与“截图真源”分离。
- **OCR 质量应同时吸收两者的文本处理。** Snapzy 解决图像输入、语言 profile、低置信度重试、对比度和竖排 CJK；Easydict 解决版面排序、段落合并、语言相关标点和 URL/数字/代码保护。SnapLingo 当前 macOS Vision 路径只取每行 `topCandidates(1)` 并拼接字符串，结果模型也只有 `text` 与整体 `confidence`，这是最明显的质量差距。
- **翻译链路主要参考 Easydict。** Easydict 提供 OCR、语言检测、翻译服务和流式取消的组合；源码还定义了“检测语言后再 OCR 一次”的 `deepOCR` 算法，但当前提交没有找到调用点，不能把它当作默认生效行为。Snapzy 没有翻译引擎、目标语言选择或翻译结果 UI，OCR 结果只是复制到剪贴板。因此不要从 Snapzy 推断翻译能力，应保留 SnapLingo 自己的多 Provider 设计。
- **优先级建议：** 先补 OCR 结构化结果和恢复策略，再补 OCR→语言检测→翻译的可取消状态机，最后做 QR/链接/通知等捕获后增强。不要为了普通矩形截图默认引入 Accessibility 或 Snapzy 的完整录屏/标注栈。

## 能力边界

| 维度 | Easydict | Snapzy | 对 SnapLingo 的含义 |
| --- | --- | --- | --- |
| 区域截图 | 每屏 SwiftUI/AppKit overlay；拖动结束后再次 `CGDisplayCreateImage` 裁剪 | 冻结快照、跨屏 composite、DPI reconciliation；live 模式在 mouse-up 锁帧 | 保留 SnapLingo 的 Rust/Tauri 会话，明确 frozen 与 live 两种语义 |
| OCR | Apple Vision；可选 Youdao/Baidu OCR；有文本清洗和语言重试 | Apple Vision 多 profile/recovery；可选 OpenAI-compatible remote OCR | 增加 profile、低质量重试、结构化行信息和隐私提示 |
| 翻译 | 多 Provider 并发、流式结果、自动语言检测、Apple 离线翻译 | 无翻译服务；OCR 只写剪贴板 | 翻译架构以 Easydict 为参照，Snapzy 只作为截图/OCR工程参照 |
| 结果反馈 | OCR 可静默复制或打开 OCR/翻译窗口；截图翻译可自动查询 | OCR spinner、原生通知、toast fallback、链接提示、QR 合并 | 保证 OCR/翻译反馈不阻塞捕获，并区分“全文剪贴板”和“结果窗口” |

## Easydict：从截图到翻译

### 截图 overlay 与时序

`Screenshot.startCapture` 先预检/请求 Screen Recording 权限，为每个 `NSScreen` 创建 screen-saver 级窗口；选区结束后 `performScreenshot` 将状态重置，延迟 0.1 秒，再调用 `screen.takeScreenshot(rect:)`，然后关闭 overlay。见 [`Screenshot.swift#L25-L52`](https://github.com/tisfeng/Easydict/blob/0bbbd4f7cdc40f160e5e54ceee9e200ae757b05e/Easydict/Swift/Feature/Screenshot/Screenshot/Screenshot.swift#L25-L52) 和 [`Screenshot.swift#L97-L116`](https://github.com/tisfeng/Easydict/blob/0bbbd4f7cdc40f160e5e54ceee9e200ae757b05e/Easydict/Swift/Feature/Screenshot/Screenshot/Screenshot.swift#L97-L116)。背景在 SwiftUI view 初始化时先抓一帧，见 [`ScreenshotOverlayView.swift#L1-L18`](https://github.com/tisfeng/Easydict/blob/0bbbd4f7cdc40f160e5e54ceee9e200ae757b05e/Easydict/Swift/Feature/Screenshot/Screenshot/ScreenshotOverlayView.swift#L1-L18)。

实际裁剪先对整块显示器调用 `CGDisplayCreateImage`，再用 `backingScaleFactor` 将点坐标放大到像素坐标，见 [`NSScreen+Extention.swift#L11-L48`](https://github.com/tisfeng/Easydict/blob/0bbbd4f7cdc40f160e5e54ceee9e200ae757b05e/Easydict/Swift/Feature/Screenshot/Screenshot/NSScreen%2BExtention.swift#L11-L48)。这对单显示器 Retina 截图足够直接，但没有 Snapzy 的跨屏合成和同一帧保证；这是 SnapLingo 不应复制的时序缺点。

### OCR 入口、静默模式与翻译模式

Easydict 明确分开三种用户意图：

- `snipTranslate` 截图后把图片送入查询窗口，根据 `autoQueryOCRText` 决定是否自动翻译。
- `silentScreenshotOCR` 不显示窗口，OCR 后只复制文本并给成功 toast。
- `screenshotOCR` 打开 OCR 结果窗口，允许用户查看 OCR 文本。

这些入口和行为见 [`EZWindowManager.m#L861-L904`](https://github.com/tisfeng/Easydict/blob/0bbbd4f7cdc40f160e5e54ceee9e200ae757b05e/Easydict/objc/ViewController/Window/WindowManager/EZWindowManager.m#L861-L904)。OCR 结果进入 `startOCRImage` 后会显示加载状态、写入输入框、显示自动检测到的语言；发生错误时保留错误提示；用户可配置自动复制 OCR 文本，最后按 `autoQuery` 决定是否开始翻译，见 [`EZBaseQueryViewController.m#L542-L616`](https://github.com/tisfeng/Easydict/blob/0bbbd4f7cdc40f160e5e54ceee9e200ae757b05e/Easydict/objc/ViewController/Window/BaseQueryWindow/EZBaseQueryViewController.m#L542-L616)。

对 SnapLingo 的直接启示是：截图 OCR、截图翻译和静默 OCR 应共享同一张原始图，但拥有不同的后续状态和反馈，不应由 UI 是否打开来隐式决定 OCR 是否成功。

### OCR 与语言检测恢复

`DetectManager.ocrAndDetectText` 先 OCR，再把 `mergedText` 写入查询模型并应用 OCR 返回的语言；见 [`DetectManager.swift#L58-L79`](https://github.com/tisfeng/Easydict/blob/0bbbd4f7cdc40f160e5e54ceee9e200ae757b05e/Easydict/Swift/Model/DetectManager.swift#L58-L79)。源码另有 `deepOCR`：它计划在用户没有指定源语言时先用 auto OCR、再检测语言，必要时按检测语言重跑 OCR，见 [`DetectManager.swift#L220-L290`](https://github.com/tisfeng/Easydict/blob/0bbbd4f7cdc40f160e5e54ceee9e200ae757b05e/Easydict/Swift/Model/DetectManager.swift#L220-L290)；但本提交中 `rg` 只找到定义本身，没有调用点，因此应视为未接线的恢复设计。Apple OCR 失败时还能按配置回退到 Youdao OCR，见 [`DetectManager.swift#L317-L350`](https://github.com/tisfeng/Easydict/blob/0bbbd4f7cdc40f160e5e54ceee9e200ae757b05e/Easydict/Swift/Model/DetectManager.swift#L317-L350)。

Easydict 的 OCR 文本处理还会按语言做标点和空白规范化，并保护 URL、域名、邮箱、文件路径、代码、版本号和省略号，见 [`OCRTextNormalizer.swift#L1-L115`](https://github.com/tisfeng/Easydict/blob/0bbbd4f7cdc40f160e5e54ceee9e200ae757b05e/Easydict/Swift/Service/Apple/AppleOCREngine/OCRTextNormalizer.swift#L1-L115)。其测试覆盖英文/中文/韩文/日文标点、数字和受保护内容，见 [`OCRPunctuationTests.swift`](https://github.com/tisfeng/Easydict/blob/0bbbd4f7cdc40f160e5e54ceee9e200ae757b05e/EasydictTests/Feature/OCR/OCRPunctuationTests.swift) 和 [`OCRTextProcessingTests.swift`](https://github.com/tisfeng/Easydict/blob/0bbbd4f7cdc40f160e5e54ceee9e200ae757b05e/EasydictTests/Feature/OCR/OCRTextProcessingTests.swift)。

### OCR 直接翻译与普通翻译

Provider 抽象同时提供 `ocr` 与 `ocrAndTranslate`。以 Youdao 为例，OCR 响应若已带逐行翻译且目标语言匹配，就直接使用 OCR 翻译结果；否则再把合并后的 OCR 文本送到普通 `translate`，见 [`YoudaoService+OCR.swift#L16-L119`](https://github.com/tisfeng/Easydict/blob/0bbbd4f7cdc40f160e5e54ceee9e200ae757b05e/Easydict/Swift/Service/Youdao/YoudaoService%2BOCR.swift#L16-L119)。这避免了同一服务的重复翻译请求，同时允许 Apple Vision OCR + 任意翻译 Provider 的组合。

### 多 Provider、流式结果与取消

`QueryService.startQueryStream` 用 `AsyncThrowingStream` 暴露增量结果，将 Provider 错误和取消转换为终态，并通过 `resultGeneration` 丢弃旧查询的迟到结果，见 [`QueryService.swift#L188-L334`](https://github.com/tisfeng/Easydict/blob/0bbbd4f7cdc40f160e5e54ceee9e200ae757b05e/Easydict/Swift/Service/Model/QueryService.swift#L188-L334)。查询前的 `prehandleQueryText` 还会处理简繁转换、语言不支持和免费额度等不需要发网络请求的结果，见 [`QueryService.swift#L587-L650`](https://github.com/tisfeng/Easydict/blob/0bbbd4f7cdc40f160e5e54ceee9e200ae757b05e/Easydict/Swift/Service/Model/QueryService.swift#L587-L650)。

Apple Provider 在 macOS 15+ 优先调用系统 Translation API，失败或不可用时回退到 AppleScript；同时保留 Vision OCR，见 [`AppleService.swift#L55-L120`](https://github.com/tisfeng/Easydict/blob/0bbbd4f7cdc40f160e5e54ceee9e200ae757b05e/Easydict/Swift/Service/Apple/AppleService.swift#L55-L120)。这说明本地离线翻译可以是一个 Provider，而不是写死在 OCR 代码中。

## Snapzy：截图/OCR 工程化基准

### 跨屏、DPI 和 overlay 生命周期

Snapzy 的 `captureArea` 会识别选区触及的显示器，跨屏时并行抓取并复用 `FrozenAreaCaptureSession.cropCompositeImage` 合成；prepared path 会从返回 `CGImage` 的实际尺寸重新计算 scale，避免 scaled display 被错误 clamp 到左上角，见 [`ScreenCaptureManager.swift#L966-L1053`](https://github.com/duongductrong/Snapzy/blob/628a9f777830abee1c8a7391855fc7f4b22f5e61/Snapzy/Services/Capture/ScreenCaptureManager.swift#L966-L1053)、[`ScreenCaptureManager.swift#L1439-L1531`](https://github.com/duongductrong/Snapzy/blob/628a9f777830abee1c8a7391855fc7f4b22f5e61/Snapzy/Services/Capture/ScreenCaptureManager.swift#L1439-L1531) 和 [`ScreenCaptureManager.swift#L1588-L1677`](https://github.com/duongductrong/Snapzy/blob/628a9f777830abee1c8a7391855fc7f4b22f5e61/Snapzy/Services/Capture/ScreenCaptureManager.swift#L1588-L1677)。

overlay 是每屏预分配的非激活 `NSPanel`，`sharingType = .none`，加入全部 Space；旧 session 会通过统一取消路径清理 completion、observer 和窗口。启动后还有 presentation watchdog，按 reassert→recreate→activate-and-recreate 修复 WindowServer 中“看似可见但实际不在当前 Space”的窗口，见 [`AreaSelectionWindow.swift#L501-L628`](https://github.com/duongductrong/Snapzy/blob/628a9f777830abee1c8a7391855fc7f4b22f5e61/Snapzy/Services/Capture/AreaSelectionWindow.swift#L501-L628) 和 [`AreaSelectionWindow.swift#L754-L923`](https://github.com/duongductrong/Snapzy/blob/628a9f777830abee1c8a7391855fc7f4b22f5e61/Snapzy/Services/Capture/AreaSelectionWindow.swift#L754-L923)。

### Live passthrough 与 mouse-up frame lock

live 选区模式让 overlay hit-transparent，通过 `CGEventTap` 消费鼠标/键盘事件，使已经出现的 tooltip、hover card 不收到 mouse-exit；Accessibility 不可用时退回普通 window-event overlay，见 [`CaptureEventTapController.swift#L109-L300`](https://github.com/duongductrong/Snapzy/blob/628a9f777830abee1c8a7391855fc7f4b22f5e61/Snapzy/Services/Capture/CaptureEventTapController.swift#L109-L300) 和 [`docs/CAPTURE.md#L118-L127`](https://github.com/duongductrong/Snapzy/blob/628a9f777830abee1c8a7391855fc7f4b22f5e61/docs/CAPTURE.md#L118-L127)。

OCR capture 在 mouse-up 的同步点抓取所有触及显示器的快照，先关闭 overlay，再异步做裁剪、QR 与 OCR；这样 hover 状态与 OCR 输入对应同一时刻，见 [`CaptureViewModel.swift#L2334-L2527`](https://github.com/duongductrong/Snapzy/blob/628a9f777830abee1c8a7391855fc7f4b22f5e61/Snapzy/Features/Capture/CaptureViewModel.swift#L2334-L2527)。

### Vision OCR 的质量策略

Snapzy 在 Vision 前把 IOSurface 图像绘制成标准 sRGB bitmap，避免 `CRImageReaderError`；随后按内容类型和语言选择 profile，并在低质量时尝试对比度增强、竖排 CJK 转正和 recovery profile，见 [`OCRService.swift#L49-L218`](https://github.com/duongductrong/Snapzy/blob/628a9f777830abee1c8a7391855fc7f4b22f5e61/Snapzy/Services/Media/OCRService.swift#L49-L218)。

每行保留 bounding box 和 confidence，按阅读顺序排序、分段并按 CJK/西文规则 reflow；候选结果按置信度、有效字符数、CJK 比例和语言脚本打分，而不是看到第一个非空结果就结束，见 [`OCRService.swift#L331-L511`](https://github.com/duongductrong/Snapzy/blob/628a9f777830abee1c8a7391855fc7f4b22f5e61/Snapzy/Services/Media/OCRService.swift#L331-L511)。profile 覆盖英文、越南文、西班牙文、俄文、法文、德文、日文、韩文、简繁中文，以及 code/dense-document/recovery，见 [`VisionOCRProfile.swift#L21-L110`](https://github.com/duongductrong/Snapzy/blob/628a9f777830abee1c8a7391855fc7f4b22f5e61/Snapzy/Services/Media/OCR/VisionOCRProfile.swift#L21-L110)。

### Remote OCR、QR 与反馈

Snapzy 的 OCR Provider 只有一个活动选择，可在内置 Vision 与自定义 OpenAI-compatible endpoint 间切换；远端使用 base64 JPEG、60 秒超时、Keychain API key、按 interface/document/code 选择默认 prompt，并区分 401、HTTP、网络和解析错误，见 [`RemoteOCRService.swift#L39-L196`](https://github.com/duongductrong/Snapzy/blob/628a9f777830abee1c8a7391855fc7f4b22f5e61/Snapzy/Services/Media/OCR/RemoteOCRService.swift#L39-L196) 和 [`OCRModelResolver.swift#L40-L122`](https://github.com/duongductrong/Snapzy/blob/628a9f777830abee1c8a7391855fc7f4b22f5e61/Snapzy/Services/Media/OCR/OCRModelResolver.swift#L40-L122)。

QR 与 OCR 并行，去重后合并到剪贴板；检测到最多 3 个 HTTP(S) 链接时显示可点击提示。OCR 通知只展示压缩后的预览，剪贴板保留全文；通知不可用时退回 toast，见 [`docs/CAPTURE.md#L132-L145`](https://github.com/duongductrong/Snapzy/blob/628a9f777830abee1c8a7391855fc7f4b22f5e61/docs/CAPTURE.md#L132-L145)。

**重要边界：** Snapzy master 没有翻译服务、翻译 Provider、源/目标语言状态或翻译窗口；官方文档把 OCR 输出定义为 clipboard text only，见 [`README.zh-CN.md#L60-L69`](https://github.com/duongductrong/Snapzy/blob/628a9f777830abee1c8a7391855fc7f4b22f5e61/README.zh-CN.md#L60-L69) 和 [`docs/CAPTURE.md#L41-L49`](https://github.com/duongductrong/Snapzy/blob/628a9f777830abee1c8a7391855fc7f4b22f5e61/docs/CAPTURE.md#L41-L49)。

## 对 SnapLingo 的改进建议

### P0：捕获正确性

1. **固定同一帧语义。** 普通 OCR/截图翻译使用 Capture Session 的冻结原图；若要保留 tooltip/hover，单独提供 live mouse-up 模式，并在 mouse-up 同步锁定像素后再关闭 overlay。不要在选区结束后重新抓一张“当前屏幕”。
2. **把物理像素和显示器几何作为裁剪真源。** 选区跨屏时按 display ID 分片、处理负坐标与 Y 轴翻转；scale 应从实际返回图像尺寸校准，而不是只信 CSS/Tauri scale。
3. **验证 overlay 排除和生命周期。** macOS 窗口适配器应保持 non-activating、all-spaces、不可被捕获；取消、Esc、重复触发、Space 切换都必须经过同一清理路径。Accessibility 只作为 live passthrough/智能元素增强，普通矩形截图不能依赖它。

### P1：OCR 质量和可诊断性

4. **扩展 OCR 结果模型。** 当前 SnapLingo macOS Vision 适配器在 [`src-tauri/src/infrastructure/system/ocr/macos.rs`](../../src-tauri/src/infrastructure/system/ocr/macos.rs) 中取每个 observation 的第一个候选并拼接行；[`src-tauri/src/domain/ocr.rs`](../../src-tauri/src/domain/ocr.rs) 只暴露 `text` 和整体 `confidence`。建议至少增加 detected language、逐行 text/confidence/bounding box、engine/profile 和 normalization 状态，使阅读顺序、段落重排和用户纠错可实现。
5. **增加成本受控的 recovery pipeline。** 先标准化 sRGB，再按 `interface`/`document`/`code` 与语言选择 Vision profile；低置信度或无结果时才执行对比度增强、语言 recovery、竖排 CJK recovery，并保留“最佳候选”及失败原因。不要每次无条件并发所有语言。
6. **在 OCR→翻译前做安全文本规范化。** 借鉴 Easydict 的 URL、邮箱、路径、版本号、代码和省略号保护，以及按语言处理标点/空白；原始 OCR 文本和规范化文本应分开保存，避免修复破坏复制原文。
7. **做真实图像回归而不只测 mock provider。** 建立多语言、双栏、竖排、低对比度、混合 CJK/拉丁文和代码样本，记录 character accuracy、no-output、平均延迟和 recovery 命中率。Snapzy 的 10 语言样本与跨屏 crop 单测可作为测试形状参考：[`OCRRecognitionTests.swift#L14-L155`](https://github.com/duongductrong/Snapzy/blob/628a9f777830abee1c8a7391855fc7f4b22f5e61/SnapzyTests/Services/Media/OCRRecognitionTests.swift#L14-L155)、[`ScreenCaptureAreaCropTests.swift#L18-L70`](https://github.com/duongductrong/Snapzy/blob/628a9f777830abee1c8a7391855fc7f4b22f5e61/SnapzyTests/Services/Capture/ScreenCaptureAreaCropTests.swift#L18-L70)。

### P1：OCR 到翻译的状态机

8. **明确三个阶段和取消边界：** `capture` → `ocr` → `detect/translate`。OCR 成功后立即允许复制/编辑；自动翻译应是可取消的后续任务，不能让网络翻译失败回滚 OCR 结果。
9. **为每次 OCR/翻译请求加 generation/request ID。** 用户编辑 OCR 文本、切换语言或再次截图时，旧 Provider 的迟到结果必须丢弃；这对应 Easydict `resultGeneration` 的做法。当前翻译协调器已有并发 Provider，但应确保 UI 不把旧结果写回新会话。
10. **区分 Provider 失败与翻译文本。** 当前 [`src-tauri/src/application/providers/translation/coordinator.rs`](../../src-tauri/src/application/providers/translation/coordinator.rs) 在单个 Provider 出错时构造 `translated_text = "Translation failed: ..."` 的结果。建议保留结构化 `error`/状态，让 UI 将失败卡片标为失败并允许重试，而不是把错误句子当正常译文复制或收藏。
11. **补齐自动语言策略。** OCR 返回的语言只是候选；短文本或混合文本应允许再次检测，必要时按检测语言重跑 OCR（Easydict 的 `deepOCR` 可作为未接线设计参考）。源语言保持 `auto` 时，翻译请求应记录检测结果和置信度，用户可手动覆盖并重试。
12. **支持“直接 OCR 翻译结果”能力。** 某些 OCR 服务会返回逐行翻译；Provider 接口可选返回该结果，目标语言匹配时直接使用，否则回落到普通翻译，避免重复网络调用。
13. **本地/远端边界可见。** 内置 Vision/Tesseract 等本地 OCR 应默认不上传；自定义远端 OCR/翻译在配置和执行反馈中显示目标域名、发送的图像/文本范围和超时/认证错误。API key 继续放凭据存储，不写入历史或导出文件。

### P2：捕获后体验

14. **把复制、结果窗口、历史、通知做成独立后处理。** OCR 复制全文应尽早完成；通知只显示截断预览，OCR/翻译窗口可继续编辑和重新翻译。QR/链接检测可并行，不应阻塞首个可用文本。
15. **补充可选的 OCR 专用入口。** 参考 Easydict 的静默 OCR、OCR 窗口和截图翻译三个 action，但沿用 SnapLingo 的统一 Capture Session，避免为每个入口复制抓屏实现。

## 最小落地顺序

1. 先确认 Capture Session 在单屏、跨屏、DPI、取消和 overlay 排除上的测试，并定义 frozen/live mouse-up 语义。
2. 将 `OcrResult` 扩展为可携带行信息/检测语言/引擎信息，同时加入 sRGB、profile、低质量 recovery 和规范化单测。
3. 将 OCR→翻译拆成可取消、带 generation 的阶段；为每个 Provider 返回结构化成功/失败状态，支持编辑后重译和语言覆盖。
4. 最后加入 QR/链接提示、原生通知、OCR 专用快捷键和 benchmark 仪表盘。

已有的截图架构取舍可继续参考 [`docs/research/screenshot-implementation-comparison.md`](screenshot-implementation-comparison.md)；本文补充的是该文未覆盖的翻译编排、OCR 文本质量和当前 SnapLingo 数据模型差距。
