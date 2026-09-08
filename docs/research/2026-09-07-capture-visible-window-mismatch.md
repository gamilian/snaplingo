# macOS 截图与可见顶层窗口不一致：Easydict / Snapzy 源码核查

调研日期：2026-09-07。只核查源码和上游 issue / PR，未运行两款上游应用复现。源码来自当日 shallow clone：Easydict 默认 `dev` 分支 [`e812288ff0a6a39fcadd367b37b49f425fcd7779`](https://github.com/tisfeng/Easydict/tree/e812288ff0a6a39fcadd367b37b49f425fcd7779)，Snapzy `master` 分支 [`e204713b40907dba3c2a3432fda5d894d6340b0c`](https://github.com/duongductrong/Snapzy/tree/e204713b40907dba3c2a3432fda5d894d6340b0c)。这两者均晚于本仓库既有对比笔记引用的提交。2026-09-08 的回归测试确认，选区完成前重新取图仍会丢失已经关闭的前台窗口；SnapLingo 因此改为在显示截图窗口之前冻结像素，并在选区底图和输出中复用同一帧。详见[截图流程核查](2026-09-08-capture-pipeline-audit.md)。

## 可直接回答用户的结论

- **Snapzy 确实遇到过类似的“最上层临时窗口消失”问题，并做了针对性修复。** 其 [issue #422](https://github.com/duongductrong/Snapzy/issues/422) 报告 OCR 快捷键导致菜单栏下拉窗口消失；维护者在[关闭说明](https://github.com/duongductrong/Snapzy/issues/422#issuecomment-5218645315)中指向修复提交 [`b7385ac`](https://github.com/duongductrong/Snapzy/commit/b7385ac10ea05d9616af88536b2a8e9f4c487733)。这是相近场景的已报告问题，不能据此认定 SnapLingo 当前故障也是菜单栏窗口或具有相同根因。
- **Snapzy 的关键措施是保留正确时刻的像素、减少选区 UI 对前台状态的干扰。** 有冻结底图模式；默认 live 模式在 mouse-up 同步锁帧，再撤销 overlay；临时菜单栏 popover 另在显示 overlay 前保留像素。它不是只靠换成 ScreenCaptureKit 解决问题。具体源码见下文。
- **Easydict 已显示冻结截图底图。** 虽然结束时再次调用截屏 API，但其底图窗口仍然显示，因此不能把“两次调用 API”直接解释为“第二次一定截到了变化后的桌面”。它没有直接从同一张缓存原图裁剪的显式保证，仍需运行验证，不能断言它一定有或一定没有用户描述的问题。见下文生命周期分析。
- **`CGDisplayCreateImage` 的弃用不构成本次故障的根因证据。** 当前 Easydict 使用它，Snapzy 的冻结快速路径、live mouse-up 锁帧和提前保存 popover 也使用它。是否遗漏窗口还与取图时点、窗口激活、capture exclusion 以及窗口种类有关；不能仅凭 API 名称决定因果。

## Easydict：有冻结底图，最后通过屏幕再次取图

源码流程：

1. 创建每屏全屏 borderless window，设为 `.screenSaver` 层级，执行 `makeKeyAndOrderFront`。
2. 初始化 `ScreenshotOverlayView`，此时 `screen.takeScreenshot()` 抓取背景图片；View 以 `Image(nsImage:)` 显示它。
3. 所有窗口创建后显式激活 Easydict，以接收键盘事件。
4. 选区完成时重置选区状态，100 ms 后再次 `screen.takeScreenshot(rect:)`；取图完成之后 `finishCapture` 才隐藏 overlay。

来源：[窗口创建顺序](https://github.com/tisfeng/Easydict/blob/e812288ff0a6a39fcadd367b37b49f425fcd7779/Easydict/Swift/Feature/Screenshot/Screenshot/Screenshot.swift#L148-L206)、[底图初始化与显示](https://github.com/tisfeng/Easydict/blob/e812288ff0a6a39fcadd367b37b49f425fcd7779/Easydict/Swift/Feature/Screenshot/Screenshot/ScreenshotOverlayView.swift#L16-L63)、[结束阶段取图和关闭顺序](https://github.com/tisfeng/Easydict/blob/e812288ff0a6a39fcadd367b37b49f425fcd7779/Easydict/Swift/Feature/Screenshot/Screenshot/Screenshot.swift#L64-L117)。

这里需要纠正既有对比笔记的一处推断：`reset()` 清空选区并隐藏暗色蒙层，但没有删除 `backgroundImage`；取图前也没有 order-out 底图窗口。截图窗口源码没有设置 `sharingType = .none`。因此第二次屏幕取图**可能仍取得覆盖在屏幕上的冻结底图**，不能简单推导成“取到了另一时刻的真实桌面”。是否完整取得该 overlay、是否有 SwiftUI 绘制竞争，需要运行验证。来源：[reset 实现](https://github.com/tisfeng/Easydict/blob/e812288ff0a6a39fcadd367b37b49f425fcd7779/Easydict/Swift/Feature/Screenshot/Screenshot/ScreenshotState.swift#L56-L64)、[overlay 实现](https://github.com/tisfeng/Easydict/blob/e812288ff0a6a39fcadd367b37b49f425fcd7779/Easydict/Swift/Feature/Screenshot/Screenshot/Screenshot.swift#L183-L227)。

底层 `takeScreenshot` 调用 `CGDisplayCreateImage(displayID)` 抓整屏，再依据 `backingScaleFactor` 裁剪。[源码](https://github.com/tisfeng/Easydict/blob/e812288ff0a6a39fcadd367b37b49f425fcd7779/Easydict/Swift/Feature/Screenshot/Screenshot/NSScreen%2BExtention.swift#L15-L48)

**风险推断而非已复现 bug：** 初始背景是在 `makeKeyAndOrderFront` 之后获取的，后续还会激活应用，因此初次快照之前可能已发生焦点相关变化；与 Snapzy 的 pre-overlay 保留方案相比，没有明确保证任何临时窗口都能保存下来。上游 issue 检索暂未确认与用户所述现象完全一致的 Easydict 报告；这不代表它不存在问题。

## Snapzy：冻结模式和 live 模式分别保证取图时序

### 冻结模式：预览与裁剪引用同一份快照

`freezesAreaCapture` 默认值是 `false`，即普通区域截图默认 live；用户可开启冻结模式。[设置读取](https://github.com/duongductrong/Snapzy/blob/e204713b40907dba3c2a3432fda5d894d6340b0c/Snapzy/Features/Capture/CaptureViewModel.swift#L137-L146)

冻结模式先获取 snapshot，构建 `FrozenAreaCaptureSession`，然后以 `frozenSession.backdrops` 启动选区。手工矩形选择完成且快照可用时，读取 `frozenSession.allSnapshots()`，以 `cropImage` 或 `cropCompositeImage` 裁剪。backdrop 的 `image` 和 crop 输入来自同一个 `snapshot.image`，因此比“显示实时桌面，结果来自早先图片”有更强的可见内容一致性；这个结论不涵盖独立窗口目标的重新取图分支。[准备快照](https://github.com/duongductrong/Snapzy/blob/e204713b40907dba3c2a3432fda5d894d6340b0c/Snapzy/Features/Capture/CaptureViewModel.swift#L697-L805)、[传入底图](https://github.com/duongductrong/Snapzy/blob/e204713b40907dba3c2a3432fda5d894d6340b0c/Snapzy/Features/Capture/CaptureViewModel.swift#L1025-L1055)、[结果裁剪](https://github.com/duongductrong/Snapzy/blob/e204713b40907dba3c2a3432fda5d894d6340b0c/Snapzy/Features/Capture/CaptureViewModel.swift#L1116-L1148)、[快照及底图模型](https://github.com/duongductrong/Snapzy/blob/e204713b40907dba3c2a3432fda5d894d6340b0c/Snapzy/Services/Capture/FrozenAreaCaptureSession.swift#L81-L118)。

这不是所有显示器在同一硬件时刻抓帧的承诺：其他屏可延迟准备；缺失快照或 Space 转换还有补抓与更新路径。冻结模式明确同时更新可见底图与裁剪真源，而不是只更新一方。[转换后更新](https://github.com/duongductrong/Snapzy/blob/e204713b40907dba3c2a3432fda5d894d6340b0c/Snapzy/Features/Capture/CaptureViewModel.swift#L1854-L1937)

### 取图 API：Core Graphics 是常用快速路径，不是完全淘汰的旧实现

没有显示光标、桌面图标/小组件排除需求时，冻结路径优先 `captureFastDisplaySnapshotOffMain`，实际调用 `CGDisplayCreateImage`。快速路径失败或不适用才走 ScreenCaptureKit；后者在 macOS 14+ 使用 `SCScreenshotManager.captureImage`，macOS 13 使用单帧 `SCStream`。[快速路径](https://github.com/duongductrong/Snapzy/blob/e204713b40907dba3c2a3432fda5d894d6340b0c/Snapzy/Services/Capture/ScreenCaptureManager.swift#L250-L309)、[路径选择](https://github.com/duongductrong/Snapzy/blob/e204713b40907dba3c2a3432fda5d894d6340b0c/Snapzy/Features/Capture/CaptureViewModel.swift#L703-L762)、[SCK 兼容包装](https://github.com/duongductrong/Snapzy/blob/e204713b40907dba3c2a3432fda5d894d6340b0c/Snapzy/Services/Capture/ScreenCaptureManager.swift#L2431-L2449)。

其 SCK 显示器过滤器可按设置排除自身应用、Finder 桌面图标或 widgets；无排除时使用 `SCContentFilter(display: display, excludingWindows: [])`。因此也不能把“使用 SCK”理解成“始终取屏幕所有可见对象”。[过滤器](https://github.com/duongductrong/Snapzy/blob/e204713b40907dba3c2a3432fda5d894d6340b0c/Snapzy/Services/Capture/ScreenCaptureManager.swift#L2452-L2483)

### live 模式：先 mouse-up 锁帧，再撤销选区窗口

live 区域完成回调会同步获取选区涉及的显示器截图，不经过 `Task` / `await`，取得像素后才 `cancelSelection()`，随后异步裁剪/保存。这样减少松手后关闭 overlay、切换应用或临时窗口消失造成的画面变化。[完成流程](https://github.com/duongductrong/Snapzy/blob/e204713b40907dba3c2a3432fda5d894d6340b0c/Snapzy/Features/Capture/CaptureViewModel.swift#L1273-L1357)

这条锁帧路径也调用 `CGDisplayCreateImage`。显示光标、排除桌面图标/widgets、任一屏抓帧失败时返回空集合，随后回退到异步 `captureArea()`。**该 fallback 没有同样的 mouse-up 像素保证**，不能宣称 Snapzy 所有选项下都严格所见即所得。源码顺序只能证明消除了此处的异步等待，不代表显示器截图没有耗时或多屏帧原子性。[锁帧条件](https://github.com/duongductrong/Snapzy/blob/e204713b40907dba3c2a3432fda5d894d6340b0c/Snapzy/Features/Capture/CaptureViewModel.swift#L1385-L1463)、[fallback](https://github.com/duongductrong/Snapzy/blob/e204713b40907dba3c2a3432fda5d894d6340b0c/Snapzy/Features/Capture/CaptureViewModel.swift#L1483-L1502)

### 减少焦点和鼠标事件对前台窗口的影响

选区窗口为 `.nonactivatingPanel`，`sharingType = .none`，常规选区避免调用 `NSApp.activate`。live passthrough 使用 event tap 驱动选区，消费 mouse move / click / scroll，避免底层 app 收到 mouse-exit 等事件而收起 tooltip / hover card；缺少 Accessibility 权限时可退回普通 panel 输入。[panel](https://github.com/duongductrong/Snapzy/blob/e204713b40907dba3c2a3432fda5d894d6340b0c/Snapzy/Services/Capture/AreaSelectionWindow.swift#L2367-L2405)、[启动顺序](https://github.com/duongductrong/Snapzy/blob/e204713b40907dba3c2a3432fda5d894d6340b0c/Snapzy/Services/Capture/AreaSelectionWindow.swift#L658-L680)、[event tap 消费事件](https://github.com/duongductrong/Snapzy/blob/e204713b40907dba3c2a3432fda5d894d6340b0c/Snapzy/Services/Capture/CaptureEventTapController.swift#L259-L310)、[权限判断](https://github.com/duongductrong/Snapzy/blob/e204713b40907dba3c2a3432fda5d894d6340b0c/Snapzy/Services/Capture/CaptureEventTapController.swift#L145-L151)。

源码也明确保留了限制：选区窗口不可见的最后恢复手段会激活 Snapzy；注释承认这可能改变前台外观、关闭临时 popover。[最后恢复路径](https://github.com/duongductrong/Snapzy/blob/e204713b40907dba3c2a3432fda5d894d6340b0c/Snapzy/Services/Capture/AreaSelectionWindow.swift#L910-L918)

## Snapzy 已确认的类似问题：菜单栏 popover

[PR #430](https://github.com/duongductrong/Snapzy/pull/430)（2026-07-29 合并，merge commit [`ea9fc3e`](https://github.com/duongductrong/Snapzy/commit/ea9fc3e422330cbc27caa7c162ab5de60b71c9e9)）针对第三方菜单栏 popover：某些窗口在显示截图 panel 时关闭，不能等选区完成才查询/取图。当前实现：

1. 创建选区窗口之前枚举符合菜单栏 popover 规则的窗口，用 `CGDisplayCreateImage` 抓其所在屏，再裁出其当时可见像素并缓存。源码注释说明此类窗口可被枚举，但未必能独立渲染。[提前缓存](https://github.com/duongductrong/Snapzy/blob/e204713b40907dba3c2a3432fda5d894d6340b0c/Snapzy/Services/Capture/WindowSelectionQueryService.swift#L173-L257)
2. 若窗口已经消失，用保留图恢复 overlay 上的视觉内容；窗口仍在时不重复显示保留图。[保留图层](https://github.com/duongductrong/Snapzy/blob/e204713b40907dba3c2a3432fda5d894d6340b0c/Snapzy/Services/Capture/AreaSelectionWindow.swift#L4071-L4119)
3. 对窗口截图，优先可用的 SCK 窗口；窗口不再可用且有保留 popover 图时使用它；再考虑 CG 窗口取图或区域 fallback。该顺序属于窗口目标路径，不等于任何手工矩形都会自动合成保留图。[决策](https://github.com/duongductrong/Snapzy/blob/e204713b40907dba3c2a3432fda5d894d6340b0c/Snapzy/Services/Capture/WindowCaptureSelectionPolicy.swift#L137-L158)
4. OCR / object cutout 后来通过 [`b7385ac`](https://github.com/duongductrong/Snapzy/commit/b7385ac10ea05d9616af88536b2a8e9f4c487733) 接入保留 popover 与 mouse-up 锁帧。当前 `captureLiveSelectionImage` 对匹配的保留窗口目标直接返回其像素，否则裁剪 mouse-up snapshots，最后才回退新抓 SCK 区域。[当前 OCR 图像来源](https://github.com/duongductrong/Snapzy/blob/e204713b40907dba3c2a3432fda5d894d6340b0c/Snapzy/Features/Capture/CaptureViewModel.swift#L1581-L1651)

**仍存在源码可见的边界：** 第三方菜单栏 popover 的保留策略不涵盖所有 Quick Look、应用内弹层或系统 UI。窗口路径若成功找到 SCK 窗口但实际取图抛错，会直接走 fresh area fallback，而不会再尝试保留图，因此还有需要实测的竞态风险。这是代码路径分析，不是本次新复现的 bug。[异常和 fallback](https://github.com/duongductrong/Snapzy/blob/e204713b40907dba3c2a3432fda5d894d6340b0c/Snapzy/Services/Capture/ScreenCaptureManager.swift#L1081-L1171)

## 对 SnapLingo 的可借鉴原则

基于以上源码推导，优先借鉴的是：**在改变前台状态之前保存显示器合成像素，以同一张图提供选区底图、放大镜、预览和输出；普通区域截图避免变成独立窗口截图，避免排除整个自身应用造成可见贴图被删掉。** 如果产品需要 live 画面，则参考 Snapzy 的 mouse-up 同步锁帧和先取像素再收起 overlay 的顺序，并明确异步 fallback 的语义。临时菜单/popover 的专门保留策略应由实际复现决定，不必提前引入整套复杂窗口识别。

这些是改进原则，尚不能替代 SnapLingo 的根因验证。仍应分别验证：初始整屏像素是否已缺失顶层窗口；显示选区窗口是否使焦点/层级改变；最后是否从另一来源取图。不能仅因 `CGDisplayCreateImage` 已弃用而跳过这三项检查。
